//! Auto-detection of tile sources and styles from the filesystem.
//!
//! Scans a directory (or single file) and builds a [`Config`] by discovering
//! `.pmtiles`, `.mbtiles`, `style.json`, fonts, sprites, and GeoJSON files.

use anyhow::Context;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::config::{Config, SourceConfig, SourceType, StyleConfig};

/// A tile source discovered during auto-detection.
#[derive(Debug, Clone)]
pub struct AutoDetectedSource {
    pub id: String,
    pub source_type: SourceType,
    pub path: PathBuf,
}

/// A map style discovered during auto-detection.
#[derive(Debug, Clone)]
pub struct AutoDetectedStyle {
    pub id: String,
    pub path: PathBuf,
}

/// Summary of everything found during auto-detection.
#[derive(Debug, Clone)]
pub struct AutoDetectReport {
    pub target: PathBuf,
    pub sources: Vec<AutoDetectedSource>,
    pub styles: Vec<AutoDetectedStyle>,
    pub geojson_files: Vec<PathBuf>,
    pub fonts_dir: Option<PathBuf>,
    pub sprites_dir: Option<PathBuf>,
    pub conflicts: Vec<String>,
}

fn source_type_suffix(source_type: &SourceType) -> &'static str {
    match source_type {
        SourceType::PMTiles => "pmtiles",
        SourceType::MBTiles => "mbtiles",
        SourceType::Dir => "dir",
        SourceType::Tar => "tar",
        #[cfg(feature = "postgres")]
        SourceType::Postgres => "postgres",
        #[cfg(feature = "raster")]
        SourceType::Cog => "cog",
        #[cfg(feature = "raster")]
        SourceType::Vrt => "vrt",
        #[cfg(feature = "geoparquet")]
        SourceType::GeoParquet => "geoparquet",
        #[cfg(feature = "duckdb")]
        SourceType::DuckDB => "duckdb",
        #[cfg(feature = "stac")]
        SourceType::Stac => "stac",
        #[cfg(feature = "dem")]
        SourceType::Dem => "dem",
    }
}

fn detect_source_type(path: &Path) -> Option<SourceType> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "pmtiles" => Some(SourceType::PMTiles),
        "mbtiles" => Some(SourceType::MBTiles),
        #[cfg(feature = "geoparquet")]
        "parquet" | "geoparquet" => Some(SourceType::GeoParquet),
        #[cfg(feature = "duckdb")]
        "duckdb" => Some(SourceType::DuckDB),
        _ => None,
    }
}

fn detect_style_id(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if file_name == "style.json" {
        return path
            .parent()
            .and_then(|p| p.file_name())
            .map(|name| name.to_string_lossy().to_string());
    }

    if file_name.ends_with(".style.json") {
        let original = path.file_name()?.to_string_lossy().to_string();
        return Some(original.trim_end_matches(".style.json").to_string());
    }

    None
}

fn ensure_unique_id(base: &str, suffix: &str, used: &mut HashSet<String>) -> (String, bool) {
    if used.insert(base.to_string()) {
        return (base.to_string(), false);
    }

    let base_suffix = format!("{}-{}", base, suffix);
    if used.insert(base_suffix.clone()) {
        return (base_suffix, true);
    }

    let mut i = 2;
    loop {
        let candidate = format!("{}-{}-{}", base, suffix, i);
        if used.insert(candidate.clone()) {
            return (candidate, true);
        }
        i += 1;
    }
}

/// Build a [`SourceConfig`] for an auto-detected source, leaving all
/// optional/advanced fields at their auto-detect defaults.
fn auto_source_config(id: String, source_type: SourceType, path: &Path) -> SourceConfig {
    SourceConfig {
        id,
        source_type,
        path: path.to_string_lossy().to_string(),
        name: None,
        attribution: None,
        description: None,
        resampling: None,
        layer_name: None,
        geometry_column: None,
        minzoom: None,
        maxzoom: None,
        query: None,
        serve_as: None,
        #[cfg(feature = "raster")]
        colormap: None,
        options: None,
        collection: None,
        asset_role: "visual".to_string(),
        dynamic: false,
        max_items: 100,
        stac_bbox: None,
        pixel_selection: crate::config::PixelSelectionMethod::First,
        tile_path_template: None,
        tms: false,
        #[cfg(feature = "dem")]
        input_source: None,
        #[cfg(feature = "dem")]
        dem_encoding: crate::config::DemEncoding::Terrarium,
        #[cfg(feature = "dem")]
        dem_scale: None,
        #[cfg(feature = "dem")]
        dem_offset: None,
        #[cfg(feature = "dem")]
        dem_band: 1,
        #[cfg(feature = "dem")]
        dem_nodata_color: None,
    }
}

