"""A plain MCP client: a model, the tools a server offers, and nothing else.

This is what MemSt looks like to a client that knows nothing about it. The
system prompt is one generic sentence and the server's own instructions.
The index of memory is not put in the prompt, and the session is not
recorded, unless the model calls the tools for that itself. Compare with
the Pi extension (`integrations/pi`), which does both.

The model is reached through an OpenAI-compatible chat endpoint
(`MEMST_LLM_API_URL`, key in `MEMST_LLM_API_KEY`).
"""

import json
import os
import subprocess
import time
import urllib.request

SYSTEM = "You are a helpful assistant. You are connected to a server that offers the tools below."


class Server:
    """`memst mcp serve` over standard input and output."""

    def __init__(self, memst: str, store: str, owner: str):
        self.process = subprocess.Popen([memst, "--store", store, "mcp", "--owner", owner, "serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        self.next_id = 0
        hello = self.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "plain-mcp-client", "version": "1"}})
        self.instructions = hello.get("instructions", "")
        self.notify("notifications/initialized")
        self.tools = self.request("tools/list", {})["tools"]

    def notify(self, method: str) -> None:
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method}) + "\n")
        self.process.stdin.flush()

    def request(self, method: str, params: dict) -> dict:
        self.next_id += 1
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}) + "\n")
        self.process.stdin.flush()
        while True:
            line = self.process.stdout.readline()
            if not line:
                raise RuntimeError("the server closed the connection")
            reply = json.loads(line)
            if reply.get("id") == self.next_id:
                if "error" in reply:
                    raise RuntimeError(reply["error"].get("message", "error"))
                return reply["result"]

    def call(self, name: str, arguments: dict) -> str:
        try:
            result = self.request("tools/call", {"name": name, "arguments": arguments})
        except RuntimeError as error:
            return f"Error: {error}"
        text = "\n".join(part.get("text", "") for part in result.get("content", []) if part.get("type") == "text")
        return ("Error: " + text) if result.get("isError") else text

    def close(self) -> None:
        try:
            self.process.stdin.close()
            self.process.wait(timeout=10)
        except Exception:
            self.process.kill()


def chat(api_url: str, body: dict, timeout: int) -> dict:
    request = urllib.request.Request(
        api_url.rstrip("/") + "/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + os.environ.get("MEMST_LLM_API_KEY", "proxy-injected")},
    )
    last = None
    for attempt in range(4):
        try:
            return json.loads(urllib.request.urlopen(request, timeout=timeout).read())
        except Exception as error:  # a transient failure of the endpoint
            last = error
            time.sleep(2 * (attempt + 1))
    raise RuntimeError(f"the model endpoint failed: {last}")


def run(memst: str, store: str, owner: str, prompt: str, model: str, api_url: str, tools: str = "", extra: dict = None, max_steps: int = 10, timeout: int = 180) -> dict:
    """One session with one prompt. `tools` is a comma-separated allowlist
    (empty for every tool the server offers)."""
    started = time.time()
    server = Server(memst, store, owner)
    try:
        allowed = set(filter(None, tools.split(",")))
        offered = [{"type": "function", "function": {"name": t["name"], "description": t.get("description", ""), "parameters": t.get("inputSchema", {"type": "object"})}} for t in server.tools if not allowed or t["name"] in allowed]
        messages = [{"role": "system", "content": SYSTEM + ("\n\nThe server says: " + server.instructions if server.instructions else "")}, {"role": "user", "content": prompt}]
        answer, tokens, calls, used = "", 0, 0, []
        for _ in range(max_steps):
            reply = chat(api_url, {"model": model, "messages": messages, "tools": offered, "max_tokens": 4000, **(extra or {})}, timeout)
            usage = reply.get("usage", {})
            tokens += int(usage.get("prompt_tokens", 0))
            calls += 1
            message = reply["choices"][0]["message"]
            messages.append(message)
            if not message.get("tool_calls"):
                answer = (message.get("content") or "").strip()
                break
            for tool_call in message["tool_calls"]:
                name = tool_call["function"]["name"]
                used.append(name)
                try:
                    arguments = json.loads(tool_call["function"].get("arguments") or "{}")
                    result = server.call(name, arguments) if (not allowed or name in allowed) else "Error: that tool is not available"
                except ValueError:
                    result = "Error: the arguments were not valid JSON"
                messages.append({"role": "tool", "tool_call_id": tool_call["id"], "content": result})
        return {"answer": answer, "prompt_tokens": tokens, "model_calls": calls, "tools": used, "seconds": round(time.time() - started, 1)}
    finally:
        server.close()
