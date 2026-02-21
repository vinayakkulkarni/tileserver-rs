use anyhow::{bail, Result};
use geo::{
    BooleanOps, BoundingRect, Geometry, Intersects, MultiLineString, Polygon, Rect, Simplify,
};
use rayon::prelude::*;
use std::path::Path;

use serde_json::{Map, Value};

use crate::convert::{
    input::read_features,
    mvt_builder::{build_tile_bytes, TileFeature},
    progress::ProgressReporter,
    tiler::{
        auto_max_zoom, dataset_bbox, expand_bbox, simplify_tolerance, tile_range_from_bbox,
        tile_to_bbox,
    },
    writer::PmTilesCollector,
};

/// Options for the conversion pipeline.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub min_zoom: u8,
    /// Maximum zoom level. `None` → auto-detected from the dataset.
    pub max_zoom: Option<u8>,
    pub layer_name: String,
    /// Base Douglas-Peucker tolerance at zoom 0, in degrees.
    /// `None` → pixel-based auto formula (~1.5 px at each zoom).
    pub simplification: Option<f64>,
    /// Property name to use as MVT feature ID (enables MapLibre feature state).
    /// `None` → features have no ID.
    pub id_property: Option<String>,
    /// If set, only these properties are included in tiles (whitelist).
    pub include_properties: Option<Vec<String>>,
    /// Properties to strip from tiles (blacklist). Ignored when `include_properties` is set.
    pub exclude_properties: Vec<String>,
}

/// Run the full conversion pipeline: read → tile → encode → write.
///
/// `reporter` receives progress updates; use `SilentReporter` to suppress output.
pub fn run(
    input: &Path,
    output: &Path,
    opts: &ConvertOptions,
    reporter: &dyn ProgressReporter,
) -> Result<()> {
    reporter.set_message("reading input");
    let features = read_features(input)?;

    if features.is_empty() {
        bail!("Input file contains no features");
    }

    tracing::info!(
        "Loaded {} features from {}",
        features.len(),
        input.display()
    );

    // Pre-compute bounding boxes once in parallel.
    // bounding_rect() traverses all coordinates of a geometry — for complex polygons
    // (e.g. country coastlines with 10k–50k points) this is expensive. Doing it
    // in parallel here pays the cost once for the entire pipeline instead of once
    // per zoom level in the candidate-tile loop plus once each in dataset_bbox /
    // auto_max_zoom / estimate_total_tiles.
    reporter.set_message("indexing");
    let bboxes: Vec<Option<Rect<f64>>> = features
        .par_iter()
        .map(|f| f.geometry.bounding_rect())
        .collect();

    let bbox = dataset_bbox(&bboxes)
        .unwrap_or_else(|| Rect::new((-180.0f64, -90.0f64), (180.0f64, 90.0f64)));

    // Resolve max_zoom: explicit value or auto-detect from dataset geometry
    let max_zoom = opts.max_zoom.unwrap_or_else(|| {
        let z = auto_max_zoom(&bboxes, features.len());
        tracing::info!("Auto-detected max zoom: {z}");
        z
    });

    // Estimate total tiles for progress reporting
    let total_tiles: u64 = estimate_total_tiles(&bboxes, opts.min_zoom, max_zoom);
    reporter.set_total(total_tiles);

    let mut collector =
        PmTilesCollector::new(opts.min_zoom, max_zoom, bbox, opts.layer_name.clone());

    for z in opts.min_zoom..=max_zoom {
        reporter.set_message(&format!("z{z}"));
        let tolerance = simplify_tolerance(z, opts.simplification);

        // Collect the set of candidate tile coords from pre-computed bounding boxes.
        // Using pre-computed bboxes avoids re-scanning all coordinates each zoom level.
        let mut candidate_tiles: std::collections::HashSet<(u32, u32)> =
            std::collections::HashSet::new();
        for bbox in bboxes.iter().flatten() {
            let range = tile_range_from_bbox(bbox, z);
            for coord in range.iter() {
                candidate_tiles.insert(coord);
            }
        }

        // For each tile: clip + simplify + encode in parallel.
        // Features are shared read-only across threads (Vec<Feature>: Sync).
        let layer_name = &opts.layer_name;
        let encoded: Result<Vec<(u32, u32, Vec<u8>)>> = candidate_tiles
            .into_par_iter()
            .map(|(x, y)| {
                let tile_bbox = tile_to_bbox(z, x, y);
                let buffered = expand_bbox(&tile_bbox, 0.01);

                // Clip and simplify each feature against this tile's bbox
                let tile_feats: Vec<TileFeature> = features
                    .iter()
                    .filter_map(|feat| {
                        let clipped = clip_geometry(&feat.geometry, &buffered)?;
                        let simplified = simplify_geometry(clipped, tolerance);
                        Some(TileFeature {
                            geometry: simplified,
                            properties: filter_properties(
                                &feat.properties,
                                opts.include_properties.as_deref(),
                                &opts.exclude_properties,
                            ),
                            id: resolve_feature_id(&feat.properties, opts.id_property.as_deref()),
                        })
                    })
                    .collect();

                let bytes = build_tile_bytes(&tile_feats, &tile_bbox, layer_name)?;
                reporter.inc(1);
                Ok((x, y, bytes))
            })
            .collect();

        // Insert into the Hilbert-ordered BTreeMap (sequential)
        for (x, y, bytes) in encoded? {
            collector.add_tile(z, x, y, bytes)?;
        }
    }

    tracing::info!(
        "Writing {} tiles to {}",
        collector.tile_count(),
        output.display()
    );
    reporter.set_message("writing");
    collector.write(output)?;
    reporter.finish();

    tracing::info!("Conversion complete: {}", output.display());
    Ok(())
}

