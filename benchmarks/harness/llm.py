"""A small OpenAI-compatible chat client with an on-disk cache.

Every call is cached by (model, messages, parameters), so an interrupted run
resumes where it stopped and a finished run can be re-scored for free. Token
usage and latency are recorded with each answer and survive in the cache.
"""

import hashlib
import json
import os
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional


@dataclass
class Completion:
    text: str
    prompt_tokens: int
    completion_tokens: int
    seconds: float
    cached: bool


class ChatClient:
    def __init__(
        self,
        api_url: str,
        model: str,
        cache_path: Path,
        api_key: Optional[str] = None,
        extra: Optional[dict] = None,
    ):
        # Provider-specific request fields, e.g. {"thinking": {"type": "disabled"}}.
        self.extra = dict(extra or {})
        self.api_url = api_url.rstrip("/")
        self.model = model
        # Where the key is injected by a proxy, any placeholder works.
        self.api_key = api_key or os.environ.get("MEMST_EVAL_API_KEY", "proxy-injected")
        self.cache_path = Path(cache_path)
        self.cache_path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = threading.Lock()
        self._cache: Dict[str, dict] = {}
        if self.cache_path.exists():
            for line in self.cache_path.read_text().splitlines():
                if line.strip():
                    row = json.loads(line)
                    self._cache[row["key"]] = row

    def complete(self, messages: List[dict], max_tokens: int = 200, temperature: float = 0.0) -> Completion:
        payload = {"model": self.model, "messages": messages, "max_tokens": max_tokens, "temperature": temperature}
        payload.update(self.extra)
        key = hashlib.sha256(json.dumps(payload, sort_keys=True).encode()).hexdigest()
        with self._lock:
            hit = self._cache.get(key)
        if hit:
            return Completion(hit["text"], hit["prompt_tokens"], hit["completion_tokens"], hit["seconds"], True)

        request = urllib.request.Request(
            f"{self.api_url}/chat/completions",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {self.api_key}"},
        )
        last_error = None
        for attempt in range(5):
            started = time.time()
            try:
                with urllib.request.urlopen(request, timeout=300) as response:
                    body = json.loads(response.read())
                seconds = time.time() - started
                usage = body.get("usage", {})
                choice = body["choices"][0]
                if choice.get("finish_reason") == "length" and not (choice["message"].get("content") or "").strip():
                    # A reasoning model spent the whole budget thinking. An
                    # empty answer must not be cached and scored as wrong.
                    raise RuntimeError(
                        f"the model produced no answer within max_tokens={max_tokens}; raise it or turn reasoning off"
                    )
                row = {
                    "key": key,
                    "text": (body["choices"][0]["message"].get("content") or "").strip(),
                    "prompt_tokens": usage.get("prompt_tokens", 0),
                    "completion_tokens": usage.get("completion_tokens", 0),
                    "seconds": seconds,
                }
                with self._lock:
                    self._cache[key] = row
                    with self.cache_path.open("a") as handle:
                        handle.write(json.dumps(row) + "\n")
                return Completion(row["text"], row["prompt_tokens"], row["completion_tokens"], seconds, False)
            except RuntimeError:
                raise
            except urllib.error.HTTPError as error:
                last_error = f"HTTP {error.code}: {error.read()[:300]!r}"
                if error.code in (400, 401, 403, 404):
                    break
            except Exception as error:  # timeouts, connection resets
                last_error = repr(error)
            time.sleep(2**attempt)
        raise RuntimeError(f"LLM call failed: {last_error}")