/// Handle a single-file auto-detect target, populating `config`/`report`.
///
/// Returns an error only for an unsupported file type.
fn detect_single_file(
    target: &Path,
    config: &mut Config,
    report: &mut AutoDetectReport,
) -> anyhow::Result<()> {
    if let Some(source_type) = detect_source_type(target) {
        let id = file_stem_id(target);
        config
            .sources
            .push(auto_source_config(id.clone(), source_type.clone(), target));
        report.sources.push(AutoDetectedSource {
            id,
            source_type,
            path: target.to_path_buf(),
        });
        return Ok(());
    }

    if let Some(style_id) = detect_style_id(target) {
        config.styles.push(StyleConfig {
            id: style_id.clone(),
            path: target.to_path_buf(),
            name: None,
        });
        report.styles.push(AutoDetectedStyle {
            id: style_id,
            path: target.to_path_buf(),
        });
        return Ok(());
    }

    let ext = target
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if ext == "geojson" {
        let parent = target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        config.files = Some(parent);
        report.geojson_files.push(target.to_path_buf());
        return Ok(());
    }

    anyhow::bail!("Unsupported file for auto-detection: {}", target.display());
}

/// Derive a source/style id from a path's file stem, defaulting to `"source"`.
fn file_stem_id(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "source".to_string())
}

/// Collect directories to scan: the target plus its immediate children, and
/// (one level deeper) the contents of any `styles/` child directory.
fn collect_scan_dirs(target: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut scan_dirs = vec![target.to_path_buf()];
    let mut children_dirs = Vec::new();

    for entry in std::fs::read_dir(target)
        .with_context(|| format!("failed to read directory {}", target.display()))?
    {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }
        children_dirs.push(path.clone());

        let is_styles_dir = path
            .file_name()
            .map(|name| name.to_string_lossy().eq_ignore_ascii_case("styles"))
            .unwrap_or(false);
        if is_styles_dir {
            for style_entry in std::fs::read_dir(&path)
                .with_context(|| format!("failed to read styles directory {}", path.display()))?
            {
                let style_path = style_entry?.path();
                if style_path.is_dir() {
                    children_dirs.push(style_path);
                }
            }
        }
    }

    children_dirs.sort();
    scan_dirs.extend(children_dirs);
    Ok(scan_dirs)
}

/// Source and style candidates discovered while scanning directories.
type Candidates = (Vec<(String, SourceType, PathBuf)>, Vec<(String, PathBuf)>);

/// Scan `scan_dirs`, classifying each file as a source, style, or GeoJSON.
fn collect_candidates(
    scan_dirs: &[PathBuf],
    report: &mut AutoDetectReport,
) -> anyhow::Result<Candidates> {
    let mut source_candidates: Vec<(String, SourceType, PathBuf)> = Vec::new();
    let mut style_candidates: Vec<(String, PathBuf)> = Vec::new();

    for dir in scan_dirs {
        for entry in std::fs::read_dir(dir)
            .with_context(|| format!("failed to read directory {}", dir.display()))?
        {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }

            if let Some(source_type) = detect_source_type(&path) {
                source_candidates.push((file_stem_id(&path), source_type, path));
            } else if let Some(style_id) = detect_style_id(&path) {
                style_candidates.push((style_id, path));
            } else if path
                .extension()
                .map(|e| e.to_string_lossy().eq_ignore_ascii_case("geojson"))
                .unwrap_or(false)
            {
                report.geojson_files.push(path);
            }
        }
    }

    source_candidates.sort_by(|a, b| a.2.cmp(&b.2));
    style_candidates.sort_by(|a, b| a.1.cmp(&b.1));
    report.geojson_files.sort();
    Ok((source_candidates, style_candidates))
}

