//! CLI argument struct for `tileserver-rs convert`.

use clap::Args;
use std::path::PathBuf;

/// Long help text for the `convert` subcommand. Documents the WGS84 input
/// assumption and links to geolith for planet-scale workloads.
pub const LONG_ABOUT: &str = "\
Convert a GeoJSON or CSV file to PMTiles (MVT/PBF) for use with any vector tile
client.

Inputs are assumed to be WGS84 (EPSG:4326) lon/lat; output tiles are Web
Mercator (EPSG:3857), as required by the MVT spec.

Examples:
  tileserver-rs convert input.geojson --output out.pmtiles
  tileserver-rs convert cities.csv --lat latitude --lng longitude --output cities.pmtiles
  tileserver-rs convert input.geojson --serve --port 8080
  tileserver-rs convert big.geojson --auto-max-zoom --simplification 10

For planet-scale tile generation (OSM PBF, Overture Maps), see:
  https://github.com/geolith/geolith";

/// Arguments accepted by the `convert` subcommand.
#[derive(Args, Debug, Clone)]
#[command(long_about = LONG_ABOUT)]
pub struct ConvertArgs {
    /// Input file (`.geojson`, `.json`, `.csv`).
    #[arg(value_name = "INPUT", required = true)]
    pub input: PathBuf,

    /// Output PMTiles archive path.
    #[arg(short, long, value_name = "OUTPUT", required_unless_present = "serve")]
    pub output: Option<PathBuf>,

    /// Min zoom (inclusive).
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=18))]
    pub min_zoom: u8,

    /// Max zoom (inclusive).
    #[arg(long, default_value_t = 14, value_parser = clap::value_parser!(u8).range(0..=18))]
    pub max_zoom: u8,

    /// Auto-detect max zoom from data density (overrides `--max-zoom`).
    #[arg(long, conflicts_with = "max_zoom")]
    pub auto_max_zoom: bool,

    /// Douglas-Peucker tolerance (in tile units). Default: per-zoom formula.
    #[arg(long)]
    pub simplification: Option<f64>,

    /// Drop features from over-dense tiles (above 1000 features).
    #[arg(long, default_value_t = true)]
    pub drop_densest: bool,

    /// Layer name in output MVT. Default: input filename stem.
    #[arg(long)]
    pub layer_name: Option<String>,

    /// Property to use as feature ID. Auto-detects from
    /// {id, gid, ogc_fid, OBJECTID} if omitted.
    #[arg(long)]
    pub id_property: Option<String>,

    /// Comma-separated whitelist of property names to include.
    #[arg(long, value_delimiter = ',')]
    pub include_properties: Vec<String>,

    /// Comma-separated blacklist of property names to skip.
    #[arg(long, value_delimiter = ',')]
    pub exclude_properties: Vec<String>,

    /// CSV: column name holding WKT geometry. Auto-detects if omitted.
    #[arg(long)]
    pub geometry_column: Option<String>,

    /// CSV: latitude column name (implies POINT geometry from `--lat` + `--lng`).
    #[arg(long, conflicts_with = "geometry_column")]
    pub lat: Option<String>,

    /// CSV: longitude column name (requires `--lat`).
    #[arg(long, requires = "lat", conflicts_with = "geometry_column")]
    pub lng: Option<String>,

    /// After conversion, serve the result immediately.
    #[arg(long)]
    pub serve: bool,

    /// Port for `--serve` (overrides default 8080).
    #[arg(short, long, requires = "serve")]
    pub port: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    #[command(name = "convert")]
    struct TestCli {
        #[command(flatten)]
        args: ConvertArgs,
    }

    fn parse(argv: &[&str]) -> Result<ConvertArgs, clap::Error> {
        TestCli::try_parse_from(argv).map(|c| c.args)
    }

    #[test]
    fn parses_minimal_geojson_arg_set() {
        let args = parse(&["convert", "in.geojson", "--output", "out.pmtiles"]).unwrap();
        assert_eq!(args.input, PathBuf::from("in.geojson"));
        assert_eq!(args.output, Some(PathBuf::from("out.pmtiles")));
    }

    #[test]
    fn parses_csv_with_lat_lng_shortcut() {
        let args = parse(&[
            "convert",
            "cities.csv",
            "--lat",
            "latitude",
            "--lng",
            "longitude",
            "--output",
            "c.pmtiles",
        ])
        .unwrap();
        assert_eq!(args.lat.as_deref(), Some("latitude"));
        assert_eq!(args.lng.as_deref(), Some("longitude"));
    }

    #[test]
    fn lat_requires_lng() {
        let err = parse(&[
            "convert",
            "c.csv",
            "--lng",
            "longitude",
            "--output",
            "o.pmtiles",
        ]);
        assert!(err.is_err());
    }

    #[test]
    fn geometry_column_conflicts_with_lat() {
        let err = parse(&[
            "convert",
            "c.csv",
            "--geometry-column",
            "wkt",
            "--lat",
            "latitude",
            "--output",
            "o.pmtiles",
        ]);
        assert!(err.is_err());
    }

    #[test]
    fn default_min_zoom_is_zero() {
        let args = parse(&["convert", "in.geojson", "--output", "o.pmtiles"]).unwrap();
        assert_eq!(args.min_zoom, 0);
    }

    #[test]
    fn default_max_zoom_is_fourteen() {
        let args = parse(&["convert", "in.geojson", "--output", "o.pmtiles"]).unwrap();
        assert_eq!(args.max_zoom, 14);
    }

    #[test]
    fn output_required_unless_serve() {
        let missing = parse(&["convert", "in.geojson"]);
        assert!(missing.is_err());
        let with_serve = parse(&["convert", "in.geojson", "--serve"]).unwrap();
        assert!(with_serve.serve);
        assert!(with_serve.output.is_none());
    }

    #[test]
    fn port_requires_serve() {
        let err = parse(&[
            "convert",
            "in.geojson",
            "--output",
            "o.pmtiles",
            "--port",
            "9000",
        ]);
        assert!(err.is_err());
    }

    #[test]
    fn long_about_mentions_geolith() {
        assert!(LONG_ABOUT.contains("https://github.com/geolith/geolith"));
    }
}
