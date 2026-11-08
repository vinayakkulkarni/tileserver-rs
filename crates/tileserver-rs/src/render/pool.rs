//! Worker-thread renderer pool for concurrent tile rendering
//!
//! Each worker thread owns persistent MapLibre Native instances keyed by
//! pixel ratio. EGL contexts stay alive for the worker's lifetime, avoiding
//! race conditions in MapLibre Native's shared EGLDisplay singleton.
//! Work is distributed via a shared crossbeam channel and results return
//! through tokio oneshot channels.

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{self, Sender};
use tokio::sync::oneshot;

use super::native::{MapMode, NativeMap, RenderOptions, RenderedImage, Size};
use crate::error::{Result, TileServerError};

/// Configuration for the renderer pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub tile_size: u32,
    pub pool_size: usize,
    pub render_timeout: Duration,
    /// Requests that may wait for a free worker before `dispatch` rejects.
    pub queue_size: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            tile_size: 512,
            pool_size: 4,
            render_timeout: Duration::from_secs(30),
            queue_size: 256,
        }
    }
}

enum RenderJob {
    Tile {
        style_json: String,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
        scale: f32,
    },
    Static {
        style_json: String,
        options: RenderOptions,
    },
}

enum RenderOutput {
    Tile(Result<Vec<u8>>),
    Static(Result<RenderedImage>),
}

struct RenderRequest {
    job: RenderJob,
    response: oneshot::Sender<RenderOutput>,
}

pub struct RendererPool {
    sender: Option<Sender<RenderRequest>>,
    config: PoolConfig,
    max_scale: u8,
    worker_handles: Vec<thread::JoinHandle<()>>,
}

impl RendererPool {
    /// Start `pool_size` renderer worker threads.
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when MapLibre Native fails to
    /// initialise or a worker thread cannot be spawned.
    pub fn new(config: PoolConfig, max_scale: u8) -> Result<Self> {
        super::native::init()?;

        let pool_size = config.pool_size.max(1);
        let tile_size = config.tile_size;
        // Bounded: under a request storm, excess requests get 503 instead of
        // queueing without limit (each holds its style JSON in memory).
        let (sender, receiver) =
            crossbeam_channel::bounded::<RenderRequest>(config.queue_size.max(1));
        let mut handles = Vec::with_capacity(pool_size);

        for i in 0..pool_size {
            let rx = receiver.clone();
            let handle = thread::Builder::new()
                .name(format!("renderer-{i}"))
                .spawn(move || {
                    tracing::info!(worker = i, "renderer worker started");
                    let mut maps = WorkerMaps::new(tile_size);

                    while let Ok(request) = rx.recv() {
                        let output = match request.job {
                            RenderJob::Tile {
                                ref style_json,
                                z,
                                x,
                                y,
                                tile_size,
                                scale,
                            } => RenderOutput::Tile(
                                maps.render_tile(style_json, z, x, y, tile_size, scale),
                            ),
                            RenderJob::Static {
                                ref style_json,
                                ref options,
                            } => RenderOutput::Static(maps.render_static(style_json, options)),
                        };
                        let _ = request.response.send(output);
                    }
                    tracing::info!(worker = i, "renderer worker stopped");
                })
                .map_err(|e| {
                    TileServerError::RenderError(format!(
                        "failed to spawn renderer thread {i}: {e}"
                    ))
                })?;
            handles.push(handle);
        }

        tracing::info!(
            pool_size,
            tile_size = config.tile_size,
            max_scale,
            timeout_secs = config.render_timeout.as_secs(),
            "renderer pool initialized"
        );

        Ok(Self {
            sender: Some(sender),
            config,
            max_scale,
            worker_handles: handles,
        })
    }

