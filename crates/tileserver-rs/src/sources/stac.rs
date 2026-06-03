//! STAC catalog-driven COG discovery and tile serving.
//!
//! This module implements a tile source that discovers Cloud-Optimized GeoTIFF
//! (COG) assets from STAC API catalogs and serves them as raster tiles.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::cog::CogSource;
use crate::sources::{TileData, TileFormat, TileMetadata, TileSource};

/// MIME type for Cloud-Optimized GeoTIFF files.
const COG_MIME_TYPE: &str = "image/tiff; application=geotiff; profile=cloud-optimized";

/// MIME type prefix for standard GeoTIFF files.
const GEOTIFF_MIME_TYPE: &str = "image/tiff";

/// A discovered COG asset from a STAC catalog item.
///
/// The priority metadata (`datetime`, `cloud_cover`) drives pixel-selection
/// strategies such as `lowest_cloud_cover` and `most_recent`; both are
/// optional because STAC items are not required to provide them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacAsset {
    /// Unique identifier from the STAC item.
    pub id: String,
    /// URL to the COG file.
    pub href: String,
    /// Bounding box as `[west, south, east, north]`.
    pub bbox: [f64; 4],
    /// Human-readable title of the asset.
    pub title: String,
    /// RFC 3339 capture timestamp from `properties.datetime`, when present.
    /// Used by the `most_recent` pixel-selection method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
    /// Cloud cover percentage `[0.0, 100.0]` from `properties.eo:cloud_cover`,
    /// when present. Used by the `lowest_cloud_cover` pixel-selection method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_cover: Option<f64>,
}

/// A tile source backed by STAC API catalog discovery.
///
/// Queries a STAC API for items in a collection, extracts COG asset URLs,
/// and delegates tile serving to either the first-asset [`CogSource`] (Phase 1)
/// or a dynamic per-tile bbox-search path (Phase 2/3, when `dynamic=true`).
pub struct StacSource {
    id: String,
    metadata: TileMetadata,
    cog_source: Option<Arc<CogSource>>,
    discovered_assets: Vec<StacAsset>,
    /// Phase 2/3: dynamic per-tile discovery config (populated when
    /// `dynamic=true` in [`SourceConfig`]).
    dynamic: Option<DynamicConfig>,
}

/// Configuration needed to re-query STAC for each incoming tile (Phase 2+).
#[derive(Clone)]
struct DynamicConfig {
    /// STAC API root URL (same as static path).
    api_url: String,
    /// Collection id to search within.
    collection: String,
    /// Asset role used to pick the COG asset out of returned items.
    asset_role: String,
    /// Upper bound on items inspected per tile.
    max_items: usize,
    /// Base SourceConfig cloned into each ad-hoc [`CogSource`].
    template: SourceConfig,
}

impl StacSource {
    /// Create a new STAC source from configuration.
    ///
    /// Discovers COG assets from the configured STAC API endpoint and
    /// initializes a [`CogSource`] for the first discovered asset.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::StacError`] if:
    /// - The `collection` field is missing from config
    /// - The STAC API request fails or returns invalid JSON
    /// - No features are found in the STAC response
    pub async fn new(config: &SourceConfig) -> Result<Self> {
        let api_url = &config.path;
        let collection = config.collection.as_deref().ok_or_else(|| {
            TileServerError::StacError("'collection' is required for stac sources".to_string())
        })?;
        let asset_role = &config.asset_role;
        let max_items = config.max_items;

        let assets =
            discover_assets(api_url, collection, asset_role, max_items, config.stac_bbox).await?;

        let bounds = config.stac_bbox.or_else(|| compute_merged_bounds(&assets));
        let name = config
            .name
            .clone()
            .unwrap_or_else(|| format!("STAC: {collection}"));

        let cog_source = if let Some(first_asset) = assets.first() {
            let cog_config = SourceConfig {
                id: config.id.clone(),
                source_type: crate::config::SourceType::Cog,
                path: to_gdal_cog_path(&first_asset.href),
                name: config.name.clone(),
                attribution: config.attribution.clone(),
                description: config.description.clone(),
                resampling: config.resampling,
                layer_name: None,
                geometry_column: None,
                query: None,
                minzoom: config.minzoom,
                maxzoom: config.maxzoom,
                serve_as: None,
                #[cfg(feature = "raster")]
                colormap: config.colormap.clone(),
                options: None,
                collection: config.collection.clone(),
                asset_role: config.asset_role.clone(),
                dynamic: config.dynamic,
                max_items: config.max_items,
                stac_bbox: config.stac_bbox,
                pixel_selection: config.pixel_selection,
                tile_path_template: None,
                tms: false,
            };

            match CogSource::from_file(&cog_config).await {
                Ok(source) => Some(Arc::new(source)),
                Err(e) => {
                    tracing::warn!(
                        "failed to create CogSource for first STAC asset {}: {e}",
                        first_asset.href
                    );
                    None
                }
            }
        } else {
            None
        };

        let dynamic = if config.dynamic {
            Some(DynamicConfig {
                api_url: api_url.to_string(),
                collection: collection.to_string(),
                asset_role: asset_role.to_string(),
                max_items,
                template: config.clone(),
            })
        } else {
            None
        };

        let metadata = TileMetadata {
            id: config.id.clone(),
            name,
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format: TileFormat::Png,
            minzoom: config.minzoom.unwrap_or(0),
            maxzoom: config.maxzoom.unwrap_or(18),
            bounds,
            center: bounds.map(|b| [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0, 10.0]),
            vector_layers: None,
        };

        Ok(Self {
            id: config.id.clone(),
            metadata,
            cog_source,
            discovered_assets: assets,
            dynamic,
        })
    }

    /// Returns the list of discovered STAC assets.
    #[must_use]
    pub fn discovered_assets(&self) -> &[StacAsset] {
        &self.discovered_assets
    }

