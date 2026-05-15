//! Integration tests for OGC API Features endpoints (`routes/ogc.rs`).
//!
//! These tests exercise the public HTTP surface of the OGC router through the
//! same `api_router` used by `main`. They run against an *empty* `AppState`
//! (no sources, no styles), so they cover:
//!
//! - The always-available endpoints: landing page (`GET /ogc`), conformance
//!   (`GET /ogc/conformance`), and the empty `/ogc/collections` listing.
//! - Every "collection not found" error branch on the per-collection
//!   handlers (`collection`, `items`, `feature`, `queryables`, `sortables`,
//!   `schema`) plus the write methods (`POST`/`PUT`/`PATCH`/`DELETE`).
//! - Query-parameter deserialization edge cases that fire *before* the source
//!   lookup (malformed `limit`/`offset` → 400 from the `Query` extractor).
//!
//! Exercising the success paths for `items`/`feature`/CQL filters requires a
//! live `PostgresTableSource`, which the shared test harness does not provide;
//! those paths live behind the `postgres-integration` cfg and are covered by
//! `tests/postgres_sources.rs` and the inline unit tests in `routes/ogc.rs`.

#![cfg(feature = "postgres")]

mod common;

use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Landing page (`GET /ogc`)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn landing_page_returns_200() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn landing_page_has_title_and_description() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc").await.json();
    assert_eq!(body["title"], "tileserver-rs OGC API");
    assert!(
        body["description"].is_string(),
        "landing page must carry a description string"
    );
}

#[tokio::test]
async fn landing_page_has_five_links() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc").await.json();
    let links = body["links"]
        .as_array()
        .expect("landing page must include a links array");
    assert_eq!(
        links.len(),
        5,
        "landing page must advertise self/conformance/data/service-desc/service-doc"
    );
}

#[tokio::test]
async fn landing_page_advertises_all_required_relations() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc").await.json();
    let links = body["links"].as_array().expect("links array");
    for rel in ["self", "conformance", "data", "service-desc", "service-doc"] {
        assert!(
            links.iter().any(|l| l["rel"] == rel),
            "landing page must include rel={rel}"
        );
    }
}

#[tokio::test]
async fn landing_page_self_link_is_ogc_root() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc").await.json();
    let self_link = body["links"]
        .as_array()
        .expect("links")
        .iter()
        .find(|l| l["rel"] == "self")
        .expect("self link present");
    let href = self_link["href"].as_str().expect("self.href is a string");
    assert!(
        href.ends_with("/ogc"),
        "self link must point at the OGC root, got {href}"
    );
    assert_eq!(self_link["type"], "application/json");
}

#[tokio::test]
async fn landing_page_conformance_link_carries_ogc_prefix() {
    // Regression: clients must not be sent to `/conformance` (404) — the
    // OGC router is mounted under `/ogc`.
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc").await.json();
    let conformance = body["links"]
        .as_array()
        .expect("links")
        .iter()
        .find(|l| l["rel"] == "conformance")
        .expect("conformance link present");
    let href = conformance["href"].as_str().expect("conformance.href");
    assert!(
        href.ends_with("/ogc/conformance"),
        "conformance link must include /ogc prefix, got {href}"
    );
}

#[tokio::test]
async fn landing_page_service_desc_points_at_openapi_json() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc").await.json();
    let link = body["links"]
        .as_array()
        .expect("links")
        .iter()
        .find(|l| l["rel"] == "service-desc")
        .expect("service-desc link present");
    let href = link["href"].as_str().expect("service-desc.href");
    assert!(
        href.ends_with("/openapi.json"),
        "service-desc must point at /openapi.json, got {href}"
    );
    assert!(
        link["type"]
            .as_str()
            .is_some_and(|t| t.contains("openapi+json")),
        "service-desc media type must advertise OpenAPI"
    );
}

#[tokio::test]
async fn landing_page_service_doc_is_html() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc").await.json();
    let link = body["links"]
        .as_array()
        .expect("links")
        .iter()
        .find(|l| l["rel"] == "service-doc")
        .expect("service-doc link present");
    assert_eq!(link["type"], "text/html");
    assert!(
        link["href"]
            .as_str()
            .is_some_and(|h| h.ends_with("/_openapi")),
        "service-doc must point at /_openapi"
    );
}

// ---------------------------------------------------------------------------
// Conformance declaration (`GET /ogc/conformance`)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn conformance_returns_200() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/conformance").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn conformance_is_stable_snapshot() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc/conformance").await.json();
    // `/conformance` carries no dynamic data (no timestamps, no hostnames,
    // no source ids), so it is the only OGC endpoint safe to snapshot here.
    insta::assert_json_snapshot!("ogc_conformance", body);
}

