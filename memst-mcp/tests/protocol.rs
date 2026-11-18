//! JSON-RPC over stdio, as an MCP client speaks it.

use memst_mcp::service::MemoryService;
use memst_mcp::{McpAdapter, McpServer, Scope, WritePolicy};
use serde_json::{json, Value};
use std::io::Cursor;
use std::sync::Arc;

fn server(dir: &std::path::Path, read_only: bool) -> McpServer {
    let service = MemoryService::open(dir, Scope::new("alice", read_only).unwrap(), WritePolicy::default()).unwrap();
    McpServer::new(McpAdapter::connected(Arc::new(service))).unwrap()
}

/// Feed lines to the server and collect the lines it answers with.
fn exchange(server: &McpServer, requests: &[Value]) -> Vec<Value> {
    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    let mut output = Vec::new();
    server.serve(Cursor::new(input), &mut output).unwrap();
    String::from_utf8(output).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

#[test]
fn a_session_from_handshake_to_tool_calls() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(dir.path(), false);
    let replies = exchange(
        &server,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "test-client", "version": "1"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            json!({"jsonrpc": "2.0", "id": "three", "method": "tools/call", "params": {
                "name": "memory_write", "arguments": {"content": "Alice prefers dark mode."}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
                "name": "memory_search", "arguments": {"query": "dark mode"}}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "ping"}),
        ],
    );
    // The notification is not answered: five replies to six messages.
    assert_eq!(replies.len(), 5);
    assert!(replies.iter().all(|r| r["jsonrpc"] == "2.0"));

    let init = &replies[0];
    assert_eq!((init["id"].as_i64(), init["result"]["protocolVersion"].as_str()), (Some(1), Some("2025-03-26")));
    assert_eq!(init["result"]["serverInfo"]["name"], "memst");
    assert!(init["result"]["capabilities"]["tools"].is_object());

    let tools = replies[1]["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["memory_search", "memory_read", "memory_get", "memory_status", "memory_write", "memory_forget"]);
    for tool in tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        assert!(tool.get("input_schema").is_none() && !tool["description"].as_str().unwrap().is_empty());
    }

    assert_eq!(replies[2]["id"], "three");
    let written = &replies[2]["result"];
    assert!(written["content"][0]["text"].as_str().unwrap().starts_with("Stored as note"));
    assert_eq!(written["content"][0]["type"], "text");
    assert!(written.get("isError").is_none());

    assert!(replies[3]["result"]["content"][0]["text"].as_str().unwrap().contains("Alice prefers dark mode."));
    assert_eq!(replies[4]["result"], json!({}));

    // The client's name is on the note it wrote.
    assert!(replies[3]["result"]["content"][0]["text"].as_str().unwrap().contains("source=mcp:test-client"));
}

#[test]
fn errors_follow_json_rpc() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(dir.path(), false);
    let code = |reply: &Value| reply["error"]["code"].as_i64();

    assert_eq!(code(&server.handle_line("{not json").unwrap()), Some(-32700));
    assert_eq!(code(&server.handle_line("[1, 2]").unwrap()), Some(-32600));
    assert_eq!(code(&server.handle_line(r#"{"jsonrpc":"2.0","id":1}"#).unwrap()), Some(-32600));
    let unknown = server.handle_line(r#"{"jsonrpc":"2.0","id":9,"method":"resources/list"}"#).unwrap();
    assert_eq!((code(&unknown), unknown["id"].as_i64()), (Some(-32601), Some(9)));
    assert_eq!(code(&server.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{}}"#).unwrap()), Some(-32602));
    // An unknown notification is ignored, as the protocol asks.
    assert!(server.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/whatever"}"#).is_none());

    // A tool that fails is a result with isError, not a protocol error, so
    // the model sees the reason.
    let failed = server
        .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_get","arguments":{"id":"nope"}}}"#)
        .unwrap();
    assert_eq!(failed["result"]["isError"], true);
    assert!(failed.get("error").is_none());
}

#[test]
fn an_unknown_protocol_version_gets_the_newest_one_we_speak() {
    let dir = tempfile::tempdir().unwrap();
    let reply = server(dir.path(), false)
        .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}"#)
        .unwrap();
    assert_eq!(reply["result"]["protocolVersion"], memst_mcp::MCP_VERSION);
}

#[test]
fn blank_lines_are_skipped_and_an_oversized_line_does_not_stop_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(dir.path(), false);
    let huge = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"ping","params":{{"pad":"{}"}}}}"#, "x".repeat(3 << 20));
    let input = format!("\n   \n{huge}\n{}\n", r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#);
    let mut output = Vec::new();
    server.serve(Cursor::new(input), &mut output).unwrap();
    let replies: Vec<Value> = String::from_utf8(output).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["error"]["message"], "request too large");
    assert_eq!((replies[1]["id"].as_i64(), &replies[1]["result"]), (Some(2), &json!({})));
}

#[test]
fn a_read_only_server_lists_no_writing_tools() {
    let dir = tempfile::tempdir().unwrap();
    drop(server(dir.path(), false)); // creates the store
    let reply = server(dir.path(), true).handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).unwrap();
    let names: Vec<&str> = reply["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["memory_search", "memory_read", "memory_get", "memory_status"]);
}