    /// Returns the source identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

#[async_trait]
impl TileSource for StacSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        if let Some(ref dyn_cfg) = self.dynamic {
            return self.get_tile_dynamic(dyn_cfg, z, x, y).await;
        }
        match self.cog_source.as_ref() {
            Some(cog) => cog.get_tile(z, x, y).await,
            None => Ok(None),
        }
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl StacSource {
    /// Phase 2/3: resolve a single tile via dynamic STAC bbox-search.
    ///
    /// 1. Convert XYZ to a WGS-84 bbox.
    /// 2. POST `/search` with `bbox` + `collections` + `limit` to find
    ///    items whose footprints intersect the tile.
    /// 3. If the result set contains exactly one item, render directly
    ///    from its COG (Phase 2).
    /// 4. Otherwise composite the top-N candidates into a single tile
    ///    by rendering each and painting them in order (Phase 3 mosaic).
    async fn get_tile_dynamic(
        &self,
        cfg: &DynamicConfig,
        z: u8,
        x: u32,
        y: u32,
    ) -> Result<Option<TileData>> {
        let bbox = tile_to_wgs84_bbox(z, x, y);
        let assets = match discover_assets_by_bbox(
            &cfg.api_url,
            &cfg.collection,
            &cfg.asset_role,
            cfg.max_items,
            bbox,
        )
        .await
        {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(error = %e, z, x, y, "STAC bbox search failed; returning empty tile");
                return Ok(None);
            }
        };

        if assets.is_empty() {
            return Ok(None);
        }

        if assets.len() == 1 {
            return render_single_asset(&assets[0], &cfg.template, z, x, y).await;
        }

        composite_mosaic(&assets, &cfg.template, z, x, y).await
    }
}

/// Discover COG assets from a STAC API catalog.
///
/// Sends a POST search request to the STAC API and extracts COG assets
/// from the returned item collection.
///
/// # Errors
///
/// Returns [`TileServerError::StacError`] if the HTTP request fails,
/// the response status is non-2xx, or the response JSON is malformed.
pub async fn discover_assets(
    api_url: &str,
    collection: &str,
    asset_role: &str,
    max_items: usize,
    bbox: Option<[f64; 4]>,
) -> Result<Vec<StacAsset>> {
    let search_url = build_search_url(api_url);
    let client = reqwest::Client::builder()
        .user_agent("tileserver-rs/stac")
        .build()
        .map_err(|e| TileServerError::StacError(format!("failed to create HTTP client: {e}")))?;

    let mut search_body = serde_json::json!({
        "collections": [collection],
        "limit": max_items
    });
    if let Some(b) = bbox {
        search_body["bbox"] = serde_json::json!([b[0], b[1], b[2], b[3]]);
    }

    let response = client
        .post(&search_url)
        .json(&search_body)
        .send()
        .await
        .map_err(|e| TileServerError::StacError(format!("stac api search failed: {e}")))?;

    if !response.status().is_success() {
        return Err(TileServerError::StacError(format!(
            "stac api returned status {}",
            response.status()
        )));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| TileServerError::StacError(format!("failed to parse stac response: {e}")))?;

    extract_assets_from_item_collection(&body, asset_role)
}

/// Build the STAC API search endpoint URL from a base API URL.
#[must_use]
pub fn build_search_url(api_url: &str) -> String {
    let base = api_url.trim_end_matches('/');
    format!("{base}/search")
}

/// Extract COG assets from a STAC item collection JSON response.
///
/// Deserialises the body as a [`stac::ItemCollection`] (community-maintained
/// spec types from the `stac` crate), then walks each [`stac::Item`] and
/// picks the best COG [`stac::Asset`] — first by matching [`asset_role`],
/// falling back to the first asset whose `type` is a COG/GeoTIFF MIME type.
///
/// The crate deserialiser is intentionally lenient (unknown top-level
/// keys land in `additional_fields`), so we gate on the presence of at
/// least one of the STAC discriminators (`type: "FeatureCollection"` or
/// a `features` array) before accepting the payload. This protects
/// against configuration errors where a non-STAC endpoint silently
/// deserialises as an empty catalogue.
///
/// # Errors
///
/// Returns [`TileServerError::StacError`] if the body fails either the
/// STAC-shape check or [`stac::ItemCollection`] deserialisation.
pub fn extract_assets_from_item_collection(
    body: &serde_json::Value,
    asset_role: &str,
) -> Result<Vec<StacAsset>> {
    let looks_like_feature_collection = body.get("type").and_then(|v| v.as_str())
        == Some("FeatureCollection")
        || body.get("features").is_some();
    if !looks_like_feature_collection {
        return Err(TileServerError::StacError(
            "stac response is missing both 'type: FeatureCollection' and 'features'".to_string(),
        ));
    }

    let collection: stac::ItemCollection = serde_json::from_value(body.clone()).map_err(|e| {
        TileServerError::StacError(format!("stac response is not a valid ItemCollection: {e}"))
    })?;

    let assets = collection
        .items
        .iter()
        .filter_map(|item| extract_cog_asset_from_item(item, asset_role))
        .collect();

    Ok(assets)
}

/// Extract a single COG asset from a typed [`stac::Item`], first by
/// `asset_role`, then by COG/GeoTIFF MIME type.
///
/// Populates priority metadata (`datetime`, `cloud_cover`) from the
/// item's `properties` map when the corresponding STAC fields are
/// present.  Missing fields are preserved as `None` so mosaic methods
/// that depend on them can degrade gracefully (e.g.
/// `lowest_cloud_cover` treats `None` as worst cloud cover).
#[must_use]
pub fn extract_cog_asset_from_item(item: &stac::Item, asset_role: &str) -> Option<StacAsset> {
    let bbox = item.bbox.as_ref().map(bbox_to_2d)?;

    let (asset_key, asset) = find_asset_by_role(item.assets.iter(), asset_role)
        .or_else(|| find_asset_by_cog_mime(item.assets.iter()))?;

    Some(StacAsset {
        id: item.id.clone(),
        href: asset.href.clone(),
        bbox,
        title: asset.title.clone().unwrap_or_else(|| {
            if item.id.is_empty() {
                asset_key.clone()
            } else {
                item.id.clone()
            }
        }),
        datetime: extract_datetime(item),
        cloud_cover: extract_cloud_cover(item),
    })
}