/// Filter a feature's properties according to whitelist / blacklist rules.
///
/// - If `include` is `Some`, only keys in that slice are kept (whitelist wins).
/// - Otherwise, keys in `exclude` are removed (blacklist).
fn filter_properties(
    props: &Map<String, Value>,
    include: Option<&[String]>,
    exclude: &[String],
) -> Map<String, Value> {
    if let Some(whitelist) = include {
        props
            .iter()
            .filter(|(k, _)| whitelist.iter().any(|w| w == *k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    } else if exclude.is_empty() {
        props.clone()
    } else {
        props
            .iter()
            .filter(|(k, _)| !exclude.iter().any(|e| e == *k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// Extract a feature ID from a named property.
///
/// The value is converted to `u64`:
/// - JSON integers → cast directly
/// - JSON strings  → parsed as decimal integer
/// - Anything else → `None`
fn resolve_feature_id(props: &Map<String, Value>, id_property: Option<&str>) -> Option<u64> {
    let key = id_property?;
    match props.get(key)? {
        Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i as u64)),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

/// Clip a geometry to a bounding rectangle. Returns None if the result is empty.
fn clip_geometry(geom: &Geometry<f64>, bbox: &Rect<f64>) -> Option<Geometry<f64>> {
    // Quick reject: bounding box doesn't intersect tile at all
    if let Some(geom_bbox) = geom.bounding_rect() {
        if !geom_bbox.intersects(bbox) {
            return None;
        }
    }

    // Convert the tile bbox to a Polygon for BooleanOps
    let clip_poly: Polygon<f64> = (*bbox).into();

    match geom {
        Geometry::Point(p) => {
            if bbox.intersects(p) {
                Some(geom.clone())
            } else {
                None
            }
        }
        Geometry::MultiPoint(mp) => {
            let pts: Vec<_> =
                mp.0.iter()
                    .filter(|p| bbox.intersects(*p))
                    .cloned()
                    .collect();
            if pts.is_empty() {
                None
            } else {
                Some(Geometry::MultiPoint(geo::MultiPoint(pts)))
            }
        }
        Geometry::LineString(ls) => {
            let mls = MultiLineString::new(vec![ls.clone()]);
            let clipped = clip_poly.clip(&mls, false);
            if clipped.0.is_empty() {
                None
            } else {
                Some(Geometry::MultiLineString(clipped))
            }
        }
        Geometry::MultiLineString(mls) => {
            let clipped = clip_poly.clip(mls, false);
            if clipped.0.is_empty() {
                None
            } else {
                Some(Geometry::MultiLineString(clipped))
            }
        }
        Geometry::Polygon(poly) => {
            let clipped = poly.intersection(&clip_poly);
            if clipped.0.is_empty() {
                None
            } else {
                Some(Geometry::MultiPolygon(clipped))
            }
        }
        Geometry::MultiPolygon(mpoly) => {
            // Intersect each sub-polygon with the tile rect
            let parts: Vec<geo::Polygon<f64>> = mpoly
                .0
                .iter()
                .flat_map(|p| p.intersection(&clip_poly).0)
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(Geometry::MultiPolygon(geo::MultiPolygon(parts)))
            }
        }
        Geometry::GeometryCollection(gc) => {
            let parts: Vec<Geometry<f64>> =
                gc.0.iter().filter_map(|g| clip_geometry(g, bbox)).collect();
            if parts.is_empty() {
                None
            } else {
                Some(Geometry::GeometryCollection(geo::GeometryCollection(parts)))
            }
        }
        // Pass through for unsupported geometry types
        other => Some(other.clone()),
    }
}

/// Simplify a geometry using Douglas-Peucker with the given tolerance.
fn simplify_geometry(geom: Geometry<f64>, tolerance: f64) -> Geometry<f64> {
    if tolerance <= 0.0 {
        return geom;
    }
    match geom {
        Geometry::LineString(ls) => Geometry::LineString(ls.simplify(&tolerance)),
        Geometry::MultiLineString(mls) => Geometry::MultiLineString(mls.simplify(&tolerance)),
        Geometry::Polygon(p) => Geometry::Polygon(p.simplify(&tolerance)),
        Geometry::MultiPolygon(mp) => Geometry::MultiPolygon(mp.simplify(&tolerance)),
        other => other,
    }
}

/// Estimate total tile count for progress reporting.
/// Uses pre-computed bounding boxes to avoid re-scanning geometry coordinates.
fn estimate_total_tiles(bboxes: &[Option<Rect<f64>>], min_zoom: u8, max_zoom: u8) -> u64 {
    let mut total = 0u64;
    for z in min_zoom..=max_zoom {
        let mut seen = std::collections::HashSet::new();
        for bbox in bboxes.iter().flatten() {
            let range = tile_range_from_bbox(bbox, z);
            for coord in range.iter() {
                seen.insert(coord);
            }
        }
        total += seen.len() as u64;
    }
    total.max(1)
}