    /// Render a raster tile via a worker thread
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderBusy`] when the render queue is full, and
    /// [`TileServerError::RenderError`] when the render fails or times out.
    pub async fn render_tile(
        &self,
        style_json: &str,
        z: u8,
        x: u32,
        y: u32,
        scale: u8,
    ) -> Result<Vec<u8>> {
        let render_start = std::time::Instant::now();
        let scale = scale.min(self.max_scale).max(1);
        let (tx, rx) = oneshot::channel();

        self.dispatch(RenderRequest {
            job: RenderJob::Tile {
                style_json: style_json.to_string(),
                z,
                x,
                y,
                tile_size: self.config.tile_size,
                scale: scale as f32,
            },
            response: tx,
        })?;

        let result = wait_for(rx, self.config.render_timeout, "render")
            .await
            .and_then(|output| match output {
                RenderOutput::Tile(result) => result,
                RenderOutput::Static(_) => Err(unexpected_output()),
            });
        record_render("tile", render_start, &result);
        result
    }

    /// Render a static map image via a worker thread
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderBusy`] when the render queue is full, and
    /// [`TileServerError::RenderError`] when the render fails or times out.
    pub async fn render_static(
        &self,
        style_json: &str,
        options: RenderOptions,
    ) -> Result<RenderedImage> {
        let render_start = std::time::Instant::now();
        let (tx, rx) = oneshot::channel();

        self.dispatch(RenderRequest {
            job: RenderJob::Static {
                style_json: style_json.to_string(),
                options,
            },
            response: tx,
        })?;

        let result = wait_for(rx, self.config.render_timeout, "static render")
            .await
            .and_then(|output| match output {
                RenderOutput::Static(result) => result,
                RenderOutput::Tile(_) => Err(unexpected_output()),
            });
        record_render("static", render_start, &result);
        result
    }

    #[must_use]
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            max_scale: self.max_scale,
            pool_size: self.config.pool_size,
        }
    }

    fn dispatch(&self, request: RenderRequest) -> Result<()> {
        self.sender
            .as_ref()
            .ok_or_else(|| TileServerError::RenderError("renderer pool is shut down".to_string()))?
            .try_send(request)
            .map_err(|e| match e {
                crossbeam_channel::TrySendError::Full(_) => TileServerError::RenderBusy,
                crossbeam_channel::TrySendError::Disconnected(_) => {
                    TileServerError::RenderError("all renderer workers have stopped".to_string())
                }
            })
    }
}

/// Wait for a worker's answer, at most `timeout`.
async fn wait_for(
    rx: oneshot::Receiver<RenderOutput>,
    timeout: std::time::Duration,
    what: &str,
) -> Result<RenderOutput> {
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(_)) => Err(TileServerError::RenderError(
            "renderer worker dropped response".to_string(),
        )),
        Err(_) => Err(TileServerError::RenderError(format!(
            "{what} timed out after {}s",
            timeout.as_secs()
        ))),
    }
}

fn unexpected_output() -> TileServerError {
    TileServerError::RenderError("unexpected render output type".to_string())
}

/// Record the render duration and outcome in the metrics.
fn record_render<T>(style: &'static str, started: std::time::Instant, result: &Result<T>) {
    let error = result.as_ref().err().map(ToString::to_string);
    crate::metrics::render_recorded(crate::metrics::RenderEvent {
        style,
        format: crate::sources::TileFormat::Png,
        duration: started.elapsed(),
        outcome: if error.is_some() {
            crate::metrics::RenderOutcome::Error
        } else {
            crate::metrics::RenderOutcome::Success
        },
        error_reason: error.as_deref(),
    });
}

impl Drop for RendererPool {
    fn drop(&mut self) {
        // Close the channel so workers exit their recv loop
        self.sender.take();
        for handle in self.worker_handles.drain(..) {
            if let Err(e) = handle.join() {
                tracing::error!("renderer worker panicked during shutdown: {e:?}");
            }
        }
        tracing::info!("renderer pool shut down");
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub max_scale: u8,
    pub pool_size: usize,
}

/// Per-worker persistent NativeMap instances keyed by scale factor.
/// Pixel ratio is fixed at NativeMap creation, so we keep one per distinct scale.
/// Maps are created lazily and reused across renders, keeping EGL contexts alive.
struct WorkerMaps {
    tile_maps: HashMap<u32, NativeMap>,
    static_map: Option<(u32, NativeMap)>,
    tile_size: u32,
}

impl WorkerMaps {
    fn new(tile_size: u32) -> Self {
        Self {
            tile_maps: HashMap::new(),
            static_map: None,
            tile_size,
        }
    }