fn extract_datetime(item: &stac::Item) -> Option<String> {
    item.properties
        .datetime
        .map(|dt| dt.to_rfc3339())
        .or_else(|| {
            item.properties
                .additional_fields
                .get("datetime")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
}

fn extract_cloud_cover(item: &stac::Item) -> Option<f64> {
    item.properties
        .additional_fields
        .get("eo:cloud_cover")
        .and_then(serde_json::Value::as_f64)
}

fn find_asset_by_role<'a, I>(assets: I, role: &str) -> Option<(&'a String, &'a stac::Asset)>
where
    I: IntoIterator<Item = (&'a String, &'a stac::Asset)> + Clone,
{
    assets
        .clone()
        .into_iter()
        .find(|(_, a)| a.roles.iter().any(|r| r == role))
        .or_else(|| assets.into_iter().find(|(k, _)| k.as_str() == role))
}

fn find_asset_by_cog_mime<'a, I>(assets: I) -> Option<(&'a String, &'a stac::Asset)>
where
    I: IntoIterator<Item = (&'a String, &'a stac::Asset)>,
{
    assets
        .into_iter()
        .find(|(_, a)| a.r#type.as_deref().is_some_and(is_cog_mime_type))
}

/// Convert a [`stac::Bbox`] to the 2D WGS-84 footprint `[west, south, east, north]`.
///
/// Per STAC spec §7.9.3 a bbox is either 4 or 6 numbers; the 6-element
/// form is `[west, south, min_elev, east, north, max_elev]` and indices
/// 0, 1, 3, 4 carry the 2D footprint. `stac::Bbox` encodes this as a
/// two-variant enum, so we access corners via its accessors instead of
/// index arithmetic (which previously caused a 3D-bbox truncation bug
/// where `bbox[2]` was mistakenly treated as `east`).
fn bbox_to_2d(bbox: &stac::Bbox) -> [f64; 4] {
    [bbox.xmin(), bbox.ymin(), bbox.xmax(), bbox.ymax()]
}

/// Compute the merged bounding box across all assets.
#[must_use]
pub fn compute_merged_bounds(assets: &[StacAsset]) -> Option<[f64; 4]> {
    if assets.is_empty() {
        return None;
    }

    let mut merged = assets[0].bbox;
    for asset in &assets[1..] {
        merged[0] = merged[0].min(asset.bbox[0]);
        merged[1] = merged[1].min(asset.bbox[1]);
        merged[2] = merged[2].max(asset.bbox[2]);
        merged[3] = merged[3].max(asset.bbox[3]);
    }

    Some(merged)
}

/// Returns `true` if the MIME type indicates a COG or GeoTIFF file.
#[must_use]
pub fn is_cog_mime_type(mime: &str) -> bool {
    mime == COG_MIME_TYPE || mime.starts_with(GEOTIFF_MIME_TYPE)
}

/// Returns `true` if the URL uses an HTTP(S) scheme (likely a STAC API endpoint).
#[must_use]
pub fn is_stac_api_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Convert an XYZ tile to its WGS-84 bounding box `[west, south, east, north]`.
///
/// Uses the standard Web Mercator tile→lonlat conversion. The result is the
/// geographic extent covered by a single tile, suitable for STAC bbox search
/// which expects WGS-84.
#[must_use]
pub fn tile_to_wgs84_bbox(z: u8, x: u32, y: u32) -> [f64; 4] {
    let n = 2f64.powi(i32::from(z));
    let lon_west = f64::from(x) / n * 360.0 - 180.0;
    let lon_east = f64::from(x + 1) / n * 360.0 - 180.0;
    let lat_north = (std::f64::consts::PI * (1.0 - 2.0 * f64::from(y) / n))
        .sinh()
        .atan()
        .to_degrees();
    let lat_south = (std::f64::consts::PI * (1.0 - 2.0 * f64::from(y + 1) / n))
        .sinh()
        .atan()
        .to_degrees();
    [lon_west, lat_south, lon_east, lat_north]
}

/// Phase 2: search the STAC API for items intersecting a bbox.
///
/// Same wire format as [`discover_assets`] but injects `bbox` and drops
/// the `limit` if the caller wants unbounded results.
///
/// # Errors
///
/// Returns [`TileServerError::StacError`] on HTTP or JSON failure.
pub async fn discover_assets_by_bbox(
    api_url: &str,
    collection: &str,
    asset_role: &str,
    max_items: usize,
    bbox: [f64; 4],
) -> Result<Vec<StacAsset>> {
    let search_url = build_search_url(api_url);
    let client = reqwest::Client::builder()
        .user_agent("tileserver-rs/stac")
        .build()
        .map_err(|e| TileServerError::StacError(format!("failed to create HTTP client: {e}")))?;

    let search_body = serde_json::json!({
        "collections": [collection],
        "bbox": bbox,
        "limit": max_items
    });

    let response = client
        .post(&search_url)
        .json(&search_body)
        .send()
        .await
        .map_err(|e| TileServerError::StacError(format!("stac bbox search failed: {e}")))?;

    if !response.status().is_success() {
        return Err(TileServerError::StacError(format!(
            "stac api returned status {}",
            response.status()
        )));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| TileServerError::StacError(format!("failed to parse stac response: {e}")))?;

    extract_assets_from_item_collection(&body, asset_role)
}

/// Convert a STAC asset `href` into a path GDAL can range-read.
///
/// STAC hrefs are typically `https://…/TCI.tif`. GDAL's `Dataset::open`
/// blocks for the entire file over HTTPS; the `/vsicurl/` VSI prefix
/// switches it to HTTP range requests so only the COG header + requested
/// overview bands are fetched. Non-HTTP paths pass through unchanged.
#[must_use]
pub fn to_gdal_cog_path(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        format!("/vsicurl/{href}")
    } else if let Some(rest) = href.strip_prefix("s3://") {
        format!("/vsis3/{rest}")
    } else {
        href.to_string()
    }
}

/// Phase 2 single-asset rendering: construct a one-shot [`CogSource`]
/// pointed at `asset.href`, render the tile, and drop the source.
///
/// The `template` SourceConfig provides shared config (colormap, resampling,
/// etc.) so per-tile renders match the catalog-wide style.
async fn render_single_asset(
    asset: &StacAsset,
    template: &SourceConfig,
    z: u8,
    x: u32,
    y: u32,
) -> Result<Option<TileData>> {
    let cfg = SourceConfig {
        id: template.id.clone(),
        source_type: crate::config::SourceType::Cog,
        path: to_gdal_cog_path(&asset.href),
        name: template.name.clone(),
        attribution: template.attribution.clone(),
        description: template.description.clone(),
        resampling: template.resampling,
        layer_name: None,
        geometry_column: None,
        query: None,
        minzoom: template.minzoom,
        maxzoom: template.maxzoom,
        serve_as: None,
        #[cfg(feature = "raster")]
        colormap: template.colormap.clone(),
        options: None,
        collection: None,
        asset_role: template.asset_role.clone(),
        dynamic: false,
        max_items: template.max_items,
        stac_bbox: template.stac_bbox,
        pixel_selection: template.pixel_selection,
        tile_path_template: None,
        tms: false,
    };

    match CogSource::from_file(&cfg).await {
        Ok(src) => src.get_tile(z, x, y).await,
        Err(e) => {
            tracing::warn!(
                asset_id = %asset.id,
                href = %asset.href,
                error = %e,
                "failed to load STAC asset for single-tile render"
            );
            Ok(None)
        }
    }
}

/// Phase 3 mosaic: composite multiple COG renders into a single tile.
///
/// Priority semantics: the **first** asset returned by STAC is the
/// highest-priority contributor and must appear on top of the final
/// image (STAC `/search` results are typically ranked newest-first /
/// best-cloud-cover-first, and consumers expect to see those pixels).
///
/// Strategy:
/// 1. Render all candidates **concurrently** via [`futures::future::join_all`].
///    Each asset's fetch+render runs as an independent future, so a
///    5-asset mosaic on cold-cache tiles completes in roughly the
///    slowest asset's latency instead of the sum of all of them.
/// 2. Preserve input order in the result set: `join_all` returns results
///    in the same order as inputs, so index `i` in the output maps back
///    to `assets[i]`'s render outcome.
/// 3. Start from the **lowest-priority** asset (the last one that
///    produced a usable layer) as the canvas.
/// 4. Overlay each higher-priority asset on top using straight-alpha
///    [`image::imageops::overlay`] so the top-priority asset's opaque
///    pixels win where it has coverage and lower-priority pixels fill
///    in the transparent gaps.
/// 5. Skip assets that fail to render or decode; a single bad COG does
///    not blank the tile.
///
/// # Errors
///
/// Returns [`TileServerError::StacError`] when the PNG re-encode fails
/// or when an inner `render_single_asset` bubbles an error up.  Per-
/// asset decode failures are *not* propagated — they are logged and
/// the asset is dropped from the composite.
async fn composite_mosaic(
    assets: &[StacAsset],
    template: &SourceConfig,
    z: u8,
    x: u32,
    y: u32,
) -> Result<Option<TileData>> {
    use crate::raster::{decode, encode, mosaic};

    let ordered_assets = order_assets_for_method(assets, template.pixel_selection);

    let render_futures = ordered_assets
        .iter()
        .map(|asset| render_single_asset(asset, template, z, x, y));
    let render_results = futures::future::join_all(render_futures).await;

    let mut method = mosaic::build(template.pixel_selection);
    let mut fed_any = false;
    for (asset, render_result) in ordered_assets.iter().zip(render_results) {
        let Some(tile) = render_result? else { continue };
        let raster = match decode::from_bytes(&tile.data) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(asset_id = %asset.id, error = %e, "mosaic: failed to decode");
                continue;
            }
        };
        method.feed(raster);
        fed_any = true;
        if method.is_done() {
            break;
        }
    }
    if !fed_any {
        return Ok(None);
    }
    let finalised = method.finalize();

    let png = encode::to_png(&finalised)
        .map_err(|e| TileServerError::StacError(format!("mosaic png encode failed: {e}")))?;

    Ok(Some(TileData {
        data: png.into(),
        format: TileFormat::Png,
        compression: crate::sources::TileCompression::None,
    }))
}

