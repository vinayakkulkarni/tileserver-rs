use anyhow::Context;
use std::path::PathBuf;

use crate::{autodetect::AutoDetectReport, config::Config};

pub fn load_runtime_config(
    config_path: Option<PathBuf>,
    detect_path: Option<PathBuf>,
) -> anyhow::Result<(Config, Option<AutoDetectReport>)> {
    if let Some(path) = config_path {
        if !path.exists() {
            anyhow::bail!("Config file not found: {}", path.display());
        }
        return Ok((Config::load(Some(path))?, None));
    }

    if let Some(path) = detect_path {
        let (config, report) = crate::autodetect::detect_config(path)?;
        return Ok((config, Some(report)));
    }

    let default_paths = vec![
        PathBuf::from("config.toml"),
        PathBuf::from("/etc/tileserver-rs/config.toml"),
    ];

    for path in default_paths {
        if path.exists() {
            return Ok((Config::load(Some(path))?, None));
        }
    }

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
}
