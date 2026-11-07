//! OGC API Features — Part 3: Filtering (CQL2-text and CQL2-JSON).
//!
//! Parses either the `cql2-text` or `cql2-json` dialect into a PostgreSQL
//! `WHERE` fragment. The implementation wraps the `cql2` crate but adds
//! two defences its 0.5.3 output lacks:
//!
//! 1. **Property allow-list** — `cql2::ToSqlAst::to_sql()` emits bare
//!    property names (e.g. `name = 'Berlin'`) with no identifier quoting,
//!    which would let a malicious filter reference system tables or reserved
//!    words (`pg_catalog.pg_shadow`, `current_user`). We walk the AST and
//!    reject any `Expr::Property` whose name is not in the caller-provided
//!    whitelist of safe column names, returning 400 Bad Request instead of
//!    letting the query reach PostgreSQL.
//!
//! 2. **Operator allow-list** — `cql2` parses any `name(args)` as an
//!    operation and `to_sql()` emits an unknown name as a SQL function call,
//!    so `pg_sleep(10) = 1` or `query_to_xml('select ...', true, true, '')
//!    like '%x%'` would run arbitrary SQL. Only the operators the CQL2
//!    standard defines are accepted; we advertise no custom functions.
//!
//! 3. **Round-trip safety gate** — `cql2`'s grammar is lenient enough to
//!    silently drop trailing garbage (e.g. `name = 'x'; DROP TABLE t; --`
//!    parses as `name = 'x'`). We re-serialize the parsed expression back
//!    to CQL2-text and require it to round-trip, which rejects any input
//!    that loses characters during parsing.
//!
//! Reference: <https://docs.ogc.org/is/19-079r2/19-079r2.html>

use cql2::{Expr, ToSqlAst};

use crate::error::TileServerError;

/// Conformance URIs advertised when Part 3 is compiled in.
pub(crate) const CONFORMANCE_FILTER: &str =
    "http://www.opengis.net/spec/ogcapi-features-3/1.0/conf/filter";
pub(crate) const CONFORMANCE_FEATURES_FILTER: &str =
    "http://www.opengis.net/spec/ogcapi-features-3/1.0/conf/features-filter";
pub(crate) const CONFORMANCE_QUERYABLES: &str =
    "http://www.opengis.net/spec/ogcapi-features-3/1.0/conf/queryables";

/// OGC API Features Part 3 declares two filter languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterLang {
    CqlText,
    CqlJson,
}

impl FilterLang {
    fn parse(raw: &str) -> Result<Self, TileServerError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "cql2-text" | "cql-text" => Ok(Self::CqlText),
            "cql2-json" | "cql-json" => Ok(Self::CqlJson),
            _ => Err(TileServerError::InvalidTileRequest),
        }
    }
}

/// Operators of CQL2 itself, in `cql2`'s canonical spelling. Anything else
/// would reach PostgreSQL as a function call.
const STANDARD_OPS: &[&str] = &[
    "and", "or", "not", "=", "<>", "<", "<=", ">", ">=", "like", "between", "in", "isNull",
    "casei", "accenti", "+", "-", "*", "/", "%", "^", "div",
];

fn is_standard_op(op: &str) -> bool {
    STANDARD_OPS.contains(&op)
        || cql2::SPATIALOPS.contains(&op)
        || cql2::TEMPORALOPS.contains(&op)
        || cql2::ARRAYOPS.contains(&op)
}

