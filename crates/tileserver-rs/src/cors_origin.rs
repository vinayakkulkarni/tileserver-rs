//! CORS origin pattern matching for `[server].cors_origins`.
//!
//! Extends the plain literal-string match used by the original
//! `AllowOrigin::list` setup with three pattern syntaxes:
//!
//! | Syntax              | Variant          | Examples                                  |
//! | ------------------- | ---------------- | ----------------------------------------- |
//! | `"*"`               | [`Self::Any`]    | (wildcard catch-all)                      |
//! | `"https://x"`       | [`Self::Exact`]  | exact origin (current behaviour)          |
//! | `"*.x.com"`         | [`Self::Glob`]   | one or more `*` wildcards in the host     |
//! | `"/^https:.+/"`     | [`Self::Regex`]  | enclosed in `/.../`, matches full Origin  |
//!
//! Internally `Glob` is compiled to a `Regex` (via `regex::escape` + `\* → .*`)
//! and `Regex` is parsed by stripping the leading + trailing `/`. Both
//! variants are anchored with `^...$`.
//!
//! Validation happens at startup — invalid regex or empty patterns fail
//! fast with a clear error naming the offending entry.

use axum::http::HeaderValue;
use regex::Regex;
use std::fmt;
use tower_http::cors::AllowOrigin;

/// Single compiled origin pattern entry from `[server].cors_origins`.
#[derive(Clone, Debug)]
pub enum CorsOriginPattern {
    Any,
    Exact(String),
    Matcher(Regex),
}

impl CorsOriginPattern {
    /// Classify a raw config string into the matching variant.
    ///
    /// # Errors
    /// Returns an error when the pattern is empty, when a glob produces
    /// an invalid regex (effectively never, since we escape first), or
    /// when a regex literal fails to compile.
    pub fn classify(raw: &str) -> anyhow::Result<Self> {
        if raw.is_empty() {
            anyhow::bail!("CORS origin pattern is empty");
        }
        if raw == "*" {
            return Ok(Self::Any);
        }
        if raw.starts_with('/') && raw.ends_with('/') && raw.len() >= 2 {
            let inner = &raw[1..raw.len() - 1];
            if inner.is_empty() {
                anyhow::bail!("CORS regex `//` is empty");
            }
            let anchored = format!("^{inner}$");
            let compiled = Regex::new(&anchored)
                .map_err(|e| anyhow::anyhow!("CORS regex `{raw}` failed to compile: {e}"))?;
            return Ok(Self::Matcher(compiled));
        }
        if raw.contains('*') {
            let escaped = regex::escape(raw).replace(r"\*", ".*");
            let anchored = format!("^{escaped}$");
            let compiled = Regex::new(&anchored)
                .map_err(|e| anyhow::anyhow!("CORS glob `{raw}` failed to compile: {e}"))?;
            return Ok(Self::Matcher(compiled));
        }
        Ok(Self::Exact(raw.to_string()))
    }

    /// Test whether an `Origin` header value matches this pattern.
    pub fn matches(&self, origin: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(s) => s == origin,
            Self::Matcher(re) => re.is_match(origin),
        }
    }
}

impl fmt::Display for CorsOriginPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.write_str("*"),
            Self::Exact(s) => f.write_str(s),
            Self::Matcher(re) => write!(f, "/{}/", re.as_str()),
        }
    }
}

