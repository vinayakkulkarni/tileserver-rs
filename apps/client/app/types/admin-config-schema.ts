/**
 * Schema-catalog types for the `/admin/config` page.
 *
 * The schema catalog is now served by the backend at
 * `GET /__admin/config/schema` (source of truth:
 * `crates/tileserver-rs/src/config_schema.rs`). These TS types mirror the
 * JSON shape served by that endpoint, so adding a new TOML key is one
 * Rust file edit and the UI picks it up automatically without a
 * coordinated frontend bump.
 *
 * Distinct from `admin-config.ts`, which models the live
 * `GET /__admin/config` API response (the *loaded* config as TOML);
 * `ConfigSectionSchema[]` is the catalog of every key the server *could*
 * accept.
 */

export type ConfigFieldType =
  | 'bool'
  | 'string'
  | 'path'
  | 'u8'
  | 'u16'
  | 'u32'
  | 'u64'
  | 'usize'
  | 'f64'
  | 'string[]'
  | 'f64[4]'
  | 'enum'
  | 'table'
  | 'table[]';

export interface ConfigFieldSchema {
  key: string;
  type: ConfigFieldType;
  /** Verbatim TOML-formatted default (quoted strings, bracketed arrays). `null` when no compile-time default. */
  default: string | null;
  description: string;
  /** True when the field is omittable. Optional fields without a runtime default render as commented suggestions. */
  optional?: boolean;
  enumValues?: readonly string[];
}

export interface ConfigSectionSchema {
  /** TOML section header, e.g. `[server]`, `[[sources]]`, `[postgres.cache]`. */
  header: string;
  /** Short prose explaining what this section is for. Rendered as a kicker. */
  blurb: string;
  /** All leaf fields, in display order. */
  fields: readonly ConfigFieldSchema[];
  /** Cargo feature gate, if any. The viewer shows these inside a "FEATURE-GATED" badge. */
  featureGate?: string;
  /** True when the section is an array-of-tables (`[[…]]`). */
  isArray?: boolean;
}

/**
 * Wire shape of `GET /__admin/config/schema`. The backend serialises
 * `Option<&'static str>` field defaults as either the string or
 * `undefined` (omitted via `skip_serializing_if`); the TS type widens
 * `default` to `string | null` so the UI never has to deal with three
 * states for "no default".
 */
export interface ConfigSchemaPayload {
  ok: boolean;
  sections: readonly ConfigSectionSchema[];
}

/**
 * One line of the annotated config view. The page renders each kind
 * with a different style so users see at a glance what is loaded versus
 * available:
 *   - `blank`           → vertical spacer between sections
 *   - `section-header`  → uppercase mono kicker with optional feature-gate badge
 *   - `set`             → primary-color line for keys present in the loaded config
 *   - `suggestion`      → muted comment line for unset optional keys
 *
 * Adding a new variant must be paired with a new render branch in
 * `pages/admin/config.vue`; otherwise the page silently drops the line.
 */
export type ConfigLine =
  | { kind: 'blank' }
  | {
      kind: 'section-header';
      header: string;
      blurb: string;
      featureGate?: string;
    }
  | {
      kind: 'set';
      key: string;
      rendered: string;
      schema: ConfigFieldSchema | null;
    }
  | { kind: 'suggestion'; key: string; schema: ConfigFieldSchema };

export interface ConfigSectionView {
  schema: ConfigSectionSchema;
  /** Lines rendered for the section: section header, then set keys, then suggestions. */
  lines: readonly ConfigLine[];
  /** True if the loaded config has at least one occurrence of this section. */
  isPresent: boolean;
  /** Number of occurrences (`[[…]]` arrays can have multiple instances). */
  occurrences: number;
}
