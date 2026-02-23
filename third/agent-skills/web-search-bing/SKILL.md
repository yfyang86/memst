---
name: web-search-bing
description: Search the web via Bing and return clean Markdown or JSON results. No API key required. Use when the agent needs to look something up on the web, find current information, research a topic, or answer questions that require up-to-date knowledge. Triggers on any web search, internet lookup, or "search for" request.
metadata: { "openclaw": { "emoji": "🔍", "requires": { "bins": ["python3"] } } }
---

# Web Search (Bing)

Scrapes Bing search results and returns clean Markdown. No API keys needed. Uses only Python stdlib (`urllib`, `re`, `html`, `json`).

## Quick usage

```bash
python3 scripts/bing-search.py "your search query"
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `-n, --count N` | Number of results | 10 |
| `-f, --format` | `markdown` or `json` | markdown |

## Examples

Basic search:

```bash
python3 scripts/bing-search.py "latest AI news"
```

Fewer results, JSON output:

```bash
python3 scripts/bing-search.py -n 3 -f json "rust programming language"
```

Multi-word queries work naturally:

```bash
python3 scripts/bing-search.py how to make sourdough bread
```

## Output formats

**Markdown** (default) — one `## N. [Title](url)` block per result with snippet text beneath.

**JSON** — structured `{"query": "...", "results": [{"title", "url", "snippet"}, ...]}` for programmatic use.

## Tips

- Keep queries short and specific for best results (1–6 words).
- Use quotes inside the query string for exact phrases: `python3 scripts/bing-search.py '"exact phrase" topic'`.
- Pipe to other tools: `python3 scripts/bing-search.py -f json "query" | jq '.results[0].url'`.
- If Bing changes its HTML structure and parsing breaks, the fallback extractor will attempt broader `<h2><a>` + `<p>` matching.
