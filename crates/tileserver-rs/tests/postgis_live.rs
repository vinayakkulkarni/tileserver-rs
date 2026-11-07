//! End-to-end tests against a live PostGIS database.
//!
//! The database is the benchmark fixture: `benchmarks/postgres/init.sql`
//! (`benchmark_points`, `get_benchmark_tiles`, `get_filtered_tiles`) and
//! `benchmarks/postgres/init-ogc.sql` (`cities`, `countries`, `roads`).
//! `DATABASE_URL` points at it; the coverage workflow starts a PostGIS
//! service and loads both files.
//!
//! These tests FAIL when the database is unreachable. They only compile with
//! the `postgres-integration` feature, so a plain `cargo test` never needs a
//! database, and a run that asks for them cannot pass by skipping.

#![cfg(feature = "postgres-integration")]

mod common;

use serde_json::{Value, json};
use tileserver_rs::config::PostgresConfig;
use tileserver_rs::sources::SourceManager;

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://tileserver:tileserver@localhost:5432/tiles".to_string())
}

fn postgres_config() -> PostgresConfig {
    toml::from_str(&format!(
        r#"
connection_string = "{url}"
pool_size = 4
pool_pre_warm = true

[[tables]]
id = "cities"
table = "cities"
geometry_column = "geom"
id_column = "id"
srid = 4326
writable = true

[[tables]]
id = "roads"
table = "roads"
geometry_column = "geom"
id_column = "id"
srid = 4326
filter = {{ expression = "false" }}

[[functions]]
id = "points"
function = "get_benchmark_tiles"
minzoom = 0
maxzoom = 14

[[functions]]
id = "filtered"
function = "get_filtered_tiles"

# Does not exist: logged and skipped, the other sources still load.
[[functions]]
id = "missing"
function = "no_such_function"

[[tables]]
id = "missing_table"
table = "no_such_table"
"#,
        url = database_url()
    ))
    .expect("valid postgres config")
}

/// An out-db raster function that returns the repo's RGB test COG, plus the
/// `[[outdb_rasters]]` entry that serves it.
#[cfg(feature = "raster")]
async fn with_outdb_source(mut config: PostgresConfig) -> PostgresConfig {
    let cog = format!(
        "{}/../../data/raster/test-rgb.cog.tif",
        env!("CARGO_MANIFEST_DIR")
    );
    // Tests run in parallel; concurrent CREATE OR REPLACE of one function
    // races in pg_proc, so create it once per process.
    static CREATED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    CREATED.get_or_init(|| create_outdb_function(cog)).await;
    config.outdb_rasters = toml::from_str::<PostgresConfig>(&format!(
        r#"
connection_string = "{url}"
[[outdb_rasters]]
id = "bay_rgb"
function = "tileserver_test_outdb"
minzoom = 0
maxzoom = 18
"#,
        url = database_url()
    ))
    .expect("valid out-db config")
    .outdb_rasters;
    config
}

#[cfg(feature = "raster")]
async fn create_outdb_function(cog: String) {
    let (client, connection) = tokio_postgres::connect(&database_url(), tokio_postgres::NoTls)
        .await
        .expect("PostGIS must be reachable: set DATABASE_URL");
    tokio::spawn(connection);
    client
        .batch_execute(&format!(
            "CREATE OR REPLACE FUNCTION tileserver_test_outdb(
                z integer, x integer, y integer, bounds text, params jsonb DEFAULT '{{}}'::jsonb)
             RETURNS TABLE(filepath text, rescale_min float8, rescale_max float8) AS $$
               SELECT '{cog}'::text, NULL::float8, NULL::float8
               WHERE coalesce(params->>'empty', '') <> 'true'
             $$ LANGUAGE sql STABLE;"
        ))
        .await
        .expect("create the out-db test function");
}

async fn server() -> axum_test::TestServer {
    let config = postgres_config();
    #[cfg(feature = "raster")]
    let config = with_outdb_source(config).await;
    let manager = SourceManager::from_configs_with_postgres(&[], Some(&config))
        .await
        .expect("PostGIS must be reachable: set DATABASE_URL");
    let mut state = common::minimal_app_state();
    state.sources = std::sync::Arc::new(manager);
    common::server_from_state(state)
}