/// Recursively walks the CQL2 AST and rejects any `Expr::Property` whose
/// name is not present in `allowed`, and any operation that is not a CQL2
/// operator.
///
/// This is the primary SQL-injection defence. Because `cql2::to_sql()`
/// emits bare identifiers (e.g. `pg_catalog.pg_shadow`), unrestricted
/// property names could reach arbitrary catalog objects. By pre-validating
/// against the column whitelist obtained from `PostgresTableSource`, the
/// filter output is guaranteed to only reference user-visible columns —
/// which in turn are safe to emit without quoting because the column
/// discovery path already constrained them to the `table_info.properties`
/// list (itself populated from `information_schema.columns`).
fn verify_properties(expr: &Expr, allowed: &[String]) -> Result<(), TileServerError> {
    match expr {
        Expr::Property { property } => {
            if allowed.iter().any(|a| a == property) {
                Ok(())
            } else {
                tracing::warn!(
                    property = %property,
                    "CQL2 filter referenced unknown property; rejecting to prevent SQL injection"
                );
                Err(TileServerError::InvalidTileRequest)
            }
        }
        Expr::Operation { op, args } => {
            if !is_standard_op(op) {
                tracing::warn!(
                    function = %op,
                    "CQL2 filter called a non-standard function; rejecting to prevent SQL injection"
                );
                return Err(TileServerError::InvalidTileRequest);
            }
            args.iter().try_for_each(|a| verify_properties(a, allowed))
        }
        Expr::Interval { interval } => interval
            .iter()
            .try_for_each(|a| verify_properties(a, allowed)),
        Expr::Timestamp { timestamp } => verify_properties(timestamp, allowed),
        Expr::Date { date } => verify_properties(date, allowed),
        Expr::BBox { bbox } => bbox.iter().try_for_each(|a| verify_properties(a, allowed)),
        Expr::Array(items) => items.iter().try_for_each(|a| verify_properties(a, allowed)),
        _ => Ok(()),
    }
}

/// Parses a CQL2 expression and returns a PostgreSQL-compatible SQL fragment
/// ready to splice into a `WHERE` clause.
///
/// `allowed_properties` is the whitelist of column names the filter may
/// reference (typically `table_info.properties`). Any other property name
/// triggers a 400.
///
/// `lang` picks the dialect; `None` auto-detects (leading `{` → JSON).
///
/// # Errors
///
/// Returns [`TileServerError::InvalidTileRequest`] when:
/// - The expression does not parse as CQL2.
/// - The re-serialisation (`to_text`) drops characters, indicating the
///   parser accepted a malformed input.
/// - The filter references a property not in `allowed_properties`.
/// - The AST cannot be translated to SQL.
pub(crate) fn translate_filter_to_sql(
    raw: &str,
    lang: Option<&str>,
    allowed_properties: &[String],
) -> Result<String, TileServerError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(TileServerError::InvalidTileRequest);
    }

    let resolved_lang = match lang {
        Some(s) => Some(FilterLang::parse(s)?),
        None => None,
    };

    let expr = match resolved_lang {
        Some(FilterLang::CqlText) => cql2::parse_text(trimmed).map_err(|e| {
            tracing::warn!(error = %e, filter = %trimmed, "CQL2-text parse failed");
            TileServerError::InvalidTileRequest
        })?,
        Some(FilterLang::CqlJson) => cql2::parse_json(trimmed).map_err(|e| {
            tracing::warn!(error = %e, filter = %trimmed, "CQL2-JSON parse failed");
            TileServerError::InvalidTileRequest
        })?,
        None => trimmed.parse::<Expr>().map_err(|e| {
            tracing::warn!(error = %e, filter = %trimmed, "CQL2 auto-detect parse failed");
            TileServerError::InvalidTileRequest
        })?,
    };

    reject_non_roundtrip(&expr, trimmed, resolved_lang)?;
    verify_properties(&expr, allowed_properties)?;

    expr.to_sql().map_err(|e| {
        tracing::warn!(error = %e, filter = %trimmed, "CQL2 -> SQL translation failed");
        TileServerError::InvalidTileRequest
    })
}

/// Safety gate: reject inputs where cql2 accepted less than we supplied.
///
/// For `cql2-text` input, we compare the parser's own normalised output
/// against the input after stripping whitespace. For JSON we only reject
/// empty text round-trip (the JSON schema validator inside cql2 already
/// blocks most hostile shapes).
fn reject_non_roundtrip(
    expr: &Expr,
    original: &str,
    lang: Option<FilterLang>,
) -> Result<(), TileServerError> {
    let treat_as_text = matches!(lang, Some(FilterLang::CqlText))
        || (lang.is_none() && !original.trim_start().starts_with('{'));
    if !treat_as_text {
        return Ok(());
    }

    let Ok(reserialised) = expr.to_text() else {
        return Err(TileServerError::InvalidTileRequest);
    };

    let canonical_original = canonicalise_cql_text(original);
    let canonical_reserialised = canonicalise_cql_text(&reserialised);

    if canonical_original != canonical_reserialised {
        tracing::warn!(
            original = %original,
            reserialised = %reserialised,
            "CQL2-text round-trip mismatch; rejecting possibly-malicious filter"
        );
        return Err(TileServerError::InvalidTileRequest);
    }
    Ok(())
}

