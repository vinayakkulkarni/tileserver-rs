use clap::Args;
use std::path::PathBuf;

/// Convert geospatial data to PMTiles archives.
#[derive(Args, Debug, Clone)]
pub struct ConvertArgs {
    /// Input GeoJSON file to convert
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    /// Output PMTiles file [default: <input>.pmtiles]
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Minimum zoom level
    #[arg(long, default_value = "0")]
    pub min_zoom: u8,

    /// Maximum zoom level [default: auto-detected from dataset geometry]
    #[arg(long)]
    pub max_zoom: Option<u8>,

    /// MVT layer name [default: input filename stem]
    #[arg(long)]
    pub layer_name: Option<String>,

    /// Douglas-Peucker simplification tolerance at zoom 0 (auto if unset)
    #[arg(long)]
    pub simplification: Option<f64>,

    /// Property name to use as MVT feature ID (enables MapLibre feature state)
    #[arg(long)]
    pub id_property: Option<String>,

    /// Only include these properties in tiles (comma-separated whitelist)
    #[arg(long, value_delimiter = ',')]
    pub include_properties: Option<Vec<String>>,

    /// Strip these properties from tiles (comma-separated blacklist)
    #[arg(long, value_delimiter = ',')]
    pub exclude_properties: Vec<String>,

    /// After conversion, start the tile server on the resulting file
    #[arg(long)]
    pub serve: bool,

    /// Port to bind when using --serve
    #[arg(short, long, default_value = "8080")]
    pub port: u16,
}

impl ConvertArgs {
    /// Resolve the output path: explicit --output or <input stem>.pmtiles
    pub fn resolve_output(&self) -> PathBuf {
        if let Some(ref out) = self.output {
            return out.clone();
        }
        let stem = self
            .input
            .file_stem()
            .unwrap_or_else(|| self.input.as_os_str());
        let mut out = self.input.with_file_name(stem);
        out.set_extension("pmtiles");
        out
    }

    /// Resolve the MVT layer name: explicit --layer-name or input filename stem
    pub fn resolve_layer_name(&self) -> String {
        if let Some(ref name) = self.layer_name {
            return name.clone();
        }
        self.input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("layer")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_with_input(input: &str) -> ConvertArgs {
        ConvertArgs {
            input: PathBuf::from(input),
            output: None,
            min_zoom: 0,
            max_zoom: None,
            layer_name: None,
            simplification: None,
            id_property: None,
            include_properties: None,
            exclude_properties: vec![],
            serve: false,
            port: 8080,
        }
    }

    #[test]
    fn resolve_output_defaults_to_input_stem_with_pmtiles_extension() {
        let args = args_with_input("/tmp/my_data.geojson");
        assert_eq!(args.resolve_output(), PathBuf::from("/tmp/my_data.pmtiles"));
    }

    #[test]
    fn resolve_output_respects_explicit_flag() {
        let mut args = args_with_input("/tmp/input.geojson");
        args.output = Some(PathBuf::from("/out/custom.pmtiles"));
        assert_eq!(args.resolve_output(), PathBuf::from("/out/custom.pmtiles"));
    }

    #[test]
    fn resolve_output_handles_file_without_extension() {
        let args = args_with_input("/tmp/data");
        // file_stem of "data" is "data"
        assert_eq!(args.resolve_output(), PathBuf::from("/tmp/data.pmtiles"));
    }

    #[test]
    fn resolve_layer_name_defaults_to_file_stem() {
        let args = args_with_input("/data/roads.geojson");
        assert_eq!(args.resolve_layer_name(), "roads");
    }

    #[test]
    fn resolve_layer_name_respects_explicit_flag() {
        let mut args = args_with_input("/data/roads.geojson");
        args.layer_name = Some("my_layer".to_string());
        assert_eq!(args.resolve_layer_name(), "my_layer");
    }

    #[test]
    fn resolve_layer_name_strips_all_extensions() {
        let args = args_with_input("/data/admin_boundaries.geojson");
        assert_eq!(args.resolve_layer_name(), "admin_boundaries");
    }
}