#[tokio::test]
async fn conformance_includes_core_class() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc/conformance").await.json();
    let classes = body["conformsTo"].as_array().expect("conformsTo array");
    assert!(
        classes.iter().any(|c| c
            .as_str()
            .is_some_and(|s| s.contains("features-1/1.0/conf/core"))),
        "conformance declaration must include Part 1 Core class"
    );
}

#[tokio::test]
async fn conformance_includes_part2_crs_class() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc/conformance").await.json();
    let classes = body["conformsTo"].as_array().expect("conformsTo");
    assert!(
        classes.iter().any(|c| c
            .as_str()
            .is_some_and(|s| s.contains("features-2/1.0/conf/crs"))),
        "conformance declaration must advertise Part 2 CRS class"
    );
}

#[tokio::test]
async fn conformance_includes_part3_filter_class() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc/conformance").await.json();
    let classes = body["conformsTo"].as_array().expect("conformsTo");
    assert!(
        classes
            .iter()
            .any(|c| c.as_str().is_some_and(|s| s.contains("/filter"))),
        "conformance declaration must advertise Part 3 Filter class"
    );
}

// ---------------------------------------------------------------------------
// Collections listing (`GET /ogc/collections`)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collections_returns_200_on_empty_state() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/collections").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn collections_returns_empty_array_when_no_postgres_sources() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc/collections").await.json();
    let cols = body["collections"]
        .as_array()
        .expect("collections must be an array");
    assert!(cols.is_empty(), "expected empty list on empty state");
}

#[tokio::test]
async fn collections_response_includes_self_link() {
    let server = common::empty_test_server();
    let body: Value = server.get("/ogc/collections").await.json();
    let links = body["links"].as_array().expect("links array");
    let self_link = links
        .iter()
        .find(|l| l["rel"] == "self")
        .expect("self link present");
    assert_eq!(self_link["type"], "application/json");
    assert!(
        self_link["href"]
            .as_str()
            .is_some_and(|h| h.ends_with("/ogc/collections")),
        "self link must point at /ogc/collections"
    );
}

// ---------------------------------------------------------------------------
// Per-collection error paths (`GET /ogc/collections/{id}` and children)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collection_unknown_id_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/collections/does-not-exist").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn items_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/collections/does-not-exist/items").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn items_unknown_collection_with_bbox_still_returns_404() {
    // The Query extractor parses `bbox` as an Option<String>, so an unknown
    // collection still resolves to the SourceNotFound branch.
    let server = common::empty_test_server();
    let resp = server
        .get("/ogc/collections/missing/items?bbox=-180,-90,180,90")
        .await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn items_unknown_collection_with_filter_still_returns_404() {
    let server = common::empty_test_server();
    let resp = server
        .get("/ogc/collections/missing/items?filter=name%3D%27foo%27")
        .await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn items_rejects_non_numeric_limit() {
    // `limit: i64` — the `Query` extractor must fail before reaching the
    // handler, surfacing 4xx rather than 404 or 500.
    let server = common::empty_test_server();
    let resp = server
        .get("/ogc/collections/anything/items?limit=not-a-number")
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "malformed limit must surface a 4xx, got {status}"
    );
}

#[tokio::test]
async fn items_rejects_non_numeric_offset() {
    let server = common::empty_test_server();
    let resp = server
        .get("/ogc/collections/anything/items?offset=banana")
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "malformed offset must surface a 4xx, got {status}"
    );
}

#[tokio::test]
async fn feature_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/collections/missing/items/42").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn queryables_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/collections/missing/queryables").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn sortables_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/collections/missing/sortables").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn schema_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/ogc/collections/missing/schema").await;
    resp.assert_status_not_found();
}

// ---------------------------------------------------------------------------
// Write methods reject missing collections too
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_items_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let payload = json!({
        "type": "Feature",
        "geometry": { "type": "Point", "coordinates": [0.0, 0.0] },
        "properties": {}
    });
    let resp = server
        .post("/ogc/collections/missing/items")
        .json(&payload)
        .await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn put_feature_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let payload = json!({
        "type": "Feature",
        "geometry": { "type": "Point", "coordinates": [1.0, 2.0] },
        "properties": {}
    });
    let resp = server
        .put("/ogc/collections/missing/items/1")
        .json(&payload)
        .await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn patch_feature_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let payload = json!({ "properties": { "name": "updated" } });
    let resp = server
        .patch("/ogc/collections/missing/items/1")
        .json(&payload)
        .await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn delete_feature_unknown_collection_returns_404() {
    let server = common::empty_test_server();
    let resp = server.delete("/ogc/collections/missing/items/1").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn post_items_rejects_malformed_json_body() {
    // The `Json` extractor must reject non-JSON bodies before the handler
    // gets a chance to do the source lookup.
    let server = common::empty_test_server();
    let resp = server
        .post("/ogc/collections/missing/items")
        .text("this is not json")
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "malformed JSON body must surface a 4xx, got {status}"
    );
}