/// Materialize source candidates into `config`/`report`, resolving id conflicts.
fn materialize_sources(
    source_candidates: Vec<(String, SourceType, PathBuf)>,
    config: &mut Config,
    report: &mut AutoDetectReport,
) {
    let mut used_source_ids = HashSet::new();
    for (base_id, source_type, path) in source_candidates {
        let suffix = source_type_suffix(&source_type);
        let (id, conflicted) = ensure_unique_id(&base_id, suffix, &mut used_source_ids);
        if conflicted {
            report.conflicts.push(format!(
                "Source ID '{}' conflicted; using '{}' for {}",
                base_id,
                id,
                path.display()
            ));
        }

        config
            .sources
            .push(auto_source_config(id.clone(), source_type.clone(), &path));
        report.sources.push(AutoDetectedSource {
            id,
            source_type,
            path,
        });
    }
}

/// Materialize style candidates into `config`/`report`, resolving id conflicts.
fn materialize_styles(
    style_candidates: Vec<(String, PathBuf)>,
    config: &mut Config,
    report: &mut AutoDetectReport,
) {
    let mut used_style_ids = HashSet::new();
    for (base_id, path) in style_candidates {
        let (id, conflicted) = ensure_unique_id(&base_id, "style", &mut used_style_ids);
        if conflicted {
            report.conflicts.push(format!(
                "Style ID '{}' conflicted; using '{}' for {}",
                base_id,
                id,
                path.display()
            ));
        }

        config.styles.push(StyleConfig {
            id: id.clone(),
            path: path.clone(),
            name: None,
        });
        report.styles.push(AutoDetectedStyle { id, path });
    }
}

