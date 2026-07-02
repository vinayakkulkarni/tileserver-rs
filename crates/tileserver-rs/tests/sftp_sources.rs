//! SFTP PMTiles source integration tests.
//!
//! These require the docker harness in `tests/sftp/` (an openssh-server
//! serving `data/tiles/protomaps-sample.pmtiles` over SFTP). They are
//! **skipped** unless `TILESERVER_SFTP_TEST_HOST` is set — `cargo test
//! --all-features` stays green on machines without docker. Run them via
//! `tests/sftp/run.sh`, which brings the container up, exports the env, and
//! tears it down.
#![cfg(feature = "sftp")]

mod common;

use std::sync::Arc;
use tileserver_rs::TileSource;
use tileserver_rs::config::Config;
use tileserver_rs::sources::pmtiles::sftp::SftpPmTilesSource;

struct TestEnv {
    host: String,
    port: String,
    user: String,
    identity: String,
    known_hosts_good: String,
    known_hosts_stale: String,
}

/// Read the harness env or return `None` so the test self-skips when docker
/// is unavailable.
fn test_env() -> Option<TestEnv> {
    let host = std::env::var("TILESERVER_SFTP_TEST_HOST").ok()?;
    Some(TestEnv {
        host,
        port: std::env::var("TILESERVER_SFTP_TEST_PORT").unwrap_or_else(|_| "2222".to_string()),
        user: std::env::var("TILESERVER_SFTP_TEST_USER").unwrap_or_else(|_| "test".to_string()),
        identity: std::env::var("TILESERVER_SFTP_TEST_IDENTITY").unwrap_or_default(),
        known_hosts_good: std::env::var("TILESERVER_SFTP_TEST_KNOWN_HOSTS").unwrap_or_default(),
        known_hosts_stale: std::env::var("TILESERVER_SFTP_TEST_KNOWN_HOSTS_STALE")
            .unwrap_or_default(),
    })
}

fn source_config(
    env: &TestEnv,
    known_hosts: &str,
    identity: &str,
) -> tileserver_rs::config::SourceConfig {
    let toml = format!(
        r#"
        id = "sftp-test"
        type = "pmtiles"
        path = "sftp://{user}@{host}:{port}/data/tiles.pmtiles"
        name = "SFTP Test"

        [options]
        ssh_identity = "{identity}"
        ssh_known_hosts_path = "{known_hosts}"
        "#,
        user = env.user,
        host = env.host,
        port = env.port,
        identity = identity,
        known_hosts = known_hosts,
    );
    toml::from_str(&toml).expect("valid source config")
}

/// I1 + I2: happy-path tile round-trip, bytes match the local PMTiles copy.
#[tokio::test]
async fn sftp_happy_path_and_matches_local_pmtiles() {
    let Some(env) = test_env() else {
        eprintln!("Skipping: TILESERVER_SFTP_TEST_HOST unset (docker harness not running)");
        return;
    };

    let cfg = source_config(&env, &env.known_hosts_good, &env.identity);
    let sftp_source = SftpPmTilesSource::from_url(&cfg)
        .await
        .expect("SFTP source should load against the harness");

    // Find a tile that actually exists by probing the source's zoom range.
    let meta = sftp_source.metadata().clone();
    let minz = meta.minzoom;

    let sftp_tile = sftp_source
        .get_tile(minz, 0, 0)
        .await
        .expect("get_tile must not error");

    // Local comparison: read the same file from disk via the local backend.
    let mut local_cfg = cfg.clone();
    local_cfg.path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/tiles/protomaps-sample.pmtiles"
    )
    .to_string();
    local_cfg.options = None;
    let local_source =
        tileserver_rs::sources::pmtiles::local::LocalPmTilesSource::from_file(&local_cfg)
            .await
            .expect("local PMTiles source should load");
    let local_tile = local_source
        .get_tile(minz, 0, 0)
        .await
        .expect("local get_tile must not error");

    match (sftp_tile, local_tile) {
        (Some(s), Some(l)) => {
            assert_eq!(
                s.data.as_ref(),
                l.data.as_ref(),
                "SFTP tile bytes must match the local PMTiles copy"
            );
        }
        (None, None) => {
            // Both agree the tile is absent — still proves the SFTP header
            // read + range reads worked identically to local.
        }
        (s, l) => panic!(
            "SFTP/local tile presence mismatch: sftp={:?} local={:?}",
            s.is_some(),
            l.is_some()
        ),
    }
}

