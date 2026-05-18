//! Integration test for the MCP stdio transport.
//!
//! Rather than spawn a child process (which would require a built binary
//! on the path), this test wires the rmcp service directly against a
//! `tokio::io::duplex` channel — bytes written to one half are read by the
//! other, simulating the stdin/stdout pipe Claude Desktop uses.

#![cfg(feature = "mcp")]

mod common;

use std::sync::Arc;
use std::time::Duration;

use rmcp::ServiceExt;
use serde_json::{Value, json};
use tileserver_rs::mcp::McpHandler;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;

const PROTOCOL_VERSION: &str = "2025-03-26";

async fn read_one_line<R: AsyncBufRead + Unpin>(reader: &mut R) -> Value {
    let mut buf = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut buf))
        .await
        .expect("read_line timed out")
        .expect("read_line failed");
    assert!(!buf.is_empty(), "got empty line from stdio");
    serde_json::from_str(buf.trim()).unwrap_or_else(|e| panic!("invalid JSON `{buf}`: {e}"))
}

#[tokio::test]
async fn mcp_stdio_initialize_and_tools_list() {
    let shared = common::minimal_shared_state();
    let state = shared.load();

    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    let (server_reader, server_writer) = tokio::io::split(server_io);
    let (client_reader, mut client_writer) = tokio::io::split(client_io);

    let handler = McpHandler::new(Arc::clone(&state));
    let server_task = tokio::spawn(async move {
        let running = handler
            .serve((server_reader, server_writer))
            .await
            .expect("server handshake completed");
        let _ = running.waiting().await;
    });

    let mut reader = BufReader::new(client_reader);

    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "tileserver-rs-stdio-test", "version": "0.0.0" }
        }
    });
    client_writer
        .write_all(format!("{init}\n").as_bytes())
        .await
        .unwrap();
    client_writer.flush().await.unwrap();

    let response = read_one_line(&mut reader).await;
    assert_eq!(response["result"]["serverInfo"]["name"], "tileserver-rs");

    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    client_writer
        .write_all(format!("{initialized}\n").as_bytes())
        .await
        .unwrap();
    client_writer.flush().await.unwrap();

    let list = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    client_writer
        .write_all(format!("{list}\n").as_bytes())
        .await
        .unwrap();
    client_writer.flush().await.unwrap();

    let list_resp = read_one_line(&mut reader).await;
    let names: Vec<String> = list_resp["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect();
    assert!(names.contains(&"tileserver_list_sources".to_string()));
    assert!(names.contains(&"tileserver_get_server_info".to_string()));

    let call = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tileserver_list_sources",
            "arguments": {}
        }
    });
    client_writer
        .write_all(format!("{call}\n").as_bytes())
        .await
        .unwrap();
    client_writer.flush().await.unwrap();

    let call_resp = read_one_line(&mut reader).await;
    let content = &call_resp["result"]["content"];
    assert!(content.is_array(), "content not array: {call_resp}");

    drop(client_writer);
    drop(reader);
    let _ = timeout(Duration::from_secs(2), server_task).await;
}
