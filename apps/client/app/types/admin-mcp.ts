/**
 * Admin MCP OAuth types.
 *
 * 1:1 mirror of the Rust response shapes in
 * `crates/tileserver-rs/src/mcp/admin_routes.rs`.
 * Both sides round-trip through serde, so any drift here will surface
 * as a runtime TypeScript narrowing failure in the admin pages.
 */

/**
 * Registered DCR client + derived session stats.
 *
 * Matches `AdminClient` (serde-renamed snake_case).
 */
export interface AdminMcpClient {
  client_id: string;
  client_name: string | null;
  redirect_uris: string[];
  active_sessions: number;
  scopes: string[];
  first_granted_at: number | null;
  last_seen_at: number | null;
}

/**
 * Single outstanding refresh token (device session).
 *
 * Matches `AdminSession`.
 */
export interface AdminMcpSession {
  token_id: string;
  client_id: string;
  client_name: string | null;
  scope: string;
  granted_at: number;
  expires_at: number;
}

/**
 * Response body for `DELETE /__admin/oauth/clients/{id}` and
 * `DELETE /__admin/oauth/sessions/{token}`.
 *
 * `revoked_sessions` is populated only on client deletes (cascade
 * count). Session deletes always set it to `null`.
 */
export interface AdminMcpDeleteResponse {
  ok: boolean;
  deleted: boolean;
  revoked_sessions: number | null;
}
