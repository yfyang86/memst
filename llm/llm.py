#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
llm.py

* reads llm configuration from ~/.config/memst/config.toml
* offers two convenience wrappers:
      • completions_one_shot()   – stream=False (returns the whole JSON)
      • completions_stream()    – stream=True  (prints partial tokens)
* usage:
      python llm.py            # default: stream=True
      python llm.py --nosync   # stream=False
"""

import argparse
import json
import pathlib
import sys
import tomllib
from typing import Any, Dict, List, Optional

import requests

# ----------------------------------------------------------------------
# 1️⃣ Load configuration
# ----------------------------------------------------------------------
CONFIG_PATH = pathlib.Path("~/.config/memst/config.toml")


def load_config(path: pathlib.Path) -> Dict[str, Any]:
    try:
        with path.open("rb") as fp:
            return tomllib.load(fp)
    except FileNotFoundError:
        sys.exit(f"❌ Config file not found: {path}")
    except tomllib.TOMLDecodeError as exc:
        sys.exit(f"❌ Invalid TOML in {path}: {exc}")


_cfg = load_config(CONFIG_PATH)

try:
    _llm = _cfg["llm"]
    BASE_URL = _llm["base_url"].rstrip("/")          # example: `localhost:7999/v1`
    API_KEY = _llm["api_key"]
    MODEL = _llm["model"]
except KeyError as exc:
    missing = exc.args[0]
    sys.exit(f"❌ Missing required key in the [llm] section: {missing}")

# ----------------------------------------------------------------------
# 2️⃣ Helper: common request building
# ----------------------------------------------------------------------
def _build_headers() -> Dict[str, str]:
    return {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json",
        # Tell the server we are happy to receive Server‑Sent Events.
        # It is ignored for non‑streaming calls, so we can always set it.
        "Accept": "text/event-stream",
    }


def _make_payload(
    prompt: str,
    temperature: float = 0.0,
    max_tokens: int = 7,
    stream: bool = False,
) -> Dict[str, Any]:
    return {
        "model": MODEL,
        "prompt": prompt,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "stream": stream,
    }


# ----------------------------------------------------------------------
# 3️⃣ Non‑streaming version (stream=False)
# ----------------------------------------------------------------------
def completions_one_shot(
    prompt: str,
    temperature: float = 0.0,
    max_tokens: int = 7,
) -> Dict[str, Any]:
    """
    Sends a single request (`stream=False`) and returns the **full** JSON
    response as a Python dict.
    """
    url = f"{BASE_URL}/completions"
    headers = _build_headers()
    payload = _make_payload(prompt, temperature, max_tokens, stream=False)

    try:
        resp = requests.post(url, headers=headers, json=payload, timeout=60)
        resp.raise_for_status()
    except requests.RequestException as exc:
        sys.stderr.write(f"\n❌ Request failed: {exc}\n")
        if getattr(exc, "response", None) is not None:
            try:
                err = exc.response.json()
                json.dump(err, sys.stderr, indent=2)
                sys.stderr.write("\n")
            except Exception:
                sys.stderr.write(exc.response.text + "\n")
        sys.exit(1)

    try:
        return resp.json()
    except json.JSONDecodeError:
        sys.exit("❌ The server did not return valid JSON.")


# ----------------------------------------------------------------------
# 4️⃣ Streaming version (stream=True)
# ----------------------------------------------------------------------
def completions_stream(
    prompt: str,
    temperature: float = 0.0,
    max_tokens: int = 7,
) -> None:
    """
    Sends a streaming request (`stream=True`).  Each incoming token is printed
    immediately, mimicking the behaviour of `curl … -d … -N`.
    """
    url = f"{BASE_URL}/completions"
    headers = _build_headers()
    payload = _make_payload(prompt, temperature, max_tokens, stream=True)

    try:
        resp = requests.post(
            url,
            headers=headers,
            json=payload,
            stream=True,          # keep the HTTP connection open
            timeout=60,
        )
        resp.raise_for_status()
    except requests.RequestException as exc:
        sys.stderr.write(f"\n❌ Request failed: {exc}\n")
        if getattr(exc, "response", None) is not None:
            try:
                err = exc.response.json()
                json.dump(err, sys.stderr, indent=2)
                sys.stderr.write("\n")
            except Exception:
                sys.stderr.write(exc.response.text + "\n")
        sys.exit(1)

    print("\n🖋️  Streaming response (press Ctrl‑C to abort):\n")
    try:
        for raw_line in resp.iter_lines(decode_unicode=True):
            if not raw_line:      # ignore keep‑alive empty lines
                continue

            # Each line looks like:  data: {...}
            if not raw_line.startswith("data:"):
                continue

            json_part = raw_line[5:].strip()   # strip the leading "data:"
            if json_part == "[DONE]":
                print("\n🚧 Stream finished.")
                break

            try:
                chunk = json.loads(json_part)
            except json.JSONDecodeError:
                print(f"⚠️  Could not parse JSON chunk: {json_part}")
                continue

            # ==== Extract the token text ====
            delta: Optional[str] = None

            # OpenAI “chat” style payload (delta → content)
            if "choices" in chunk and isinstance(chunk["choices"], list):
                choice = chunk["choices"][0]
                if isinstance(choice, dict):
                    # Chat style
                    if "delta" in choice and isinstance(choice["delta"], dict):
                        delta = choice["delta"].get("content")
                    # Classic completion style
                    elif "text" in choice:
                        delta = choice["text"]

            if delta:
                # Print without a newline so it looks like a live stream
                print(delta, end="", flush=True)
    except KeyboardInterrupt:
        print("\n✋  Interrupted by user.")
    finally:
        resp.close()


# ----------------------------------------------------------------------
# 5️⃣ CLI entry‑point – choose stream / non‑stream with a flag
# ----------------------------------------------------------------------
def _cli() -> None:
    parser = argparse.ArgumentParser(
        description="Demo wrapper for the martingale completions endpoint."
    )
    parser.add_argument(
        "prompt",
        nargs="?",
        default="Say this is a test",
        help="Prompt to send to the model (default: %(default)s)",
    )
    parser.add_argument(
        "--nosync",
        dest="stream",
        action="store_false",
        help="Do a non‑streaming request (returns full JSON).",
    )
    parser.add_argument(
        "--temperature",
        type=float,
        default=0.0,
        help="Sampling temperature (default: %(default)s).",
    )
    parser.add_argument(
        "--max-tokens",
        type=int,
        default=7,
        help="Maximum number of tokens to generate (default: %(default)s).",
    )
    args = parser.parse_args()

    if args.stream:
        completions_stream(
            prompt=args.prompt,
            temperature=args.temperature,
            max_tokens=args.max_tokens,
        )
    else:
        result = completions_one_shot(
            prompt=args.prompt,
            temperature=args.temperature,
            max_tokens=args.max_tokens,
        )
        print("\n🔎 Full response payload:")
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # Show the actual text (if the server follows OpenAI schema)
        try:
            text = result["choices"][0].get("text") or result["choices"][0]["message"]["content"]
            print("\n💬 Model reply:")
            print(text.strip())
        except Exception:
            pass


if __name__ == "__main__":
    _cli()