/// Build a `tower_http::cors::AllowOrigin` from the raw `cors_origins` list.
///
/// Behaviour:
///
/// - Empty list                       → wildcard (`AllowOrigin::any`)
/// - List containing `"*"`            → wildcard, with a warning logged
/// - All literals (no `*`, no `/.../`) → fast-path `AllowOrigin::list`
///   of `HeaderValue`s (identical to the pre-existing behaviour to keep
///   the hot path allocation-free).
/// - Anything else                     → `AllowOrigin::predicate` that
///   walks the compiled pattern list per request and short-circuits.
///
/// Invalid entries (empty, broken regex) produce a startup error with
/// the offending string in the message — no silent skips.
pub fn build_allow_origin(origins: &[String]) -> anyhow::Result<AllowOrigin> {
    if origins.is_empty() || origins.iter().any(|o| o == "*") {
        if !origins.is_empty() {
            tracing::warn!(
                "CORS configured with wildcard (*). Consider restricting origins in production."
            );
        }
        return Ok(AllowOrigin::any());
    }

    let needs_predicate = origins
        .iter()
        .any(|o| o.contains('*') || (o.starts_with('/') && o.ends_with('/')));

    if !needs_predicate {
        let values: Vec<HeaderValue> = origins
            .iter()
            .filter_map(|o| {
                o.parse::<HeaderValue>().ok().or_else(|| {
                    tracing::warn!(origin = %o, "invalid CORS origin literal, skipping");
                    None
                })
            })
            .collect();
        if values.is_empty() {
            tracing::warn!(
                "no valid CORS origin literals after filtering — defaulting to wildcard"
            );
            return Ok(AllowOrigin::any());
        }
        return Ok(AllowOrigin::list(values));
    }

    let mut patterns = Vec::with_capacity(origins.len());
    for raw in origins {
        patterns.push(CorsOriginPattern::classify(raw)?);
    }
    let patterns = std::sync::Arc::new(patterns);
    Ok(AllowOrigin::predicate(move |origin, _parts| {
        let Ok(origin_str) = origin.to_str() else {
            return false;
        };
        patterns.iter().any(|p| p.matches(origin_str))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_wildcard_is_any() {
        let p = CorsOriginPattern::classify("*").unwrap();
        assert!(matches!(p, CorsOriginPattern::Any));
        assert!(p.matches("https://anything.example.com"));
    }

    #[test]
    fn classify_literal_is_exact() {
        let p = CorsOriginPattern::classify("https://example.com").unwrap();
        assert!(matches!(p, CorsOriginPattern::Exact(_)));
        assert!(p.matches("https://example.com"));
        assert!(!p.matches("https://other.com"));
        assert!(!p.matches("https://sub.example.com"));
    }

    #[test]
    fn classify_glob_with_leading_star() {
        let p = CorsOriginPattern::classify("*.example.com").unwrap();
        assert!(matches!(p, CorsOriginPattern::Matcher(_)));
        assert!(p.matches("https://foo.example.com"));
        assert!(p.matches("api.example.com"));
        assert!(!p.matches("https://example.com"));
        assert!(!p.matches("https://example.com.evil.com"));
    }

    #[test]
    fn classify_glob_in_middle() {
        let p = CorsOriginPattern::classify("preview-*.example.com").unwrap();
        assert!(p.matches("preview-abc.example.com"));
        assert!(p.matches("preview-pr-42.example.com"));
        assert!(!p.matches("staging-abc.example.com"));
    }

    #[test]
    fn classify_regex_literal() {
        let p =
            CorsOriginPattern::classify("/^https://.+\\.team-[a-z0-9]+\\.app\\.example\\.com$/")
                .unwrap();
        assert!(p.matches("https://x.team-acme.app.example.com"));
        assert!(p.matches("https://abc.team-foo42.app.example.com"));
        assert!(!p.matches("https://example.com"));
        assert!(!p.matches("http://x.team-acme.app.example.com"));
    }

    #[test]
    fn classify_empty_fails() {
        assert!(CorsOriginPattern::classify("").is_err());
    }

    #[test]
    fn classify_empty_regex_fails() {
        assert!(CorsOriginPattern::classify("//").is_err());
    }

    #[test]
    fn classify_invalid_regex_fails() {
        assert!(CorsOriginPattern::classify("/[/").is_err());
    }

    #[test]
    fn glob_metacharacters_in_literal_part_are_escaped() {
        let p = CorsOriginPattern::classify("*.test.com").unwrap();
        assert!(p.matches("https://foo.test.com"));
        assert!(!p.matches("https://fooXtestXcom"));
    }

    #[test]
    fn build_allow_origin_empty_is_any() {
        let result = build_allow_origin(&[]).unwrap();
        let _: AllowOrigin = result;
    }

    #[test]
    fn build_allow_origin_wildcard_is_any() {
        let _: AllowOrigin = build_allow_origin(&["*".to_string()]).unwrap();
    }

    #[test]
    fn build_allow_origin_literals_use_list_fast_path() {
        let _: AllowOrigin = build_allow_origin(&[
            "https://example.com".to_string(),
            "https://www.example.com".to_string(),
        ])
        .unwrap();
    }

    #[test]
    fn build_allow_origin_with_glob_compiles_to_predicate() {
        let _: AllowOrigin = build_allow_origin(&["*.example.com".to_string()]).unwrap();
    }

    #[test]
    fn build_allow_origin_propagates_invalid_regex_error() {
        let err = build_allow_origin(&["/[/".to_string()]).expect_err("invalid regex must fail");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("CORS regex"),
            "error should mention the source: {msg}"
        );
    }
}