    fn get_tile_map(&mut self, scale: f32) -> Result<&mut NativeMap> {
        let key = scale_key(scale);
        if !self.tile_maps.contains_key(&key) {
            let map = NativeMap::new(
                Size::new(self.tile_size, self.tile_size),
                scale,
                MapMode::Tile,
            )?;
            self.tile_maps.insert(key, map);
        }
        Ok(self
            .tile_maps
            .get_mut(&key)
            .expect("key was just inserted into tile_maps"))
    }

    fn render_tile(
        &mut self,
        style_json: &str,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
        scale: f32,
    ) -> Result<Vec<u8>> {
        let map = self.get_tile_map(scale)?;
        map.set_size(Size::new(tile_size, tile_size));
        map.load_style(style_json)?;
        let image = map.render_tile(z, x, y, tile_size, scale)?;
        image.to_png()
    }

    fn render_static(
        &mut self,
        style_json: &str,
        options: &RenderOptions,
    ) -> Result<RenderedImage> {
        let ratio_key = scale_key(options.pixel_ratio);
        let map = match reuse_if_same(self.static_map.take(), ratio_key) {
            Some(map) => map,
            None => NativeMap::new(options.size, options.pixel_ratio, MapMode::Static)?,
        };
        let (_, map) = self.static_map.insert((ratio_key, map));
        map.set_size(options.size);
        map.load_style(style_json)?;
        map.render(Some(RenderOptions {
            size: options.size,
            pixel_ratio: options.pixel_ratio,
            camera: options.camera,
            mode: options.mode,
        }))
    }
}

/// The cached map when it was built for `key`; `None` means build a new one.
fn reuse_if_same<T>(slot: Option<(u32, T)>, key: u32) -> Option<T> {
    slot.and_then(|(k, map)| (k == key).then_some(map))
}

/// Map cache key for a pixel ratio: hundredths, so 1.5 and 1.50 share a map.
fn scale_key(ratio: f32) -> u32 {
    (ratio * 100.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuse_if_same_keeps_only_a_matching_map() {
        assert_eq!(reuse_if_same(Some((100, "map")), 100), Some("map"));
        assert_eq!(reuse_if_same(Some((100, "map")), 200), None);
        assert_eq!(reuse_if_same::<&str>(None, 100), None);
    }

    #[test]
    fn scale_key_groups_ratios_by_hundredths() {
        assert_eq!(scale_key(1.0), 100);
        assert_eq!(scale_key(2.5), 250);
        assert_eq!(scale_key(1.5), scale_key(1.50));
        assert_ne!(scale_key(1.0), scale_key(2.0));
    }

    #[tokio::test]
    async fn wait_for_returns_the_answer_or_names_the_failure() {
        let (tx, rx) = oneshot::channel();
        tx.send(RenderOutput::Tile(Ok(vec![1]))).ok();
        let out = wait_for(rx, std::time::Duration::from_secs(1), "render").await;
        assert!(matches!(out, Ok(RenderOutput::Tile(Ok(ref b))) if b == &[1]));

        let (tx, rx) = oneshot::channel::<RenderOutput>();
        drop(tx);
        let dropped = wait_for(rx, std::time::Duration::from_secs(1), "render").await;
        assert!(
            dropped
                .err()
                .unwrap()
                .to_string()
                .contains("dropped response")
        );

        let (_tx, rx) = oneshot::channel::<RenderOutput>();
        let late = wait_for(rx, std::time::Duration::from_millis(10), "static render").await;
        assert!(
            late.err()
                .unwrap()
                .to_string()
                .contains("static render timed out")
        );
    }

    #[test]
    fn record_render_accepts_success_and_error() {
        let started = std::time::Instant::now();
        record_render("tile", started, &Ok::<_, TileServerError>(()));
        record_render::<()>("static", started, &Err(unexpected_output()));
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let config = PoolConfig::default();
        let pool = RendererPool::new(config, 3);
        assert!(pool.is_ok());
    }
}