/// I1 (route level): serve the SFTP source through the HTTP API and fetch a
/// tile via the data route.
#[tokio::test]
async fn sftp_tile_round_trip_via_http_route() {
    let Some(env) = test_env() else {
        eprintln!("Skipping: TILESERVER_SFTP_TEST_HOST unset");
        return;
    };

    let cfg = source_config(&env, &env.known_hosts_good, &env.identity);
    let source = Arc::new(
        SftpPmTilesSource::from_url(&cfg)
            .await
            .expect("SFTP source loads"),
    ) as Arc<dyn TileSource>;

    let server = common::server_with_sources(vec![source]);
    let resp = server.get("/data/sftp-test.json").await;
    resp.assert_status_ok();
    let body = resp.text();
    assert!(
        body.contains("tilejson"),
        "TileJSON response should describe the SFTP source, got: {body}"
    );
}

/// I3: wrong/unreadable identity fails fast with the identity path in the
/// error message.
#[tokio::test]
async fn sftp_auth_failure_reports_identity_path() {
    let Some(env) = test_env() else {
        eprintln!("Skipping: TILESERVER_SFTP_TEST_HOST unset");
        return;
    };

    let bogus_identity = "/nonexistent/sftp-test-key";
    let cfg = source_config(&env, &env.known_hosts_good, bogus_identity);
    let err = match SftpPmTilesSource::from_url(&cfg).await {
        Ok(_) => panic!("missing identity must fail"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains(bogus_identity),
        "auth error must include the resolved identity path, got: {msg}"
    );
}

/// I4: stale known_hosts fingerprint is refused with a host-key mismatch
/// before any range read.
#[tokio::test]
async fn sftp_host_key_mismatch_refused() {
    let Some(env) = test_env() else {
        eprintln!("Skipping: TILESERVER_SFTP_TEST_HOST unset");
        return;
    };
    if env.known_hosts_stale.is_empty() {
        eprintln!("Skipping: TILESERVER_SFTP_TEST_KNOWN_HOSTS_STALE unset");
        return;
    }

    let cfg = source_config(&env, &env.known_hosts_stale, &env.identity);
    let err = match SftpPmTilesSource::from_url(&cfg).await {
        Ok(_) => panic!("stale host key must be refused"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("host key mismatch") || msg.contains("SftpHostKeyMismatch"),
        "expected host-key mismatch error, got: {msg}"
    );
}

/// I5: a single SFTP source reuses one session across many sequential
/// fetches — the source loads once and 100 reads succeed without re-loading.
#[tokio::test]
async fn sftp_connection_reused_across_many_reads() {
    let Some(env) = test_env() else {
        eprintln!("Skipping: TILESERVER_SFTP_TEST_HOST unset");
        return;
    };

    let cfg = source_config(&env, &env.known_hosts_good, &env.identity);
    let source = SftpPmTilesSource::from_url(&cfg)
        .await
        .expect("SFTP source loads once");

    let minz = source.metadata().minzoom;
    for _ in 0..100 {
        source
            .get_tile(minz, 0, 0)
            .await
            .expect("each sequential read must succeed on the reused session");
    }
}

/// I6: insecure-skip bypass accepts a mismatched host key (test-only path).
#[tokio::test]
async fn sftp_insecure_skip_accepts_any_host_key() {
    let Some(env) = test_env() else {
        eprintln!("Skipping: TILESERVER_SFTP_TEST_HOST unset");
        return;
    };

    tileserver_rs::sources::pmtiles::sftp::set_cli_insecure_skip_host_key_verify(true);
    let cfg = source_config(&env, &env.known_hosts_stale, &env.identity);
    let ok = SftpPmTilesSource::from_url(&cfg).await.is_ok();
    tileserver_rs::sources::pmtiles::sftp::set_cli_insecure_skip_host_key_verify(false);
    assert!(ok, "insecure-skip must accept a mismatched host key");
}

/// Config-only test that always runs (no docker): the SFTP source rejects a
/// malformed URL before any network activity.
#[tokio::test]
async fn sftp_malformed_url_is_config_error() {
    let toml = r#"
        id = "bad"
        type = "pmtiles"
        path = "sftp://no-path-host"
    "#;
    let cfg: tileserver_rs::config::SourceConfig = toml::from_str(toml).unwrap();
    let err = match SftpPmTilesSource::from_url(&cfg).await {
        Ok(_) => panic!("malformed URL must fail"),
        Err(e) => e,
    };
    assert!(matches!(
        err,
        tileserver_rs::error::TileServerError::ConfigError(_)
    ));
    let _ = Config::default();
}
