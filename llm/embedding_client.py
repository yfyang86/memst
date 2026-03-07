#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
embedding_client.py

A tiny wrapper around an OpenAI‑compatible *embeddings* endpoint.
It reads its configuration from ~/.config/memst/config.toml, validates the
input, performs a POST request with ``requests`` and returns the numeric
vectors as plain Python lists.

Usage (as a script):
    python embedding_client.py "Some text to embed"
    python embedding_client.py -f sentences.txt   # one sentence per line

Import in other code:
    from embedding_client import EmbeddingClient
    client = EmbeddingClient()
    vecs = client.embed(["first", "second"])
"""

import argparse
import json
import pathlib
import sys
import tomllib
from typing import List, Sequence, Union

import requests

# ----------------------------------------------------------------------
# 3️⃣ Load the TOML configuration
# ----------------------------------------------------------------------
CONFIG_PATH = pathlib.Path("~/.config/memst/config.toml")


def _load_config() -> dict:
    try:
        with CONFIG_PATH.open("rb") as fp:
            return tomllib.load(fp)
    except FileNotFoundError:
        sys.exit(f"❌ Config file not found: {CONFIG_PATH}")
    except tomllib.TOMLDecodeError as exc:
        sys.exit(f"❌ Invalid TOML in {CONFIG_PATH}: {exc}")


_cfg = _load_config()

# Validate that the required `[embedding]` table exists
try:
    _emb_cfg = _cfg["embedding"]
    BASE_URL = _emb_cfg["base_url"].rstrip("/") + "/"   # guarantee trailing slash
    MODEL = _emb_cfg["model"]
    MAX_TOKENS = int(_emb_cfg.get("max_context_length", 0))   # 0 → no check
    EXPECTED_DIM = int(_emb_cfg.get("embedding_length", 0))   # 0 → no check
except KeyError as exc:
    missing = exc.args[0]
    sys.exit(f"❌ Missing required key in the [embedding] section: {missing}")


# ----------------------------------------------------------------------
# 4️⃣ Core client class
# ----------------------------------------------------------------------
class EmbeddingClient:
    """
    Minimal wrapper for ``/v1/embeddings``.
    """

    def __init__(self,
                 base_url: str = BASE_URL,
                 model: str = MODEL,
                 max_context_length: int = MAX_TOKENS,
                 embedding_length: int = EXPECTED_DIM,
                 timeout: int = 60):
        self.base_url = base_url.rstrip("/") + "/"
        self.endpoint = self.base_url + "embeddings"   # final URL
        self.model = model
        self.max_context_length = max_context_length
        self.embedding_length = embedding_length
        self.timeout = timeout
        self._session = requests.Session()   # keep‑alive & connection pooling

    # ------------------------------------------------------------------
    def _check_input(self, texts: Sequence[str]) -> None:
        """Raise an error if any string is longer than the allowed token budget."""
        if self.max_context_length <= 0:
            return          # no limit configured

        # Very rough token estimation – you can replace this with a proper tokenizer.
        # The simple heuristic works for most English‑ish strings.
        for i, txt in enumerate(texts):
            approx_tokens = len(txt.split())
            if approx_tokens > self.max_context_length:
                raise ValueError(
                    f"Input #{i} is {approx_tokens} tokens, exceeds the "
                    f"configured max_context_length={self.max_context_length}"
                )

    # ------------------------------------------------------------------
    def embed(self,
              texts: Union[str, Sequence[str]],
              *,
              model: str | None = None,
              temperature: float = 0.0) -> List[List[float]]:
        """
        Send one or many strings to the embedding service and return a list of
        numeric vectors (one per input).

        Parameters
        ----------
        texts : str | Sequence[str]
            The text to embed. A single string is automatically wrapped in a list.
        model : str | None
            Override the model name defined in the config (rarely needed).
        temperature : float
            Some services expose a temperature for embeddings – default 0.
            It is passed through unchanged.

        Returns
        -------
        List[List[float]]
            A list with the same length as ``texts``; each inner list is the
            embedding vector.
        """
        # Normalise to a list
        if isinstance(texts, str):
            texts = [texts]
        elif not isinstance(texts, (list, tuple)):
            raise TypeError("texts must be a string or a list/tuple of strings")

        self._check_input(texts)

        payload = {
            "model": model or self.model,
            "input": list(texts),        # ensure it's a plain list, not a tuple
        }
        # Some providers also accept a temperature field – we include it
        if temperature != 0.0:
            payload["temperature"] = temperature

        headers = {
            "Content-Type": "application/json",
            # No Authorization needed for the example service; add it here if required.
        }

        try:
            resp = self._session.post(
                self.endpoint,
                headers=headers,
                json=payload,
                timeout=self.timeout,
            )
            resp.raise_for_status()
        except requests.RequestException as exc:
            # Print a helpful JSON error payload if the server sent one.
            sys.stderr.write(f"\n❌ Request failed: {exc}\n")
            if getattr(exc, "response", None) is not None:
                try:
                    err = exc.response.json()
                    json.dump(err, sys.stderr, indent=2)
                    sys.stderr.write("\n")
                except Exception:
                    sys.stderr.write(exc.response.text + "\n")
            raise

        # --------------------------------------------------------------
        # Parse response – we expect the OpenAI‑compatible schema:
        #   {"data": [{"embedding": [...], "index": 0}, ...], "model": "..."}
        # --------------------------------------------------------------
        try:
            data = resp.json()
        except json.JSONDecodeError:
            raise RuntimeError("Server returned non‑JSON response")

        if "data" not in data:
            raise RuntimeError(f"Unexpected payload – no 'data' field: {data}")

        embeddings: List[List[float]] = []
        for item in data["data"]:
            vec = item.get("embedding")
            if not isinstance(vec, list):
                raise RuntimeError(f"Malformed embedding entry: {item}")
            if self.embedding_length and len(vec) != self.embedding_length:
                sys.stderr.write(
                    f"⚠️  Warning: received vector length {len(vec)} "
                    f"but config expects {self.embedding_length}\n"
                )
            embeddings.append(vec)

        # Preserve order – the API should already respect the order of `input`
        return embeddings

    # ------------------------------------------------------------------
    def close(self) -> None:
        """Close the underlying ``requests`` session."""
        self._session.close()


# ----------------------------------------------------------------------
# 5️⃣ Simple CLI for quick testing
# ----------------------------------------------------------------------
def _cli() -> None:
    parser = argparse.ArgumentParser(
        description="Compute embeddings using the config‑driven service."
    )
    group = parser.add_mutually_exclusive_group(required=False)
    group.add_argument(
        "-f",
        "--file",
        type=argparse.FileType("r"),
        help="Path to a text file – one line = one document to embed.",
    )
    parser.add_argument(
        "text",
        nargs="*",
        help="Text(s) to embed directly from the command line. "
             "Ignored if -f/--file is supplied.",
    )
    parser.add_argument(
        "--model",
        default=None,
        help="Override the model name from the config.",
    )
    args = parser.parse_args()

    if args.file:
        # Strip newline characters but keep empty lines as empty strings
        inputs = [line.rstrip("\n") for line in args.file]
    else:
        if not args.text:
            parser.error("Provide at least one text argument or use -f/--file.")
        inputs = args.text

    client = EmbeddingClient()
    try:
        vectors = client.embed(inputs, model=args.model)
    finally:
        client.close()

    # Pretty‑print the result – mirrors the curl output
    output = {
        "object": "list",
        "data": [
            {"object": "embedding", "embedding": vec, "index": i}
            for i, vec in enumerate(vectors)
        ],
        "model": client.model,
        "usage": {"prompt_tokens": 0, "total_tokens": 0},
    }
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    _cli()
