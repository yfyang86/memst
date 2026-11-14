#!/usr/bin/env python3
"""Scrape Bing search results and return clean Markdown. No API key needed."""

import argparse
import html
import json
import re
import sys
import urllib.parse
import urllib.request


BING_URL = "https://www.bing.com/search"
HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36"
    ),
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "en-US,en;q=0.9",
}


def fetch_html(query: str, count: int = 10) -> str:
    """Fetch raw HTML from Bing search results page."""
    params = urllib.parse.urlencode({"q": query, "count": str(count)})
    url = f"{BING_URL}?{params}"
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=15) as resp:
        return resp.read().decode("utf-8", errors="replace")


def strip_tags(text: str) -> str:
    """Remove HTML tags and decode entities."""
    text = re.sub(r"<[^>]+>", "", text)
    return html.unescape(text).strip()


def extract_results(raw_html: str) -> list[dict]:
    """Parse Bing HTML and extract organic results."""
    results = []

    # Primary pattern: <li class="b_algo"> blocks
    blocks = re.findall(
        r'<li\s+class="b_algo"[^>]*>(.*?)</li>', raw_html, re.DOTALL
    )

    for block in blocks:
        # Extract URL from <a href="...">
        url_m = re.search(r'<a\s[^>]*href="(https?://[^"]+)"', block)
        if not url_m:
            continue
        url = url_m.group(1)

        # Extract title text from the first <a>...</a>
        title_m = re.search(r"<a\s[^>]*>(.*?)</a>", block, re.DOTALL)
        title = strip_tags(title_m.group(1)) if title_m else url

        # Extract snippet from <p> or <div class="b_caption">
        snippet = ""
        snippet_m = re.search(
            r'<div\s+class="b_caption"[^>]*>(.*?)</div>', block, re.DOTALL
        )
        if snippet_m:
            # Try <p> inside caption
            p_m = re.search(r"<p[^>]*>(.*?)</p>", snippet_m.group(1), re.DOTALL)
            snippet = strip_tags(p_m.group(1)) if p_m else strip_tags(snippet_m.group(1))
        else:
            p_m = re.search(r"<p[^>]*>(.*?)</p>", block, re.DOTALL)
            if p_m:
                snippet = strip_tags(p_m.group(1))

        if not title and not snippet:
            continue

        results.append({"title": title, "url": url, "snippet": snippet})

    # Fallback: broader link+snippet extraction if b_algo parsing found nothing
    if not results:
        for m in re.finditer(
            r'<h2[^>]*>\s*<a\s[^>]*href="(https?://[^"]+)"[^>]*>(.*?)</a>\s*</h2>'
            r'.*?<p[^>]*>(.*?)</p>',
            raw_html,
            re.DOTALL,
        ):
            results.append({
                "title": strip_tags(m.group(2)),
                "url": m.group(1),
                "snippet": strip_tags(m.group(3)),
            })

    return results


def format_markdown(results: list[dict], query: str) -> str:
    """Render results as clean Markdown."""
    if not results:
        return f"# Bing: {query}\n\nNo results found.\n"

    lines = [f"# Bing: {query}\n"]
    for i, r in enumerate(results, 1):
        lines.append(f"## {i}. [{r['title']}]({r['url']})\n")
        if r["snippet"]:
            lines.append(f"{r['snippet']}\n")
    return "\n".join(lines)


def format_json(results: list[dict], query: str) -> str:
    """Render results as JSON."""
    return json.dumps({"query": query, "results": results}, indent=2)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Search Bing and return clean results (no API key)."
    )
    parser.add_argument("query", nargs="+", help="Search query terms")
    parser.add_argument(
        "-n", "--count", type=int, default=10, help="Number of results (default: 10)"
    )
    parser.add_argument(
        "-f",
        "--format",
        choices=["markdown", "json"],
        default="markdown",
        help="Output format (default: markdown)",
    )
    args = parser.parse_args()

    query = " ".join(args.query)
    try:
        raw = fetch_html(query, args.count)
    except Exception as e:
        print(f"Error fetching Bing results: {e}", file=sys.stderr)
        sys.exit(1)

    results = extract_results(raw)

    if args.format == "json":
        print(format_json(results, query))
    else:
        print(format_markdown(results, query))


if __name__ == "__main__":
    main()
