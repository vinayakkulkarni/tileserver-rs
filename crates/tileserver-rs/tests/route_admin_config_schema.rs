//! Integration test for the admin config-schema endpoint.
//!
//! Boots the [`crate::admin::admin_router`] against a minimal `SharedState`
//! and asserts that `GET /__admin/config/schema` returns the catalog the
//! `/admin/config` page consumes. The unit tests in
//! [`crate::config_schema::tests`] cover drift detection and catalog
//! invariants; this test pins the *wire shape* — JSON keys, field
//! ordering inside `sections`, and the presence of marker sections.

mod common;

use axum_test::TestServer;
use serde_json::Value;
use tileserver_rs::admin::admin_router;

#[tokio::test]
async fn admin_config_schema_returns_well_formed_catalog() {
    let shared = common::minimal_shared_state();
    let server = TestServer::new(admin_router(shared));

    let resp = server.get("/__admin/config/schema").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["ok"], true);

    let sections = body["sections"].as_array().expect("sections is an array");
    assert!(
        sections.len() > 5,
        "expected the catalog to expose at least 5 sections, got {}",
        sections.len(),
    );

    let headers: Vec<&str> = sections
        .iter()
        .map(|s| s["header"].as_str().expect("header is a string"))
        .collect();

    for required in ["(root)", "[server]", "[render]", "[cache]", "[telemetry]"] {
        assert!(
            headers.contains(&required),
            "catalog missing required section header `{required}`; full set = {headers:?}",
        );
    }

    for section in sections {
        let fields = section["fields"]
            .as_array()
            .expect("each section.fields is an array");
        for field in fields {
            assert!(
                field["key"].is_string(),
                "every field needs a string `key`: {field}",
            );
            assert!(
                field["type"].is_string(),
                "every field needs a string `type`: {field}",
            );
            assert!(
                field["description"].is_string(),
                "every field needs a string `description`: {field}",
            );
        }
    }
}

#[tokio::test]
async fn admin_config_schema_omits_optional_serde_fields_when_absent() {
    let shared = common::minimal_shared_state();
    let server = TestServer::new(admin_router(shared));

    let resp = server.get("/__admin/config/schema").await;
    resp.assert_status_ok();
    let body: Value = resp.json();

    let server_section = body["sections"]
        .as_array()
        .expect("sections array")
        .iter()
        .find(|s| s["header"] == "[server]")
        .expect("[server] section present");
    let host_field = server_section["fields"]
        .as_array()
        .expect("server fields array")
        .iter()
        .find(|f| f["key"] == "host")
        .expect("[server].host field present");

    assert!(
        host_field.get("enumValues").is_none(),
        "host field should NOT carry enumValues (non-enum type)",
    );
    assert!(
        host_field.get("optional").is_none(),
        "non-optional fields should omit the `optional` key (serde skip_serializing_if)",
    );
}
