---
name: web-search-duckduckgo
description: Search the web using DuckDuckGo and return clean Markdown results. No API keys or external dependencies required (stdlib Python only). Use when you need to look something up online, find current information, research a topic, or verify facts. Supports web search, instant answers, and JSON output.
metadata: { "openclaw": { "emoji": "🔍", "requires": { "bins": ["python3", "curl"] } } }
---

# Web Search (DuckDuckGo)

Two methods, no API keys or pip installs needed.

## Web search via script (primary)

Full search results as Markdown — uses only Python stdlib:

```bash
python scripts/ddg_search.py "search query"
# Output: Markdown with titles, URLs, and snippets
```

More results:

```bash
./scripts/ddg_search.py "search query" -n 10
```

JSON output for programmatic use:

```bash
./scripts/ddg_search.py "search query" --json
```

## Instant answers via curl (quick facts)

DuckDuckGo Instant Answer API — great for definitions, quick facts:

```bash
curl -s "https://api.duckduckgo.com/?q=python+programming&format=json&no_html=1" | python3 -c "
import sys,json; d=json.load(sys.stdin)
for k in ('Answer','AbstractText','Definition'):
    if d.get(k): print(f'{k}: {d[k]}'); break
else:
    print('No instant answer. Use ddg_search.py for web results.')
"
```

Or just grab the raw JSON:

```bash
curl -s "https://api.duckduckgo.com/?q=query&format=json&no_html=1"
```

Key JSON fields: `AbstractText`, `Answer`, `Definition`, `RelatedTopics[]`

Docs: https://api.duckduckgo.com/api

## Tips

- Keep queries short (1–6 words) for best results.
- URL-encode spaces as `+` in curl commands.
- Use the script for full web results; use the curl API for quick lookups.
- If first search is insufficient, refine the query — don't repeat it.
