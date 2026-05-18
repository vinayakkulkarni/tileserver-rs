//! Model Context Protocol (MCP) server integration.
//!
//! This module is only compiled when the `mcp` feature is enabled. It
//! exposes:
//!
//! - **Tier A introspection tools** — `tileserver_list_sources`,
//!   `tileserver_get_source_tilejson`, `tileserver_list_styles`,
//!   `tileserver_get_style`, `tileserver_get_tile_metadata`,
//!   `tileserver_get_server_info`.
//! - **Tier B data-access tools** — `tileserver_render_static_map`,
//!   `tileserver_get_tile`, `tileserver_query_features_at_point`, and
//!   (feature-gated) `tileserver_query_features_cql2` /
//!   `tileserver_search_stac_items`.
//! - **Tier D resource templates** — `tileserver://styles/{id}` and
//!   `tileserver://data/{id}.json`.
//!
//! Two transports are supported:
//!
//! 1. Streamable HTTP at `/mcp` (mounted on the main listener).
//! 2. stdio (`tileserver-rs mcp-stdio --config …`).

#![deny(clippy::correctness)]

pub mod error;
pub mod handlers;
pub mod resources;
pub mod transport;

pub use handlers::McpHandler;
pub use transport::{mcp_router, run_stdio};
