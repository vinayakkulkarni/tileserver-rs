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
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            tile_size: 512,
            pool_size: 4,
            render_timeout: Duration::from_secs(30),
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
    pub fn new(config: PoolConfig, max_scale: u8) -> Result<Self> {
        super::native::init()?;

        let pool_size = config.pool_size.max(1);
        let tile_size = config.tile_size;
        let (sender, receiver) = crossbeam_channel::unbounded::<RenderRequest>();
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

        let result = match tokio::time::timeout(self.config.render_timeout, rx).await {
            Ok(Ok(RenderOutput::Tile(result))) => result,
            Ok(Ok(_)) => Err(TileServerError::RenderError(
                "unexpected render output type".to_string(),
            )),
            Ok(Err(_)) => Err(TileServerError::RenderError(
                "renderer worker dropped response".to_string(),
            )),
            Err(_) => Err(TileServerError::RenderError(format!(
                "render timed out after {}s",
                self.config.render_timeout.as_secs()
            ))),
        };

        let error_msg: Option<String> = match &result {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        };
        let (outcome, error_ref) = if error_msg.is_some() {
            (crate::metrics::RenderOutcome::Error, error_msg.as_deref())
        } else {
            (crate::metrics::RenderOutcome::Success, None)
        };
        crate::metrics::render_recorded(crate::metrics::RenderEvent {
            style: "tile",
            format: crate::sources::TileFormat::Png,
            duration: render_start.elapsed(),
            outcome,
            error_reason: error_ref,
        });

        result
    }

    /// Render a static map image via a worker thread
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

        let result = match tokio::time::timeout(self.config.render_timeout, rx).await {
            Ok(Ok(RenderOutput::Static(result))) => result,
            Ok(Ok(_)) => Err(TileServerError::RenderError(
                "unexpected render output type".to_string(),
            )),
            Ok(Err(_)) => Err(TileServerError::RenderError(
                "renderer worker dropped response".to_string(),
            )),
            Err(_) => Err(TileServerError::RenderError(format!(
                "static render timed out after {}s",
                self.config.render_timeout.as_secs()
            ))),
        };

        let error_msg: Option<String> = match &result {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        };
        let (outcome, error_ref) = if error_msg.is_some() {
            (crate::metrics::RenderOutcome::Error, error_msg.as_deref())
        } else {
            (crate::metrics::RenderOutcome::Success, None)
        };
        crate::metrics::render_recorded(crate::metrics::RenderEvent {
            style: "static",
            format: crate::sources::TileFormat::Png,
            duration: render_start.elapsed(),
            outcome,
            error_reason: error_ref,
        });

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
            .send(request)
            .map_err(|_| {
                TileServerError::RenderError("all renderer workers have stopped".to_string())
            })
    }
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
        let key = (scale * 100.0) as u32;
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
        let ratio_key = (options.pixel_ratio * 100.0) as u32;
        let needs_new = match &self.static_map {
            Some((key, _)) => *key != ratio_key,
            None => true,
        };

        if needs_new {
            let map = NativeMap::new(options.size, options.pixel_ratio, MapMode::Static)?;
            self.static_map = Some((ratio_key, map));
        }

        let (_, map) = self
            .static_map
            .as_mut()
            .expect("static_map was initialized above");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creation() {
        let config = PoolConfig::default();
        let pool = RendererPool::new(config, 3);
        assert!(pool.is_ok());
    }
}