/// Reorder assets to give the chosen [`PixelSelectionMethod`] its
/// expected priority sequence.
///
/// - `LowestCloudCover`: sort by `cloud_cover` ascending, placing
///   assets with no metadata at the end (treated as worst case so an
///   explicitly-clear asset always wins over an unlabelled one).
/// - All other methods: preserve input order (STAC's own ranking).
fn order_assets_for_method(
    assets: &[StacAsset],
    method: crate::config::PixelSelectionMethod,
) -> Vec<StacAsset> {
    use crate::config::PixelSelectionMethod;
    let mut out = assets.to_vec();
    if method == PixelSelectionMethod::LowestCloudCover {
        out.sort_by(|a, b| {
            let ak = a.cloud_cover.unwrap_or(f64::INFINITY);
            let bk = b.cloud_cover.unwrap_or(f64::INFINITY);
            ak.partial_cmp(&bk).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item_json(item_id: &str, role: &str, mime: &str) -> serde_json::Value {
        serde_json::json!({
            "id": item_id,
            "type": "Feature",
            "bbox": [-122.5, 37.0, -122.0, 37.5],
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[-122.5, 37.0], [-122.0, 37.0], [-122.0, 37.5], [-122.5, 37.5], [-122.5, 37.0]]]
            },
            "properties": {
                "datetime": "2023-01-15T00:00:00Z"
            },
            "assets": {
                "visual": {
                    "href": "https://example.com/cog.tif",
                    "type": mime,
                    "title": "Visual RGB",
                    "roles": [role]
                },
                "thumbnail": {
                    "href": "https://example.com/thumb.png",
                    "type": "image/png",
                    "title": "Thumbnail",
                    "roles": ["thumbnail"]
                }
            }
        })
    }

    fn sample_item(item_id: &str, role: &str, mime: &str) -> stac::Item {
        serde_json::from_value(sample_item_json(item_id, role, mime)).expect("valid STAC Item")
    }

    fn sample_item_collection(items: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "type": "FeatureCollection",
            "features": items,
            "numberMatched": items.len(),
            "numberReturned": items.len()
        })
    }

    #[test]
    fn test_build_search_url_basic() {
        assert_eq!(
            build_search_url("https://earth-search.aws.element84.com/v1"),
            "https://earth-search.aws.element84.com/v1/search"
        );
    }

    #[test]
    fn test_build_search_url_strips_trailing_slash() {
        assert_eq!(
            build_search_url("https://example.com/stac/"),
            "https://example.com/stac/search"
        );
    }

    #[test]
    fn test_bbox_to_2d_from_2d() {
        let bbox = stac::Bbox::TwoDimensional([-122.5, 37.0, -122.0, 37.5]);
        assert_eq!(bbox_to_2d(&bbox), [-122.5, 37.0, -122.0, 37.5]);
    }

    #[test]
    fn test_bbox_to_2d_collapses_3d() {
        let bbox = stac::Bbox::ThreeDimensional([-122.5, 37.0, 0.0, -122.0, 37.5, 100.0]);
        assert_eq!(bbox_to_2d(&bbox), [-122.5, 37.0, -122.0, 37.5]);
    }

    #[test]
    fn test_stac_item_rejects_invalid_bbox_length() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "test",
            "stac_version": "1.0.0",
            "geometry": null,
            "bbox": [-122.5, 37.0, 0.0, -122.0, 37.5],
            "properties": {"datetime": null},
            "assets": {},
            "links": []
        });
        assert!(serde_json::from_value::<stac::Item>(item_json).is_err());
    }

    #[test]
    fn test_tile_to_wgs84_bbox_z0_covers_world() {
        let b = tile_to_wgs84_bbox(0, 0, 0);
        assert!((b[0] - -180.0).abs() < 1e-6);
        assert!((b[2] - 180.0).abs() < 1e-6);
        assert!((b[1] - -85.051_128).abs() < 1e-3);
        assert!((b[3] - 85.051_128).abs() < 1e-3);
    }

    #[test]
    fn test_tile_to_wgs84_bbox_z1_nw_quadrant() {
        let b = tile_to_wgs84_bbox(1, 0, 0);
        assert!((b[0] - -180.0).abs() < 1e-6);
        assert!((b[2] - 0.0).abs() < 1e-6);
        assert!(b[1] < b[3]);
    }

    #[test]
    fn test_stac_item_missing_bbox_is_optional() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "no-bbox",
            "stac_version": "1.0.0",
            "geometry": null,
            "properties": {"datetime": null},
            "assets": {},
            "links": []
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        assert!(item.bbox.is_none());
    }

    #[test]
    fn test_is_cog_mime_type_exact() {
        assert!(is_cog_mime_type(COG_MIME_TYPE));
    }

    #[test]
    fn test_is_cog_mime_type_geotiff() {
        assert!(is_cog_mime_type("image/tiff"));
    }

    #[test]
    fn test_is_cog_mime_type_geotiff_with_params() {
        assert!(is_cog_mime_type("image/tiff; application=geotiff"));
    }

    #[test]
    fn test_is_cog_mime_type_false() {
        assert!(!is_cog_mime_type("image/png"));
        assert!(!is_cog_mime_type("application/json"));
    }

    #[test]
    fn test_is_stac_api_url_https() {
        assert!(is_stac_api_url("https://earth-search.aws.element84.com/v1"));
    }

    #[test]
    fn test_is_stac_api_url_http() {
        assert!(is_stac_api_url("http://localhost:8080/stac"));
    }

    #[test]
    fn test_is_stac_api_url_false() {
        assert!(!is_stac_api_url("/local/path"));
        assert!(!is_stac_api_url("s3://bucket/path"));
    }

    #[test]
    fn test_find_asset_by_role_found() {
        let item = sample_item("s2-001", "visual", COG_MIME_TYPE);
        let (_, asset) = find_asset_by_role(item.assets.iter(), "visual").unwrap();
        assert_eq!(asset.href, "https://example.com/cog.tif");
    }

    #[test]
    fn test_find_asset_by_role_not_found() {
        let item = sample_item("s2-001", "visual", COG_MIME_TYPE);
        assert!(find_asset_by_role(item.assets.iter(), "data").is_none());
    }

    #[test]
    fn test_find_asset_by_role_fallback_to_key() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "test",
            "stac_version": "1.0.0",
            "geometry": null,
            "bbox": [0.0, 0.0, 1.0, 1.0],
            "properties": {"datetime": null},
            "links": [],
            "assets": {
                "visual": {
                    "href": "https://example.com/file.tif",
                    "type": COG_MIME_TYPE
                }
            }
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        assert!(find_asset_by_role(item.assets.iter(), "visual").is_some());
    }

    #[test]
    fn test_find_asset_by_cog_mime_found() {
        let item = sample_item("s2-001", "visual", COG_MIME_TYPE);
        assert!(find_asset_by_cog_mime(item.assets.iter()).is_some());
    }

    #[test]
    fn test_find_asset_by_cog_mime_geotiff() {
        let item = sample_item("s2-001", "visual", "image/tiff; application=geotiff");
        assert!(find_asset_by_cog_mime(item.assets.iter()).is_some());
    }

    #[test]
    fn test_find_asset_by_cog_mime_none() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "png-only",
            "stac_version": "1.0.0",
            "geometry": null,
            "bbox": [0.0, 0.0, 1.0, 1.0],
            "properties": {"datetime": null},
            "links": [],
            "assets": {
                "thumb": {
                    "href": "https://example.com/thumb.png",
                    "type": "image/png"
                }
            }
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        assert!(find_asset_by_cog_mime(item.assets.iter()).is_none());
    }

    #[test]
    fn test_extract_cog_asset_by_role() {
        let item = sample_item("s2-001", "visual", COG_MIME_TYPE);
        let asset = extract_cog_asset_from_item(&item, "visual").unwrap();
        assert_eq!(asset.id, "s2-001");
        assert_eq!(asset.href, "https://example.com/cog.tif");
        assert_eq!(asset.title, "Visual RGB");
        assert_eq!(asset.bbox, [-122.5, 37.0, -122.0, 37.5]);
    }

    #[test]
    fn test_extract_cog_asset_fallback_to_mime() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "landsat-001",
            "stac_version": "1.0.0",
            "geometry": null,
            "bbox": [10.0, 20.0, 11.0, 21.0],
            "properties": {"datetime": null},
            "links": [],
            "assets": {
                "B04": {
                    "href": "https://example.com/B04.tif",
                    "type": COG_MIME_TYPE,
                    "title": "Band 4",
                    "roles": ["data"]
                }
            }
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        let asset = extract_cog_asset_from_item(&item, "visual").unwrap();
        assert_eq!(asset.id, "landsat-001");
        assert_eq!(asset.href, "https://example.com/B04.tif");
    }

    #[test]
    fn test_extract_cog_asset_no_match() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "empty",
            "stac_version": "1.0.0",
            "geometry": null,
            "bbox": [0.0, 0.0, 1.0, 1.0],
            "properties": {"datetime": null},
            "links": [],
            "assets": {
                "thumb": {
                    "href": "https://example.com/thumb.png",
                    "type": "image/png",
                    "roles": ["thumbnail"]
                }
            }
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        assert!(extract_cog_asset_from_item(&item, "visual").is_none());
    }

    #[test]
    fn test_extract_cog_asset_missing_bbox() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "no-bbox",
            "stac_version": "1.0.0",
            "geometry": null,
            "properties": {"datetime": null},
            "links": [],
            "assets": {
                "visual": {
                    "href": "https://example.com/cog.tif",
                    "type": COG_MIME_TYPE,
                    "roles": ["visual"]
                }
            }
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        assert!(extract_cog_asset_from_item(&item, "visual").is_none());
    }

    #[test]
    fn test_extract_assets_from_item_collection() {
        let items = vec![
            sample_item_json("item-1", "visual", COG_MIME_TYPE),
            sample_item_json("item-2", "visual", COG_MIME_TYPE),
        ];
        let collection = sample_item_collection(items);
        let assets = extract_assets_from_item_collection(&collection, "visual").unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].id, "item-1");
        assert_eq!(assets[1].id, "item-2");
    }

    #[test]
    fn test_extract_assets_empty_collection() {
        let collection = sample_item_collection(vec![]);
        let assets = extract_assets_from_item_collection(&collection, "visual").unwrap();
        assert!(assets.is_empty());
    }

    #[test]
    fn test_extract_assets_missing_features_is_empty() {
        let body = serde_json::json!({"type": "FeatureCollection"});
        let assets = extract_assets_from_item_collection(&body, "visual").unwrap();
        assert!(assets.is_empty());
    }

    #[test]
    fn test_extract_assets_non_stac_payload_errors() {
        let body = serde_json::json!({"not": "a stac response"});
        let result = extract_assets_from_item_collection(&body, "visual");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_assets_mixed_items() {
        let items = vec![
            sample_item_json("has-cog", "visual", COG_MIME_TYPE),
            serde_json::json!({
                "id": "no-cog",
                "bbox": [0.0, 0.0, 1.0, 1.0],
                "assets": {
                    "thumb": {
                        "href": "https://example.com/thumb.png",
                        "type": "image/png",
                        "roles": ["thumbnail"]
                    }
                }
            }),
        ];
        let collection = sample_item_collection(items);
        let assets = extract_assets_from_item_collection(&collection, "visual").unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, "has-cog");
    }

    #[test]
    fn test_compute_merged_bounds_empty() {
        assert!(compute_merged_bounds(&[]).is_none());
    }

    #[test]
    fn test_compute_merged_bounds_single() {
        let assets = vec![StacAsset {
            id: "a".to_string(),
            href: "https://example.com/a.tif".to_string(),
            bbox: [10.0, 20.0, 11.0, 21.0],
            title: "A".to_string(),
            datetime: None,
            cloud_cover: None,
        }];
        let bounds = compute_merged_bounds(&assets).unwrap();
        assert_eq!(bounds, [10.0, 20.0, 11.0, 21.0]);
    }

    #[test]
    fn test_compute_merged_bounds_multiple() {
        let assets = vec![
            StacAsset {
                id: "a".to_string(),
                href: "https://example.com/a.tif".to_string(),
                bbox: [10.0, 20.0, 11.0, 21.0],
                title: "A".to_string(),
                datetime: None,
                cloud_cover: None,
            },
            StacAsset {
                id: "b".to_string(),
                href: "https://example.com/b.tif".to_string(),
                bbox: [9.0, 19.0, 12.0, 22.0],
                title: "B".to_string(),
                datetime: None,
                cloud_cover: None,
            },
        ];
        let bounds = compute_merged_bounds(&assets).unwrap();
        assert_eq!(bounds, [9.0, 19.0, 12.0, 22.0]);
    }

    #[test]
    fn test_compute_merged_bounds_non_overlapping() {
        let assets = vec![
            StacAsset {
                id: "west".to_string(),
                href: "https://example.com/w.tif".to_string(),
                bbox: [-10.0, 0.0, -5.0, 5.0],
                title: "West".to_string(),
                datetime: None,
                cloud_cover: None,
            },
            StacAsset {
                id: "east".to_string(),
                href: "https://example.com/e.tif".to_string(),
                bbox: [5.0, 0.0, 10.0, 5.0],
                title: "East".to_string(),
                datetime: None,
                cloud_cover: None,
            },
        ];
        let bounds = compute_merged_bounds(&assets).unwrap();
        assert_eq!(bounds, [-10.0, 0.0, 10.0, 5.0]);
    }

    #[test]
    fn test_stac_asset_serialization_roundtrip() {
        let asset = StacAsset {
            id: "test-id".to_string(),
            href: "https://example.com/cog.tif".to_string(),
            bbox: [-180.0, -90.0, 180.0, 90.0],
            title: "Test Asset".to_string(),
            datetime: None,
            cloud_cover: None,
        };
        let json = serde_json::to_string(&asset).unwrap();
        let deserialized: StacAsset = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.href, "https://example.com/cog.tif");
        assert_eq!(deserialized.bbox, [-180.0, -90.0, 180.0, 90.0]);
        assert_eq!(deserialized.title, "Test Asset");
    }

    #[test]
    fn test_stac_asset_deserialization() {
        let json = r#"{"id":"s2","href":"https://ex.com/s2.tif","bbox":[0,0,1,1],"title":"S2"}"#;
        let asset: StacAsset = serde_json::from_str(json).unwrap();
        assert_eq!(asset.id, "s2");
        assert_eq!(asset.bbox, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_extract_cog_asset_title_fallback_to_id() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "item-no-title",
            "stac_version": "1.0.0",
            "geometry": null,
            "bbox": [0.0, 0.0, 1.0, 1.0],
            "properties": {"datetime": null},
            "links": [],
            "assets": {
                "visual": {
                    "href": "https://example.com/cog.tif",
                    "type": COG_MIME_TYPE,
                    "roles": ["visual"]
                }
            }
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        let asset = extract_cog_asset_from_item(&item, "visual").unwrap();
        assert_eq!(asset.title, "item-no-title");
    }

    #[test]
    fn test_extract_cog_asset_multiple_roles() {
        let item_json = serde_json::json!({
            "type": "Feature",
            "id": "multi-role",
            "stac_version": "1.0.0",
            "geometry": null,
            "bbox": [0.0, 0.0, 1.0, 1.0],
            "properties": {"datetime": null},
            "links": [],
            "assets": {
                "B04": {
                    "href": "https://example.com/B04.tif",
                    "type": COG_MIME_TYPE,
                    "title": "Red Band",
                    "roles": ["data", "visual"]
                }
            }
        });
        let item: stac::Item = serde_json::from_value(item_json).unwrap();
        let asset = extract_cog_asset_from_item(&item, "visual").unwrap();
        assert_eq!(asset.id, "multi-role");
        assert_eq!(asset.title, "Red Band");
    }

    #[test]
    fn test_config_stac_source_type_serialization() {
        assert_eq!(
            serde_json::to_string(&crate::config::SourceType::Stac).unwrap(),
            "\"stac\""
        );
    }

    #[test]
    fn test_config_stac_source_type_deserialization() {
        let parsed: crate::config::SourceType = serde_json::from_str("\"stac\"").unwrap();
        assert_eq!(parsed, crate::config::SourceType::Stac);
    }

    #[test]
    fn test_config_stac_toml_parsing() {
        let toml = r#"
            [[sources]]
            id = "sentinel2"
            type = "stac"
            path = "https://earth-search.aws.element84.com/v1"
            name = "Sentinel-2 L2A"
            collection = "sentinel-2-l2a"
            asset_role = "visual"
            dynamic = false
            max_items = 50
        "#;

        let config: crate::config::Config = toml::from_str(toml).unwrap();
        assert_eq!(config.sources.len(), 1);
        let src = &config.sources[0];
        assert_eq!(src.id, "sentinel2");
        assert_eq!(src.source_type, crate::config::SourceType::Stac);
        assert_eq!(src.path, "https://earth-search.aws.element84.com/v1");
        assert_eq!(src.collection.as_deref(), Some("sentinel-2-l2a"));
        assert_eq!(src.asset_role, "visual");
        assert!(!src.dynamic);
        assert_eq!(src.max_items, 50);
    }

    #[test]
    fn test_config_stac_defaults() {
        let toml = r#"
            [[sources]]
            id = "test-stac"
            type = "stac"
            path = "https://stac-api.example.com"
            collection = "my-collection"
        "#;

        let config: crate::config::Config = toml::from_str(toml).unwrap();
        let src = &config.sources[0];
        assert_eq!(src.asset_role, "visual");
        assert!(!src.dynamic);
        assert_eq!(src.max_items, 100);
    }

    #[test]
    fn test_config_stac_alongside_pmtiles() {
        let toml = r#"
            [[sources]]
            id = "osm"
            type = "pmtiles"
            path = "/data/osm.pmtiles"

            [[sources]]
            id = "sentinel"
            type = "stac"
            path = "https://earth-search.aws.element84.com/v1"
            collection = "sentinel-2-l2a"
        "#;

        let config: crate::config::Config = toml::from_str(toml).unwrap();
        assert_eq!(config.sources.len(), 2);
        assert_eq!(
            config.sources[0].source_type,
            crate::config::SourceType::PMTiles
        );
        assert_eq!(
            config.sources[1].source_type,
            crate::config::SourceType::Stac
        );
    }

    #[test]
    fn test_config_multiple_stac_sources() {
        let toml = r#"
            [[sources]]
            id = "sentinel"
            type = "stac"
            path = "https://earth-search.aws.element84.com/v1"
            collection = "sentinel-2-l2a"

            [[sources]]
            id = "landsat"
            type = "stac"
            path = "https://landsatlook.usgs.gov/stac-server"
            collection = "landsat-c2-l2"
            asset_role = "data"
            max_items = 200
        "#;

        let config: crate::config::Config = toml::from_str(toml).unwrap();
        assert_eq!(config.sources.len(), 2);
        assert_eq!(
            config.sources[0].collection.as_deref(),
            Some("sentinel-2-l2a")
        );
        assert_eq!(
            config.sources[1].collection.as_deref(),
            Some("landsat-c2-l2")
        );
        assert_eq!(config.sources[1].asset_role, "data");
        assert_eq!(config.sources[1].max_items, 200);
    }

    #[test]
    fn test_stac_error_display() {
        let err = TileServerError::StacError("connection timeout".to_string());
        assert_eq!(err.to_string(), "stac error: connection timeout");
    }

    #[test]
    fn test_stac_error_status_code() {
        use axum::response::IntoResponse;
        let err = TileServerError::StacError("api failure".to_string());
        let response = err.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_to_gdal_cog_path_https() {
        let result = to_gdal_cog_path("https://example.com/cog.tif");
        assert_eq!(result, "/vsicurl/https://example.com/cog.tif");
    }

    #[test]
    fn test_to_gdal_cog_path_http() {
        let result = to_gdal_cog_path("http://example.com/cog.tif");
        assert_eq!(result, "/vsicurl/http://example.com/cog.tif");
    }

    #[test]
    fn test_to_gdal_cog_path_s3() {
        let result = to_gdal_cog_path("s3://bucket/path/cog.tif");
        assert_eq!(result, "/vsis3/bucket/path/cog.tif");
    }

    #[test]
    fn test_to_gdal_cog_path_local_passthrough() {
        let result = to_gdal_cog_path("/local/path/cog.tif");
        assert_eq!(result, "/local/path/cog.tif");
    }

    #[test]
    fn test_to_gdal_cog_path_relative_passthrough() {
        let result = to_gdal_cog_path("relative/cog.tif");
        assert_eq!(result, "relative/cog.tif");
    }

    fn make_asset_with_cloud_cover(id: &str, cloud_cover: Option<f64>) -> StacAsset {
        StacAsset {
            id: id.to_string(),
            href: format!("https://example.com/{}.tif", id),
            bbox: [-1.0, -1.0, 1.0, 1.0],
            title: format!("title {}", id),
            datetime: None,
            cloud_cover,
        }
    }

    #[test]
    fn test_order_assets_for_method_first_preserves_order() {
        let assets = vec![
            make_asset_with_cloud_cover("a", Some(50.0)),
            make_asset_with_cloud_cover("b", Some(10.0)),
            make_asset_with_cloud_cover("c", Some(30.0)),
        ];
        let ordered = order_assets_for_method(&assets, crate::config::PixelSelectionMethod::First);
        assert_eq!(ordered[0].id, "a");
        assert_eq!(ordered[1].id, "b");
        assert_eq!(ordered[2].id, "c");
    }

    #[test]
    fn test_order_assets_for_method_lowest_cloud_cover_sorts() {
        let assets = vec![
            make_asset_with_cloud_cover("a", Some(50.0)),
            make_asset_with_cloud_cover("b", Some(10.0)),
            make_asset_with_cloud_cover("c", Some(30.0)),
        ];
        let ordered = order_assets_for_method(
            &assets,
            crate::config::PixelSelectionMethod::LowestCloudCover,
        );
        assert_eq!(ordered[0].id, "b");
        assert_eq!(ordered[1].id, "c");
        assert_eq!(ordered[2].id, "a");
    }

    #[test]
    fn test_order_assets_for_method_lowest_cloud_cover_missing_treated_as_worst() {
        let assets = vec![
            make_asset_with_cloud_cover("a", None),
            make_asset_with_cloud_cover("b", Some(20.0)),
            make_asset_with_cloud_cover("c", Some(5.0)),
        ];
        let ordered = order_assets_for_method(
            &assets,
            crate::config::PixelSelectionMethod::LowestCloudCover,
        );
        assert_eq!(ordered[0].id, "c");
        assert_eq!(ordered[1].id, "b");
        assert_eq!(ordered[2].id, "a");
    }

    #[test]
    fn test_extract_datetime_present() {
        let item = sample_item("test", "data", "image/tiff");
        let dt = extract_datetime(&item).expect("datetime should be present");
        assert!(dt.starts_with("2023-01-15T00:00:00"), "got {}", dt);
    }

    #[test]
    fn test_extract_datetime_missing() {
        let json = serde_json::json!({
            "id": "no-dt",
            "type": "Feature",
            "stac_version": "1.0.0",
            "bbox": [-1.0, -1.0, 1.0, 1.0],
            "geometry": null,
            "properties": {"datetime": null},
            "assets": {}
        });
        let item: stac::Item = serde_json::from_value(json).unwrap();
        assert!(extract_datetime(&item).is_none());
    }

    #[test]
    fn test_extract_cloud_cover_present() {
        let json = serde_json::json!({
            "id": "with-cc",
            "type": "Feature",
            "stac_version": "1.0.0",
            "bbox": [-1.0, -1.0, 1.0, 1.0],
            "geometry": null,
            "properties": {"datetime": null, "eo:cloud_cover": 12.5},
            "assets": {}
        });
        let item: stac::Item = serde_json::from_value(json).unwrap();
        assert_eq!(extract_cloud_cover(&item), Some(12.5));
    }

    #[test]
    fn test_extract_cloud_cover_missing() {
        let json = serde_json::json!({
            "id": "no-cc",
            "type": "Feature",
            "stac_version": "1.0.0",
            "bbox": [-1.0, -1.0, 1.0, 1.0],
            "geometry": null,
            "properties": {"datetime": null},
            "assets": {}
        });
        let item: stac::Item = serde_json::from_value(json).unwrap();
        assert!(extract_cloud_cover(&item).is_none());
    }

    #[test]
    fn test_extract_cloud_cover_non_numeric_returns_none() {
        let json = serde_json::json!({
            "id": "string-cc",
            "type": "Feature",
            "stac_version": "1.0.0",
            "bbox": [-1.0, -1.0, 1.0, 1.0],
            "geometry": null,
            "properties": {"datetime": null, "eo:cloud_cover": "not-a-number"},
            "assets": {}
        });
        let item: stac::Item = serde_json::from_value(json).unwrap();
        assert!(extract_cloud_cover(&item).is_none());
    }

    #[test]
    fn test_tile_to_wgs84_bbox_z0() {
        let bbox = tile_to_wgs84_bbox(0, 0, 0);
        assert!((bbox[0] - (-180.0)).abs() < 0.01);
        assert!((bbox[2] - 180.0).abs() < 0.01);
        assert!(bbox[1] < -85.0 && bbox[1] > -86.0);
        assert!(bbox[3] > 85.0 && bbox[3] < 86.0);
    }

    #[test]
    fn test_tile_to_wgs84_bbox_z2_corner() {
        let bbox = tile_to_wgs84_bbox(2, 0, 0);
        assert!((bbox[0] - (-180.0)).abs() < 0.01);
        assert!((bbox[2] - (-90.0)).abs() < 0.01);
        assert!(bbox[3] > 0.0);
    }

    #[test]
    fn test_extract_assets_from_item_collection_basic() {
        let item1 = sample_item_json(
            "i1",
            "data",
            "image/tiff; application=geotiff; profile=cloud-optimized",
        );
        let collection = sample_item_collection(vec![item1]);
        let assets = extract_assets_from_item_collection(&collection, "data").unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, "i1");
    }

    #[test]
    fn test_extract_assets_from_item_collection_filters_non_cog() {
        let item1 = sample_item_json("i1", "thumbnail", "image/png");
        let collection = sample_item_collection(vec![item1]);
        let result = extract_assets_from_item_collection(&collection, "data");
        assert!(result.is_ok(), "result: {:?}", result);
    }

    #[test]
    fn test_extract_assets_from_item_collection_empty_features() {
        let collection = sample_item_collection(vec![]);
        let assets = extract_assets_from_item_collection(&collection, "data").unwrap();
        assert!(assets.is_empty());
    }
}
