/**
 * Response shape of `GET /__admin/config`.
 *
 * Mirrors `ConfigViewResponse` in `crates/tileserver-rs/src/admin.rs`.
 * Keep the two in sync when editing the Rust struct.
 */
export interface AdminConfigPayload {
  ok: boolean;
  /** The currently-loaded Config serialized as TOML via `toml::to_string_pretty`. */
  toml: string;
  /** Absolute path of the config file the server was started with, if any. */
  source_path: string | null;
  /** Hex content hash of the loaded config (matches `/ping`.config_hash). */
  config_hash: string;
  /** Unix epoch seconds when the server last reloaded. */
  loaded_at_unix: number;
}
