//! Unified config resolution with auto-detect fallback.
//!
//! Priority order:
//! 1. Explicit `--config path` (fail-fast if missing)
//! 2. Positional `detect_path` (auto-detect from that path)
//! 3. Default config locations (`config.toml`, `/etc/tileserver-rs/config.toml`)
//! 4. CWD auto-detect (scan current directory)

use anyhow::Context;
use std::path::PathBuf;

use crate::{autodetect::AutoDetectReport, config::Config};

/// Resolve the runtime configuration and optional auto-detect report.
///
/// # Errors
///
/// Returns an error if an explicit config path does not exist or the
/// auto-detect target is unreadable.
pub fn load_runtime_config(
    config_path: Option<PathBuf>,
    detect_path: Option<PathBuf>,
) -> anyhow::Result<(Config, Option<AutoDetectReport>)> {
    // 1. Explicit config — fail-fast if missing
    if let Some(path) = config_path {
        if !path.exists() {
            anyhow::bail!("Config file not found: {}", path.display());
        }
        return Ok((Config::load(Some(path))?, None));
    }

    // 2. Positional detect path
    if let Some(path) = detect_path {
        let (config, report) = crate::autodetect::detect_config(path)?;
        return Ok((config, Some(report)));
    }

    // 3. Default config locations
    let default_paths = vec![
        PathBuf::from("config.toml"),
        PathBuf::from("/etc/tileserver-rs/config.toml"),
    ];

    for path in default_paths {
        if path.exists() {
            return Ok((Config::load(Some(path))?, None));
        }
    }

    // 4. CWD auto-detect
    let cwd = std::env::current_dir().context("failed to get current working directory")?;
    let (config, report) = crate::autodetect::detect_config(cwd)?;
    Ok((config, Some(report)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_explicit_config_takes_precedence_over_auto_detect() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_path = root.join("config.toml");
        std::fs::write(
            &config_path,
            r#"
                [[sources]]
                id = "from-config"
                type = "pmtiles"
                path = "/tmp/from-config.pmtiles"
            "#,
        )
        .unwrap();

        std::fs::write(root.join("detected.pmtiles"), b"mock").unwrap();

        let (config, report) =
            load_runtime_config(Some(config_path), Some(root.to_path_buf())).unwrap();

        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].id, "from-config");
        assert!(report.is_none());
    }

    #[test]
    fn test_missing_explicit_config_fails() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing-config.toml");

        let result = load_runtime_config(Some(missing.clone()), None);

        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("Config file not found"));
        assert!(msg.contains(missing.to_string_lossy().as_ref()));
    }

    #[test]
    fn test_positional_detect_path_auto_detects() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(root.join("detected.pmtiles"), b"mock").unwrap();

        let (config, report) =
            load_runtime_config(None, Some(root.to_path_buf())).expect("detect must succeed");

        assert!(
            report.is_some(),
            "positional detect path must produce an auto-detect report"
        );
        // The auto-detect walk registers the mock archive as a source.
        assert_eq!(config.sources.len(), 1);
    }

    #[test]
    fn test_missing_detect_path_fails() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("no-such-dir");

        let result = load_runtime_config(None, Some(missing));

        assert!(
            result.is_err(),
            "auto-detect on an unreadable target must error"
        );
    }
}
