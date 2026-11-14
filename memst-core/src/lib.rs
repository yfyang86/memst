//! MemSt Core - Git-like memory architecture for LLM session management
//!
//! This crate provides the core data structures and storage logic for
//! content-addressable memory management with Git-like semantics.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

// A search backend is required; fail with a clear message rather than with
// dozens of type errors inside the search module.
#[cfg(not(any(feature = "native-backend", feature = "tantivy-backend")))]
compile_error!("memst-core needs a search backend: enable the `native-backend` or `tantivy-backend` feature");

pub mod config;
pub mod embed;
pub mod context;
pub mod error;
pub mod extract;
pub mod graph;
pub mod hybrid;
pub mod llm;
pub mod memory;
pub mod objects;
pub mod search;
pub mod store;
pub mod text;
pub mod types;
pub mod vector;