/// Canonicalises CQL2-text for the round-trip comparison.
///
/// `cql2::to_text()` normalises the user input by wrapping groups in
/// parentheses, standardising casing, and inserting whitespace around
/// operators and after commas. To tell apart a legitimate difference
/// (whitespace, casing, redundant parens the user omitted) from a
/// malicious difference (dropped tokens), we strip everything the
/// normaliser is allowed to change:
///
/// - parentheses are removed (cql2 always parenthesises nested exprs)
/// - every whitespace run OUTSIDE a quoted string is removed entirely
/// - casing is lowered uniformly OUTSIDE quoted strings (string literals
///   preserve their original casing — `name = 'Paris'` must not canonicalise
///   to `name='paris'` or we'd accept `'PARIS'` as equivalent)
///
/// The remaining token stream must match byte-for-byte between the user's
/// input and the round-tripped output. Any SQL trailer
/// (`; DROP TABLE ...; --`) is detectable here because it contains
/// semicolons or `--` that cql2 cannot emit itself.
///
/// Whitespace stripping (not collapsing) is load-bearing: users write
/// `population>5000000` (no spaces), cql2 re-emits `(population > 5000000)`
/// (with spaces). A collapsing canonicaliser would see these as different
/// ("population>5000000" vs "population > 5000000") and reject the filter.
fn canonicalise_cql_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_string = false;
    let mut string_quote = '\0';
    for ch in s.chars() {
        if in_string {
            out.push(ch);
            if ch == string_quote {
                in_string = false;
            }
            continue;
        }
        match ch {
            '\'' | '"' => {
                in_string = true;
                string_quote = ch;
                out.push(ch);
            }
            '(' | ')' => continue,
            c if c.is_whitespace() => continue,
            c => out.push(c.to_ascii_lowercase()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed() -> Vec<String> {
        vec![
            "name".to_string(),
            "country".to_string(),
            "population".to_string(),
            "is_capital".to_string(),
            "founded_year".to_string(),
            "iso_a2".to_string(),
            "geom".to_string(),
        ]
    }

    #[test]
    fn parses_basic_text_equality() {
        let sql =
            translate_filter_to_sql("name = 'Berlin'", Some("cql2-text"), &allowed()).unwrap();
        assert!(sql.contains("name"));
        assert!(sql.contains("'Berlin'"));
        assert!(sql.contains('='));
    }

    #[test]
    fn parses_comparison_and_conjunction() {
        let sql = translate_filter_to_sql(
            "population > 1000000 AND is_capital = true",
            Some("cql2-text"),
            &allowed(),
        )
        .unwrap();
        assert!(sql.contains("population"));
        assert!(sql.contains("1000000"));
        assert!(sql.contains("is_capital"));
        assert!(sql.to_lowercase().contains("and"));
    }

    #[test]
    fn parses_is_null() {
        let sql =
            translate_filter_to_sql("founded_year IS NULL", Some("cql2-text"), &allowed()).unwrap();
        assert!(sql.to_uppercase().contains("IS NULL"));
    }

    #[test]
    fn parses_in_operator() {
        let sql =
            translate_filter_to_sql("iso_a2 IN ('US','GB','DE')", Some("cql2-text"), &allowed())
                .unwrap();
        assert!(sql.contains("'US'"));
        assert!(sql.contains("'DE'"));
    }

    #[test]
    fn parses_between() {
        let sql = translate_filter_to_sql(
            "population BETWEEN 1000000 AND 10000000",
            Some("cql2-text"),
            &allowed(),
        )
        .unwrap();
        assert!(sql.to_uppercase().contains("BETWEEN"));
    }

    #[test]
    fn parses_cql2_json() {
        let json = r#"{"op":"=","args":[{"property":"iso_a2"},"DE"]}"#;
        let sql = translate_filter_to_sql(json, Some("cql2-json"), &allowed()).unwrap();
        assert!(sql.contains("iso_a2"));
        assert!(sql.contains("'DE'"));
    }

    #[test]
    fn auto_detect_picks_json_on_brace() {
        let json = r#"{"op":"=","args":[{"property":"iso_a2"},"DE"]}"#;
        let sql = translate_filter_to_sql(json, None, &allowed()).unwrap();
        assert!(sql.contains("iso_a2"));
    }

    #[test]
    fn auto_detect_picks_text_otherwise() {
        let sql = translate_filter_to_sql("name = 'Paris'", None, &allowed()).unwrap();
        assert!(sql.contains("name"));
    }

    #[test]
    fn rejects_empty_filter() {
        assert!(translate_filter_to_sql("", Some("cql2-text"), &allowed()).is_err());
        assert!(translate_filter_to_sql("   ", Some("cql2-text"), &allowed()).is_err());
    }

    #[test]
    fn rejects_unknown_filter_lang() {
        assert!(translate_filter_to_sql("name = 'x'", Some("xpath"), &allowed()).is_err());
    }

    #[test]
    fn accepts_legacy_cql_text_alias() {
        let sql = translate_filter_to_sql("name = 'Paris'", Some("cql-text"), &allowed()).unwrap();
        assert!(sql.contains("'Paris'"));
    }

    #[test]
    fn spatial_intersects_maps_to_st_intersects() {
        let sql = translate_filter_to_sql(
            "S_INTERSECTS(geom, POINT(2.35 48.85))",
            Some("cql2-text"),
            &allowed(),
        )
        .unwrap();
        assert!(sql.to_lowercase().contains("st_intersects"));
    }

    #[test]
    fn sql_injection_trailing_semicolon_is_rejected_by_roundtrip_gate() {
        let attack = "name = 'x'; DROP TABLE cities; --";
        assert!(
            translate_filter_to_sql(attack, Some("cql2-text"), &allowed()).is_err(),
            "round-trip gate must reject trailing SQL garbage"
        );
    }

    #[test]
    fn rejects_function_calls_that_are_not_cql2_operators() {
        for filter in [
            "pg_sleep(10) = 1",
            "upper(name) = 'A'",
            "query_to_xml('select 1', true, true, '') like '%1%'",
            r#"{"op": "=", "args": [{"op": "pg_sleep", "args": [1]}, 1]}"#,
        ] {
            assert!(
                matches!(
                    translate_filter_to_sql(filter, None, &allowed()),
                    Err(TileServerError::InvalidTileRequest)
                ),
                "{filter} must be rejected"
            );
        }
    }

    #[test]
    fn accepts_standard_functions() {
        let props = allowed();
        for filter in [
            "casei(name) = casei('berlin')",
            "accenti(name) = accenti('Zürich')",
            "population + 1 > 2",
        ] {
            translate_filter_to_sql(filter, None, &props)
                .unwrap_or_else(|e| panic!("{filter}: {e:?}"));
        }
    }

    #[test]
    fn rejects_unknown_property() {
        assert!(
            translate_filter_to_sql("pg_catalog.pg_shadow = 'x'", Some("cql2-text"), &allowed())
                .is_err(),
            "catalog reference must be blocked"
        );
        assert!(
            translate_filter_to_sql("unknown_col = 1", Some("cql2-text"), &allowed()).is_err(),
            "non-whitelisted column must be blocked"
        );
    }

    #[test]
    fn rejects_property_in_json_filter_too() {
        let json = r#"{"op":"=","args":[{"property":"pg_shadow"},"x"]}"#;
        assert!(translate_filter_to_sql(json, Some("cql2-json"), &allowed()).is_err());
    }

    #[test]
    fn accepts_filter_without_spaces_around_operators() {
        let sql =
            translate_filter_to_sql("population>5000000", Some("cql2-text"), &allowed()).unwrap();
        assert!(sql.contains("population"));
        assert!(sql.contains("5000000"));
    }

    #[test]
    fn accepts_tight_equality() {
        let sql = translate_filter_to_sql("iso_a2='DE'", Some("cql2-text"), &allowed()).unwrap();
        assert!(sql.contains("iso_a2"));
        assert!(sql.contains("'DE'"));
    }

    #[test]
    fn accepts_between_with_varied_whitespace() {
        let sql = translate_filter_to_sql(
            "population  BETWEEN  1000000  AND  10000000",
            Some("cql2-text"),
            &allowed(),
        )
        .unwrap();
        assert!(sql.to_uppercase().contains("BETWEEN"));
    }

    #[test]
    fn canonicaliser_ignores_whitespace_around_operators() {
        assert_eq!(
            canonicalise_cql_text("population>5000000"),
            canonicalise_cql_text("(population > 5000000)")
        );
        assert_eq!(
            canonicalise_cql_text("iso_a2='DE'"),
            canonicalise_cql_text("(iso_a2 = 'DE')")
        );
    }
}
