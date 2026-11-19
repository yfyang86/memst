//! MemSt MCP - Model Context Protocol server
//!
//! Lets an MCP client (Claude Code, Cursor and others) read and write one
//! owner's memory in a MemSt store, over stdio.
//!
//! - [`policy`]: who may do what, and what may be written
//! - [`service`]: the memory operations
//! - [`adapter`]: the tools, and their arguments
//! - [`server`]: JSON-RPC over stdio

pub mod adapter;
pub mod dream;
mod migrate;
pub mod note;
pub mod page;
pub mod policy;
pub mod server;
pub mod service;

pub use adapter::{McpAdapter, McpContent, McpManifest, McpResource, McpTool, McpToolCall, McpToolResult};
pub use policy::{Scope, Screening, WritePolicy};
pub use server::{McpServer, PROTOCOL_VERSIONS};
pub use service::{MemoryService, Refusal};

/// The newest MCP protocol revision this server speaks.
pub const MCP_VERSION: &str = PROTOCOL_VERSIONS[0];
