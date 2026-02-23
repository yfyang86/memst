#!/usr/bin/env python3
"""DuckDuckGo web search → clean Markdown. Zero external dependencies.

Usage:
    python ddg_search.py "search query"
    python ddg_search.py "search query" -n 10
    python ddg_search.py "search query" --json
"""

import argparse
import json
import sys
import re
from html.parser import HTMLParser
from urllib.request import Request, urlopen
from urllib.parse import urlencode, unquote


# ---------------------------------------------------------------------------
# HTML parser for DuckDuckGo HTML-lite results
# ---------------------------------------------------------------------------

class DDGResultParser(HTMLParser):
    """Parse DuckDuckGo HTML search results page."""

    def __init__(self):
        super().__init__()
        self.results = []
        self._current = {}
        self._capture = None  # "title" | "snippet" | None
        self._text_buf = []
        self._in_result_link = False
        self._in_snippet = False

    def handle_starttag(self, tag, attrs):
        attrs_dict = dict(attrs)
        cls = attrs_dict.get("class", "")

        # Result title link
        if tag == "a" and "result__a" in cls:
            href = attrs_dict.get("href", "")
            # DuckDuckGo wraps URLs in a redirect; extract the real URL
            real_url = self._extract_url(href)
            self._current = {"title": "", "url": real_url, "snippet": ""}
            self._capture = "title"
            self._text_buf = []

        # Result snippet
        if tag == "a" and "result__snippet" in cls:
            self._capture = "snippet"
            self._text_buf = []

    def handle_endtag(self, tag):
        if tag == "a" and self._capture == "title":
            self._current["title"] = self._flush_buf()
            self._capture = None
        elif tag == "a" and self._capture == "snippet":
            self._current["snippet"] = self._flush_buf()
            self._capture = None
            # snippet is the last piece per result — commit it
            if self._current.get("title") and self._current.get("url"):
                self.results.append(self._current)
            self._current = {}

    def handle_data(self, data):
        if self._capture:
            self._text_buf.append(data)

    # -- helpers --

    def _flush_buf(self):
        text = " ".join("".join(self._text_buf).split())
        self._text_buf = []
        return text

    @staticmethod
    def _extract_url(href):
        """Extract real URL from DuckDuckGo redirect wrapper."""
        # DDG uses //duckduckgo.com/l/?uddg=<encoded_url>&...
        if "uddg=" in href:
            match = re.search(r"uddg=([^&]+)", href)
            if match:
                return unquote(match.group(1))
        # Sometimes links are already direct
        if href.startswith("http"):
            return href
        return href


# ---------------------------------------------------------------------------
# Search function
# ---------------------------------------------------------------------------

HEADERS = {
    "User-Agent": "Mozilla/5.0 (compatible; DDGSearch-CLI/1.0)",
    "Accept": "text/html",
    "Accept-Language": "en-US,en;q=0.9",
}


def search(query: str, max_results: int = 5) -> list[dict]:
    """Search DuckDuckGo HTML-lite and return parsed results."""
    url = "https://html.duckduckgo.com/html/?" + urlencode({"q": query})
    req = Request(url, headers=HEADERS)

    with urlopen(req, timeout=15) as resp:
        html = resp.read().decode("utf-8", errors="replace")

    parser = DDGResultParser()
    parser.feed(html)
    return parser.results[:max_results]


# ---------------------------------------------------------------------------
# Formatters
# ---------------------------------------------------------------------------

def to_markdown(results: list[dict], query: str) -> str:
    if not results:
        return f"# Search: {query}\n\nNo results found."

    lines = [f"# Search: {query}", f"*{len(results)} results*\n"]
    for i, r in enumerate(results, 1):
        lines.append(f"## {i}. [{r['title']}]({r['url']})\n")
        if r.get("snippet"):
            lines.append(f"{r['snippet']}\n")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description="Search DuckDuckGo → Markdown")
    ap.add_argument("query", help="Search query")
    ap.add_argument("-n", "--max", type=int, default=5, dest="max_results",
                    help="Max results (default: 5)")
    ap.add_argument("--json", action="store_true", dest="as_json",
                    help="Output raw JSON instead of Markdown")
    args = ap.parse_args()

    try:
        results = search(args.query, args.max_results)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    if args.as_json:
        print(json.dumps(results, indent=2, ensure_ascii=False))
    else:
        print(to_markdown(results, args.query))


if __name__ == "__main__":
    main()
