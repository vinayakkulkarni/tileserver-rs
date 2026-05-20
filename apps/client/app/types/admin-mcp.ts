/**
 * Admin MCP OAuth types.
 *
 * 1:1 mirror of the Rust response shapes in
 * `crates/tileserver-rs/src/mcp/admin_routes.rs`.
 * Both sides round-trip through serde, so any drift here will surface
 * as a runtime TypeScript narrowing failure in the admin pages.
 */

import type { Component } from 'vue';

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

/**
 * Single breadcrumb segment shown in admin page headers.
 *
 * `to` omitted → segment renders as muted, non-clickable text (used for
 * the current page). When present, must be a real navigable route.
 */
export interface AdminBreadcrumbCrumb {
  label: string;
  to?: string;
}

/**
 * Friendly error wrapper for admin endpoint failures. Never leaks raw
 * HTTP status codes or stack traces to the end user.
 */
export interface AdminFriendlyError {
  title: string;
  body: string;
  hint?: string;
}

/**
 * Sidebar navigation entry in the admin layout. `to` is a real route,
 * `icon` is a Lucide component imported by the layout consumer.
 */
export interface AdminNavItem {
  label: string;
  to: string;
  icon: Component;
}

/**
 * Sidebar navigation group. `heading` is the uppercase mono kicker shown
 * above the group; null for ungrouped entries (rendered without a header).
 */
export interface AdminNavGroup {
  heading: string | null;
  items: AdminNavItem[];
}
