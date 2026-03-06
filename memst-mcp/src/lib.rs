//! MemSt MCP - Model Context Protocol Adapter
//!
//! This crate provides MCP (Model Context Protocol) compatibility
//! for integration with Claude Code, Cursor, and other MCP-aware agents.

pub mod adapter;

pub use adapter::{McpAdapter, McpManifest, McpTool, McpToolCall, McpToolResult, McpContent, McpResource};

/// Version of the MCP protocol supported
pub const MCP_VERSION: &str = "2025-03-06";

/// MemSt MCP server
pub struct MemStMcpServer {
    adapter: McpAdapter,
}

impl MemStMcpServer {
    /// Create a new MCP server
    pub fn new() -> Self {
        Self {
            adapter: McpAdapter::new(),
        }
    }

    /// Get the adapter
    pub fn adapter(&self) -> &McpAdapter {
        &self.adapter
    }

    /// Get protocol version
    pub fn version(&self) -> &'static str {
        MCP_VERSION
    }
}

impl Default for MemStMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server() {
        let server = MemStMcpServer::new();
        assert_eq!(server.version(), MCP_VERSION);
    }
}
