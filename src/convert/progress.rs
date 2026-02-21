use std::sync::{Arc, Mutex};

/// Abstracts progress reporting for both CLI and HTTP contexts.
pub trait ProgressReporter: Send + Sync {
    fn set_total(&self, total: u64);
    fn inc(&self, delta: u64);
    fn set_message(&self, msg: &str);
    fn finish(&self);
}

/// CLI progress reporter backed by `indicatif`.
pub struct IndicatifReporter {
    bar: indicatif::ProgressBar,
}

impl Default for IndicatifReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl IndicatifReporter {
    pub fn new() -> Self {
        use indicatif::{ProgressBar, ProgressStyle};
        let bar = ProgressBar::new(0);
        bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
                     {pos}/{len} tiles {msg}",
                )
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("=>-"),
        );
        Self { bar }
    }
}

impl ProgressReporter for IndicatifReporter {
    fn set_total(&self, total: u64) {
        self.bar.set_length(total);
    }
    fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }
    fn set_message(&self, msg: &str) {
        self.bar.set_message(msg.to_string());
    }
    fn finish(&self) {
        self.bar.finish_with_message("done");
    }
}

/// HTTP progress reporter: stores progress in a shared float.
pub struct HttpProgressReporter {
    progress: Arc<Mutex<f32>>,
    total: Mutex<u64>,
    done: Mutex<u64>,
}

impl HttpProgressReporter {
    pub fn new(progress: Arc<Mutex<f32>>) -> Self {
        Self {
            progress,
            total: Mutex::new(1),
            done: Mutex::new(0),
        }
    }
}

impl ProgressReporter for HttpProgressReporter {
    fn set_total(&self, total: u64) {
        *self.total.lock().unwrap() = total.max(1);
    }
    fn inc(&self, delta: u64) {
        let mut done = self.done.lock().unwrap();
        *done += delta;
        let total = *self.total.lock().unwrap();
        let pct = (*done as f32 / total as f32).min(1.0);
        *self.progress.lock().unwrap() = pct;
    }
    fn set_message(&self, _msg: &str) {}
    fn finish(&self) {
        *self.progress.lock().unwrap() = 1.0;
    }
}

/// A no-op reporter for tests or silent mode.
pub struct SilentReporter;

impl ProgressReporter for SilentReporter {
    fn set_total(&self, _total: u64) {}
    fn inc(&self, _delta: u64) {}
    fn set_message(&self, _msg: &str) {}
    fn finish(&self) {}
}
