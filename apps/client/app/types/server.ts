/**
 * PingResponse — mirrors the Rust admin.rs `PingResponse` struct exactly.
 *
 * Every field here MUST come from the `/ping` endpoint.
 * Never add fake metrics (p50, p99, region, QPS) — see HONEST-DATA RULE (CLAUDE.md Rule #20).
 */
export interface PingResponse {
  status: string;
  config_hash: string;
  loaded_at_unix: number;
  loaded_sources: number;
  loaded_styles: number;
  renderer_enabled: boolean;
  prometheus_listener_active: boolean;
  version: string;
  cache_enabled: boolean;
  cache_entries: number;
  cache_bytes: number;
}
