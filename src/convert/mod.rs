//! GeoJSON → PMTiles conversion pipeline.
//!
//! # Module layout
//!
//! | File | Responsibility |
//! |------|---------------|
//! | `args.rs` | CLI `ConvertArgs` struct |
//! | `http.rs` | Axum HTTP handlers |
//! | `job.rs` | `ConvertJob` / `ConvertState` (HTTP job store) |
//! | `pipeline.rs` | Full conversion orchestrator |
//! | `tiler.rs` | Web Mercator tile math |
//! | `mvt_builder.rs` | MVT tile encoding |
//! | `writer.rs` | PMTiles archive writer |
//! | `progress.rs` | `ProgressReporter` trait + implementations |
//! | `input/` | Format-specific readers |
//!
//! # Public API (called from `main.rs` / `cli.rs`)
//!
//! ```ignore
//! convert::run_cli(args)?;          // CLI subcommand
//! convert::router(state)            // Axum HTTP sub-router
//! ```

pub mod args;
pub mod http;
pub mod input;
pub mod job;
pub mod mvt_builder;
pub mod pipeline;
pub mod progress;
pub mod tiler;
pub mod writer;

pub use args::ConvertArgs;
pub use job::ConvertState;

use anyhow::Result;
use axum::{routing, Extension, Router};
use std::{path::PathBuf, time::Duration};

/// Result of the CLI convert subcommand.
pub struct ConvertResult {
    /// Path to the written PMTiles file.
    pub output_path: PathBuf,
    /// Source ID used as the layer name (for --serve).
    pub source_id: String,
}

/// Entry point for the CLI `convert` subcommand.
///
/// Returns `ConvertResult` so that `main.rs` can start a tile server when
/// `--serve` is requested without duplicating startup logic.
pub fn run_cli(args: &ConvertArgs) -> Result<ConvertResult> {
    use pipeline::{run, ConvertOptions};
    use progress::IndicatifReporter;

    let output = args.resolve_output();
    let layer_name = args.resolve_layer_name();

    let opts = ConvertOptions {
        min_zoom: args.min_zoom,
        max_zoom: args.max_zoom,
        layer_name: layer_name.clone(),
        simplification: args.simplification,
        id_property: args.id_property.clone(),
        include_properties: args.include_properties.clone(),
        exclude_properties: args.exclude_properties.clone(),
    };

    let reporter = IndicatifReporter::new();
    run(&args.input, &output, &opts, &reporter)?;

    eprintln!("Written {} tiles to {}", layer_name, output.display());

    Ok(ConvertResult {
        output_path: output,
        source_id: layer_name,
    })
}

/// Build the Axum sub-router for HTTP conversion endpoints.
///
/// Mount this onto the main router with `.merge(convert::router(state))`.
pub fn router(state: ConvertState) -> Router {
    // Background TTL sweep: remove jobs older than 1 hour every 10 minutes
    let sweep_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(600));
        loop {
            interval.tick().await;
            sweep_state.sweep_expired(Duration::from_secs(3600));
        }
    });

    Router::new()
        .route("/convert", routing::post(http::start_conversion))
        .route("/convert/{id}/status", routing::get(http::job_status))
        .route(
            "/convert/{id}/download",
            routing::get(http::download_result),
        )
        .layer(Extension(state))
}
