//! Progress reporting for the conversion pipeline, wrapping `indicatif`.

use indicatif::{ProgressBar, ProgressStyle};

/// A thin wrapper over an `indicatif` bar with feature and tile counters. When
/// constructed hidden (see [`Progress::hidden`]) all operations are no-ops,
/// which keeps tests and non-TTY runs quiet.
pub struct Progress {
    bar: ProgressBar,
    features: u64,
    tiles: u64,
}

impl Progress {
    /// Create a visible spinner-style progress bar.
    #[must_use]
    pub fn new() -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{spinner} {msg} ({elapsed})")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        Self {
            bar,
            features: 0,
            tiles: 0,
        }
    }

    /// Create a hidden bar; every method becomes a no-op. Used in tests.
    #[must_use]
    pub fn hidden() -> Self {
        Self {
            bar: ProgressBar::hidden(),
            features: 0,
            tiles: 0,
        }
    }

    /// Record `n` processed features and refresh the message.
    pub fn tick_features(&mut self, n: u64) {
        self.features += n;
        self.refresh();
    }

    /// Record `n` written tiles and refresh the message.
    pub fn tick_tiles(&mut self, n: u64) {
        self.tiles += n;
        self.refresh();
    }

    /// Number of features counted so far.
    #[must_use]
    pub fn features(&self) -> u64 {
        self.features
    }

    /// Number of tiles counted so far.
    #[must_use]
    pub fn tiles(&self) -> u64 {
        self.tiles
    }

    /// Finish and clear the bar.
    pub fn finish(self) {
        self.bar.finish_and_clear();
    }

    fn refresh(&self) {
        self.bar
            .set_message(format!("{} features → {} tiles", self.features, self.tiles));
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_features_advances_counter() {
        let mut p = Progress::hidden();
        p.tick_features(3);
        p.tick_features(2);
        assert_eq!(p.features(), 5);
    }

    #[test]
    fn tick_tiles_advances_counter() {
        let mut p = Progress::hidden();
        p.tick_tiles(4);
        assert_eq!(p.tiles(), 4);
    }

    #[test]
    fn counters_start_at_zero() {
        let p = Progress::hidden();
        assert_eq!(p.features(), 0);
        assert_eq!(p.tiles(), 0);
    }

    #[test]
    fn finish_consumes_without_panicking() {
        let p = Progress::hidden();
        p.finish();
    }

    #[test]
    fn new_builds_visible_bar_and_ticks() {
        let mut p = Progress::new();
        p.tick_features(2);
        p.tick_tiles(3);
        assert_eq!(p.features(), 2);
        assert_eq!(p.tiles(), 3);
        p.finish();
    }

    #[test]
    fn default_equals_new() {
        let p = Progress::default();
        assert_eq!(p.features(), 0);
        assert_eq!(p.tiles(), 0);
        p.finish();
    }
}