/// Scan `target_path` and build a [`Config`] plus a report of what was found.
///
/// # Errors
///
/// Returns an error if the path does not exist or cannot be read.
pub fn detect_config(target_path: PathBuf) -> anyhow::Result<(Config, AutoDetectReport)> {
    if !target_path.exists() {
        anyhow::bail!("Auto-detect path does not exist: {}", target_path.display());
    }

    let target = target_path.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize auto-detect path: {}",
            target_path.display()
        )
    })?;

    let mut config = Config::default();
    let mut report = AutoDetectReport {
        target: target.clone(),
        sources: Vec::new(),
        styles: Vec::new(),
        geojson_files: Vec::new(),
        fonts_dir: None,
        sprites_dir: None,
        conflicts: Vec::new(),
    };

    if target.is_file() {
        detect_single_file(&target, &mut config, &mut report)?;
        return Ok((config, report));
    }

    let scan_dirs = collect_scan_dirs(&target)?;
    let (source_candidates, style_candidates) = collect_candidates(&scan_dirs, &mut report)?;
    materialize_sources(source_candidates, &mut config, &mut report);
    materialize_styles(style_candidates, &mut config, &mut report);

    let fonts_dir = target.join("fonts");
    if fonts_dir.is_dir() {
        config.fonts = Some(fonts_dir.clone());
        report.fonts_dir = Some(fonts_dir);
    }

    let sprites_dir = target.join("sprites");
    if sprites_dir.is_dir() {
        report.sprites_dir = Some(sprites_dir);
    }

    if !report.geojson_files.is_empty() {
        config.files = Some(target.clone());
    }

    Ok((config, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_auto_detect_directory_sources_styles_and_fonts() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();

        std::fs::write(root.join("openmaptiles.pmtiles"), b"mock").unwrap();
        std::fs::write(root.join("terrain.mbtiles"), b"mock").unwrap();

        let style_dir = root.join("styles/osm-bright");
        std::fs::create_dir_all(&style_dir).unwrap();
        std::fs::write(style_dir.join("style.json"), b"{}").unwrap();

        std::fs::create_dir_all(root.join("fonts")).unwrap();

        let (config, report) = detect_config(root.to_path_buf()).unwrap();

        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.styles.len(), 1);
        assert_eq!(config.styles[0].id, "osm-bright");
        assert_eq!(config.fonts, Some(root.join("fonts")));

        assert_eq!(report.sources.len(), 2);
        assert_eq!(report.styles.len(), 1);
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn test_auto_detect_disambiguates_conflicting_source_ids() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        std::fs::write(root.join("tiles.pmtiles"), b"mock").unwrap();
        std::fs::write(root.join("tiles.mbtiles"), b"mock").unwrap();

        let (config, report) = detect_config(root.to_path_buf()).unwrap();

        assert_eq!(config.sources.len(), 2);
        let ids: HashSet<_> = config.sources.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains("tiles"));
        assert!(ids.contains("tiles-pmtiles") || ids.contains("tiles-mbtiles"));
        assert!(!report.conflicts.is_empty());
    }

    #[test]
    #[cfg(feature = "stac")]
    fn test_source_type_suffix_stac() {
        assert_eq!(source_type_suffix(&SourceType::Stac), "stac");
    }

    #[test]
    fn test_source_type_suffix_known_types() {
        assert_eq!(source_type_suffix(&SourceType::PMTiles), "pmtiles");
        assert_eq!(source_type_suffix(&SourceType::MBTiles), "mbtiles");
    }

    #[test]
    fn test_detect_source_type_extensions() {
        assert_eq!(
            detect_source_type(Path::new("foo.pmtiles")),
            Some(SourceType::PMTiles)
        );
        assert_eq!(
            detect_source_type(Path::new("foo.mbtiles")),
            Some(SourceType::MBTiles)
        );
        assert_eq!(
            detect_source_type(Path::new("UPPER.PMTILES")),
            Some(SourceType::PMTiles)
        );
        assert_eq!(detect_source_type(Path::new("foo.txt")), None);
        assert_eq!(detect_source_type(Path::new("noext")), None);
    }

    #[cfg(feature = "geoparquet")]
    #[test]
    fn test_detect_source_type_geoparquet() {
        assert_eq!(
            detect_source_type(Path::new("a.parquet")),
            Some(SourceType::GeoParquet)
        );
        assert_eq!(
            detect_source_type(Path::new("a.geoparquet")),
            Some(SourceType::GeoParquet)
        );
    }

    #[cfg(feature = "duckdb")]
    #[test]
    fn test_detect_source_type_duckdb() {
        assert_eq!(
            detect_source_type(Path::new("a.duckdb")),
            Some(SourceType::DuckDB)
        );
    }

    #[test]
    fn test_detect_style_id_style_json() {
        assert_eq!(
            detect_style_id(Path::new("/styles/osm-bright/style.json")),
            Some("osm-bright".to_string())
        );
        assert_eq!(
            detect_style_id(Path::new("/styles/dark/STYLE.JSON")),
            Some("dark".to_string())
        );
    }

    #[test]
    fn test_detect_style_id_dotted_form() {
        assert_eq!(
            detect_style_id(Path::new("/styles/dark.style.json")),
            Some("dark".to_string())
        );
    }

    #[test]
    fn test_detect_style_id_non_match() {
        assert_eq!(detect_style_id(Path::new("/styles/foo.json")), None);
        assert_eq!(detect_style_id(Path::new("/styles/style.txt")), None);
        assert_eq!(detect_style_id(Path::new("/")), None);
    }

    #[test]
    fn test_ensure_unique_id_no_conflict() {
        let mut used = HashSet::new();
        let (id, conflicted) = ensure_unique_id("foo", "pmtiles", &mut used);
        assert_eq!(id, "foo");
        assert!(!conflicted);
    }

    #[test]
    fn test_ensure_unique_id_first_collision() {
        let mut used = HashSet::new();
        used.insert("foo".to_string());
        let (id, conflicted) = ensure_unique_id("foo", "pmtiles", &mut used);
        assert_eq!(id, "foo-pmtiles");
        assert!(conflicted);
    }

    #[test]
    fn test_ensure_unique_id_numbered_fallback() {
        let mut used = HashSet::new();
        used.insert("foo".to_string());
        used.insert("foo-pmtiles".to_string());
        let (id, conflicted) = ensure_unique_id("foo", "pmtiles", &mut used);
        assert_eq!(id, "foo-pmtiles-2");
        assert!(conflicted);

        let (id2, _) = ensure_unique_id("foo", "pmtiles", &mut used);
        assert_eq!(id2, "foo-pmtiles-3");
    }

    #[test]
    fn test_detect_config_nonexistent_path_errors() {
        let err = detect_config(PathBuf::from("/__definitely_does_not_exist__/x")).unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn test_detect_config_single_pmtiles_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("world.pmtiles");
        std::fs::write(&path, b"mock").unwrap();

        let (config, report) = detect_config(path.clone()).unwrap();

        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].id, "world");
        assert_eq!(report.sources.len(), 1);
        assert!(report.styles.is_empty());
    }

    #[test]
    fn test_detect_config_single_style_json() {
        let temp = TempDir::new().unwrap();
        let style_dir = temp.path().join("custom");
        std::fs::create_dir_all(&style_dir).unwrap();
        let style_path = style_dir.join("style.json");
        std::fs::write(&style_path, b"{}").unwrap();

        let (config, report) = detect_config(style_path).unwrap();

        assert_eq!(config.styles.len(), 1);
        assert_eq!(config.styles[0].id, "custom");
        assert_eq!(report.styles.len(), 1);
    }

    #[test]
    fn test_detect_config_single_geojson() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("data.geojson");
        std::fs::write(&path, b"{}").unwrap();

        let (config, report) = detect_config(path).unwrap();

        assert!(config.files.is_some());
        assert_eq!(report.geojson_files.len(), 1);
    }

    #[test]
    fn test_detect_config_single_unsupported_file_errors() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("data.xml");
        std::fs::write(&path, b"<x/>").unwrap();

        let err = detect_config(path).unwrap_err();
        assert!(err.to_string().contains("Unsupported"));
    }

    #[test]
    fn test_detect_config_directory_with_geojson() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::write(root.join("a.geojson"), b"{}").unwrap();
        std::fs::write(root.join("b.geojson"), b"{}").unwrap();

        let (config, report) = detect_config(root.clone()).unwrap();

        assert_eq!(report.geojson_files.len(), 2);
        assert_eq!(config.files, Some(root.clone()));
    }

    #[test]
    fn test_detect_config_directory_with_sprites_dir() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir_all(root.join("sprites")).unwrap();
        std::fs::write(root.join("a.pmtiles"), b"mock").unwrap();

        let (_config, report) = detect_config(root.clone()).unwrap();

        assert_eq!(report.sprites_dir, Some(root.join("sprites")));
    }

    #[test]
    fn test_detect_config_styles_subdir_traversal() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();

        std::fs::create_dir_all(root.join("styles/light")).unwrap();
        std::fs::create_dir_all(root.join("styles/dark")).unwrap();
        std::fs::write(root.join("styles/light/style.json"), b"{}").unwrap();
        std::fs::write(root.join("styles/dark/style.json"), b"{}").unwrap();

        let (config, report) = detect_config(root.clone()).unwrap();

        assert_eq!(config.styles.len(), 2);
        let ids: HashSet<_> = config.styles.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains("light"));
        assert!(ids.contains("dark"));
        assert_eq!(report.styles.len(), 2);
    }

    #[test]
    fn test_detect_config_dotted_style_files() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::write(root.join("light.style.json"), b"{}").unwrap();
        std::fs::write(root.join("dark.style.json"), b"{}").unwrap();

        let (config, _report) = detect_config(root).unwrap();

        assert_eq!(config.styles.len(), 2);
        let ids: HashSet<_> = config.styles.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains("light"));
        assert!(ids.contains("dark"));
    }

    #[test]
    fn test_detect_config_directory_mixed_sources_styles_geojson_sorted() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();

        std::fs::write(root.join("zebra.pmtiles"), b"mock").unwrap();
        std::fs::write(root.join("alpha.mbtiles"), b"mock").unwrap();
        std::fs::write(root.join("region.geojson"), b"{}").unwrap();

        let style_dir = root.join("styles/bright");
        std::fs::create_dir_all(&style_dir).unwrap();
        std::fs::write(style_dir.join("style.json"), b"{}").unwrap();

        let (config, report) = detect_config(root.clone()).unwrap();

        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.styles.len(), 1);
        assert_eq!(config.styles[0].id, "bright");
        assert_eq!(report.geojson_files.len(), 1);
        assert_eq!(config.files, Some(root.clone()));

        // source_candidates are sorted by path; "alpha.mbtiles" < "zebra.pmtiles"
        assert_eq!(config.sources[0].id, "alpha");
        assert_eq!(config.sources[1].id, "zebra");
        assert_eq!(config.sources[0].source_type, SourceType::MBTiles);
        assert_eq!(config.sources[1].source_type, SourceType::PMTiles);
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn test_detect_config_style_id_collision_disambiguation() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();

        std::fs::create_dir_all(root.join("styles/shared")).unwrap();
        std::fs::write(root.join("styles/shared/style.json"), b"{}").unwrap();
        std::fs::write(root.join("shared.style.json"), b"{}").unwrap();

        let (config, report) = detect_config(root).unwrap();

        assert_eq!(config.styles.len(), 2);
        let ids: HashSet<_> = config.styles.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains("shared"));
        assert!(ids.iter().any(|id| id.starts_with("shared-style")));
        assert!(!report.conflicts.is_empty());
    }
}
