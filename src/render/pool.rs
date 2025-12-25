use chromiumoxide::browser::HeadlessMode;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::error::{Result, TileServerError};

/// Browser instance pool for rendering
pub struct BrowserPool {
    browser: Arc<Browser>,
    /// Semaphore to limit concurrent rendering operations
    semaphore: Arc<Semaphore>,
}

impl BrowserPool {
    /// Create a new browser pool
    pub async fn new(max_concurrent: usize) -> Result<Self> {
        tracing::info!("Initializing headless browser for rendering");

        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                .headless_mode(HeadlessMode::True)
                .disable_default_args()
                .args(vec![
                    "--no-sandbox",
                    "--disable-dev-shm-usage",
                    "--disable-gpu",
                    "--disable-software-rasterizer",
                    "--disable-extensions",
                    "--disable-setuid-sandbox",
                    "--no-first-run",
                    "--no-zygote",
                    "--single-process",
                    "--hide-scrollbars",
                    "--mute-audio",
                ])
                .build()
                .map_err(|e| TileServerError::RenderError(e.to_string()))?,
        )
        .await
        .map_err(|e| TileServerError::RenderError(e.to_string()))?;

        // Spawn handler in background
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = event {
                    tracing::error!("Browser handler error: {}", e);
                }
            }
        });

        tracing::info!(
            "Browser initialized with max {} concurrent renders",
            max_concurrent
        );

        Ok(Self {
            browser: Arc::new(browser),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        })
    }

    /// Get a reference to the browser
    pub fn browser(&self) -> Arc<Browser> {
        self.browser.clone()
    }

    /// Acquire a permit for rendering (limits concurrent operations)
    pub async fn acquire_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        self.semaphore
            .acquire()
            .await
            .map_err(|e| TileServerError::RenderError(format!("Failed to acquire permit: {}", e)))
    }
}
