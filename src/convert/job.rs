use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

/// Status of a conversion job.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Processing,
    Done,
    Failed,
}

/// A running or completed conversion job.
#[derive(Debug, Clone)]
pub struct ConvertJob {
    pub id: String,
    pub status: JobStatus,
    /// Progress from 0.0 to 1.0.
    pub progress: Arc<Mutex<f32>>,
    /// Path to the output PMTiles file (set when Done).
    pub output_path: Option<PathBuf>,
    /// Error message (set when Failed).
    pub error: Option<String>,
    /// When the job was created (for TTL cleanup).
    pub created_at: Instant,
}

impl ConvertJob {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            status: JobStatus::Processing,
            progress: Arc::new(Mutex::new(0.0)),
            output_path: None,
            error: None,
            created_at: Instant::now(),
        }
    }

    pub fn current_progress(&self) -> f32 {
        *self.progress.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// HTTP response body for GET /convert/{id}/status
#[derive(Serialize)]
pub struct JobStatusResponse {
    pub id: String,
    pub status: JobStatus,
    pub progress: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Shared job store for HTTP endpoint.
#[derive(Clone, Default)]
pub struct ConvertState {
    inner: Arc<RwLock<HashMap<String, ConvertJob>>>,
    temp_dir: Arc<PathBuf>,
}

impl ConvertState {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("tileserver-rs-convert");
        let _ = std::fs::create_dir_all(&temp_dir);
        Self {
            inner: Arc::default(),
            temp_dir: Arc::new(temp_dir),
        }
    }

    pub fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }

    pub fn insert(&self, job: ConvertJob) {
        self.inner
            .write()
            .expect("ConvertState lock poisoned")
            .insert(job.id.clone(), job);
    }

    /// Update a job's status and optional output path / error in place.
    pub fn update<F>(&self, id: &str, f: F)
    where
        F: FnOnce(&mut ConvertJob),
    {
        if let Ok(mut jobs) = self.inner.write() {
            if let Some(job) = jobs.get_mut(id) {
                f(job);
            }
        }
    }

    pub fn get_status(&self, id: &str) -> Option<JobStatusResponse> {
        let jobs = self.inner.read().ok()?;
        let job = jobs.get(id)?;
        Some(JobStatusResponse {
            id: job.id.clone(),
            status: job.status.clone(),
            progress: job.current_progress(),
            error: job.error.clone(),
        })
    }

    pub fn get_output_path(&self, id: &str) -> Option<PathBuf> {
        let jobs = self.inner.read().ok()?;
        let job = jobs.get(id)?;
        if job.status == JobStatus::Done {
            job.output_path.clone()
        } else {
            None
        }
    }

    /// Remove completed/failed jobs older than `ttl`.
    pub fn sweep_expired(&self, ttl: Duration) {
        if let Ok(mut jobs) = self.inner.write() {
            jobs.retain(|_, job| {
                job.status == JobStatus::Processing || job.created_at.elapsed() < ttl
            });
        }
    }
}
