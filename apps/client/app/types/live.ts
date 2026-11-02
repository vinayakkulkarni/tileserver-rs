/**
 * Live source updates (`GET /ws`, `[live]` config).
 *
 * Mirrors `ServerMessage` / `ClientMessage` in
 * `crates/tileserver-rs/src/live.rs`.
 */

export type LiveServerMessage =
  | { type: 'hello'; sources: string[] }
  | { type: 'reload'; source: string };

export type LiveClientMessage =
  | { type: 'subscribe'; source: string }
  | { type: 'unsubscribe'; source: string };

/** The parts of a MapLibre style source that name its tile URLs. */
export interface LiveStyleSource {
  url?: string;
  tiles?: string[];
}