#[tokio::test]
async fn loads_every_table_and_function_source() {
    let server = server().await;
    let body: Value = server.get("/data.json").await.json();
    let ids: Vec<&str> = body
        .as_array()
        .expect("source list")
        .iter()
        .filter_map(|s| s["id"].as_str())
        .collect();
    for id in ["cities", "roads", "points", "filtered"] {
        assert!(ids.contains(&id), "{id} missing from {ids:?}");
    }
    for id in ["missing", "missing_table"] {
        assert!(!ids.contains(&id), "{id} must not load: {ids:?}");
    }
}

#[tokio::test]
async fn table_source_serves_a_vector_tile_with_features() {
    let server = server().await;
    // z0 covers the whole world, so every generated city is in it.
    let resp = server.get("/data/cities/0/0/0.pbf").await;
    resp.assert_status_ok();
    assert!(resp.as_bytes().len() > 100, "a z0 tile with 500 cities");
}

#[cfg(feature = "cel")]
#[tokio::test]
async fn table_cel_preset_filters_every_feature_out() {
    let server = server().await;
    let resp = server.get("/data/roads/0/0/0.pbf").await;
    assert_eq!(resp.status_code(), 204, "a preset that keeps nothing");
}

#[tokio::test]
async fn function_source_serves_tiles_with_and_without_query_params() {
    let server = server().await;
    let plain = server.get("/data/points/0/0/0.pbf").await;
    plain.assert_status_ok();
    assert!(!plain.as_bytes().is_empty());

    let filtered = server
        .get("/data/filtered/0/0/0.pbf")
        .add_query_param("category", "a")
        .await;
    assert!(
        filtered.status_code().is_success(),
        "status {}",
        filtered.status_code()
    );
}

#[tokio::test]
async fn ogc_items_pages_with_next_link_and_number_matched() {
    let server = server().await;
    let body: Value = server
        .get("/ogc/collections/cities/items")
        .add_query_param("limit", "10")
        .await
        .json();
    assert_eq!(body["type"], "FeatureCollection");
    assert_eq!(body["numberReturned"], 10);
    assert!(body["numberMatched"].as_i64().unwrap() > 10);
    let rels: Vec<&str> = body["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l["rel"].as_str())
        .collect();
    assert_eq!(rels, ["self", "next"]);
}

#[tokio::test]
async fn ogc_items_last_page_has_no_next_link() {
    let server = server().await;
    let all: Value = server.get("/ogc/collections/roads/items").await.json();
    let matched = all["numberMatched"].as_i64().unwrap();
    let body: Value = server
        .get("/ogc/collections/roads/items")
        .add_query_param("limit", "5")
        .add_query_param("offset", (matched - 2).to_string())
        .await
        .json();
    assert_eq!(body["numberReturned"], 2);
    assert_eq!(body["links"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn ogc_items_applies_bbox_and_cql2_filter() {
    let server = server().await;
    let world: Value = server.get("/ogc/collections/cities/items").await.json();
    let boxed: Value = server
        .get("/ogc/collections/cities/items")
        .add_query_param("bbox", "-10,-10,10,10")
        .await
        .json();
    assert!(boxed["numberMatched"].as_i64() < world["numberMatched"].as_i64());

    let filtered: Value = server
        .get("/ogc/collections/cities/items")
        .add_query_param("filter", "population > 0")
        .await
        .json();
    assert!(filtered["numberMatched"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn ogc_items_reprojects_to_the_requested_crs() {
    let server = server().await;
    let resp = server
        .get("/ogc/collections/cities/items")
        .add_query_param("limit", "1")
        .add_query_param("crs", "http://www.opengis.net/def/crs/EPSG/0/3857")
        .await;
    resp.assert_status_ok();
    assert!(
        resp.header("content-crs")
            .to_str()
            .unwrap()
            .contains("3857")
    );
    let body: Value = resp.json();
    let x = body["features"][0]["geometry"]["coordinates"][0]
        .as_f64()
        .unwrap();
    // Web Mercator metres, not degrees.
    assert!(x.abs() <= 20_037_508.35);
}

#[tokio::test]
async fn ogc_items_rejects_a_bad_filter_with_400() {
    let server = server().await;
    let resp = server
        .get("/ogc/collections/cities/items")
        .add_query_param("filter", "no_such_column = 1")
        .await;
    resp.assert_status_bad_request();
}

/// A SQL function call in `filter` must never reach PostgreSQL: with no
/// operator allow-list, `pg_sleep(2) IS NULL` would hold the request ~2 s.
#[tokio::test]
async fn ogc_items_rejects_sql_functions_in_the_filter() {
    let server = server().await;
    let started = std::time::Instant::now();
    for filter in [
        "pg_sleep(2) IS NULL",
        "query_to_xml('select 1', true, true, '') like '%1%'",
    ] {
        server
            .get("/ogc/collections/cities/items")
            .add_query_param("filter", filter)
            .await
            .assert_status_bad_request();
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

#[tokio::test]
async fn ogc_queryables_and_schema_list_table_columns() {
    let server = server().await;
    let q: Value = server
        .get("/ogc/collections/cities/queryables")
        .await
        .json();
    assert!(q["properties"]["population"].is_object(), "{q}");
    let s: Value = server.get("/ogc/collections/cities/schema").await.json();
    assert!(
        s["properties"]["properties"]["properties"]["name"].is_object(),
        "{s}"
    );
}

#[tokio::test]
async fn ogc_feature_crud_round_trip() {
    let server = server().await;
    let created = server
        .post("/ogc/collections/cities/items")
        .json(&json!({
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [77.59, 12.97]},
            "properties": {"name": "Test City", "country": "IN", "population": 1}
        }))
        .await;
    assert_eq!(created.status_code(), 201, "{}", created.text());
    let location = created.header("location").to_str().unwrap().to_string();
    let fid = location.rsplit('/').next().unwrap().to_string();
    let path = format!("/ogc/collections/cities/items/{fid}");

    let got: Value = server.get(&path).await.json();
    assert_eq!(got["properties"]["name"], "Test City");

    server
        .patch(&path)
        .json(&json!({"properties": {"population": 2}}))
        .await
        .assert_status_success();
    let patched: Value = server.get(&path).await.json();
    assert_eq!(patched["properties"]["population"], 2);

    // RFC 7396: `null` clears the property.
    server
        .patch(&path)
        .json(&json!({"properties": {"population": null}}))
        .await
        .assert_status_success();
    let cleared: Value = server.get(&path).await.json();
    assert!(cleared["properties"]["population"].is_null(), "{cleared}");

    server
        .put(&path)
        .json(&json!({
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [0.0, 0.0]},
            "properties": {"name": "Replaced", "country": "XX", "population": 3}
        }))
        .await
        .assert_status_success();
    let replaced: Value = server.get(&path).await.json();
    assert_eq!(replaced["properties"]["name"], "Replaced");

    server.delete(&path).await.assert_status_success();
    server.get(&path).await.assert_status_not_found();
}

#[tokio::test]
async fn ogc_write_to_a_read_only_table_is_rejected() {
    let server = server().await;
    let resp = server
        .post("/ogc/collections/roads/items")
        .json(&json!({
            "type": "Feature",
            "geometry": {"type": "LineString", "coordinates": [[0.0, 0.0], [1.0, 1.0]]},
            "properties": {"name": "nope"}
        }))
        .await;
    assert!(
        resp.status_code().is_client_error(),
        "{}",
        resp.status_code()
    );
}

#[tokio::test]
async fn ogc_geojson_tile_export_returns_features() {
    let server = server().await;
    let resp = server.get("/data/cities/0/0/0.geojson").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(!body["features"].as_array().unwrap().is_empty());
}

/// z10 tile over the RGB test COG (San Francisco).
#[cfg(feature = "raster")]
#[tokio::test]
async fn outdb_raster_source_renders_png_and_honours_query_params() {
    let server = server().await;
    let resp = server.get("/data/bay_rgb/10/163/395.png").await;
    resp.assert_status_ok();
    assert!(resp.as_bytes().starts_with(&[0x89, b'P', b'N', b'G']));

    // The function returns no rows for `empty=true`: nothing to render.
    let empty = server
        .get("/data/bay_rgb/10/163/395.png")
        .add_query_param("empty", "true")
        .await;
    assert!(
        matches!(empty.status_code().as_u16(), 204 | 404),
        "status {}",
        empty.status_code()
    );

    // Outside zoom range and out of the tile grid.
    server
        .get("/data/bay_rgb/10/5000/395.png")
        .await
        .assert_status_bad_request();
}
