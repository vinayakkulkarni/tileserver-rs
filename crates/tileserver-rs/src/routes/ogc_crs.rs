//! OGC API Features — Part 2: Coordinate Reference Systems by Reference.
//!
//! Helpers shared by the OGC handlers to parse and emit CRS URIs, and to
//! turn them into PostGIS SRIDs for `ST_Transform`. The public `Crs` type
//! normalises the two equivalent URIs that clients send for WGS 84
//! lon/lat (`OGC:CRS84` and `EPSG:4326`) so downstream SQL always sees a
//! single canonical SRID (4326).
//!
//! Reference: <https://docs.ogc.org/is/18-058/18-058.html>

use crate::error::TileServerError;

/// Canonical URI for WGS 84 longitude/latitude (OGC:CRS84).
pub(crate) const CRS84_URI: &str = "http://www.opengis.net/def/crs/OGC/1.3/CRS84";

/// Canonical URI for WGS 84 longitude/latitude with ellipsoidal height (OGC:CRS84h).
pub(crate) const CRS84H_URI: &str = "http://www.opengis.net/def/crs/OGC/0/CRS84h";

/// A parsed OGC API CRS reference.
///
/// Flattens the many textual encodings clients send (OGC CRS84, EPSG HTTP
/// URI, EPSG URN, raw `EPSG:4326`) into a single `(srid, uri)` pair that
/// the PostGIS layer and response serializers can both use without
/// re-parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Crs {
    srid: i32,
    uri: String,
}

impl Crs {
    /// WGS 84 longitude/latitude — the OGC API Features default CRS.
    #[must_use]
    pub(crate) fn crs84() -> Self {
        Self {
            srid: 4326,
            uri: CRS84_URI.to_string(),
        }
    }

    /// Returns a CRS built directly from an EPSG SRID.
    #[must_use]
    pub(crate) fn from_srid(srid: i32) -> Self {
        Self {
            srid,
            uri: format!("http://www.opengis.net/def/crs/EPSG/0/{srid}"),
        }
    }

    /// PostGIS SRID to pass to `ST_Transform`/`ST_SetSRID`.
    #[must_use]
    pub(crate) const fn srid(&self) -> i32 {
        self.srid
    }

    /// Returns the value for the OGC `Content-Crs` response header, which
    /// the spec requires be wrapped in angle brackets.
    #[must_use]
    pub(crate) fn header_value(&self) -> String {
        format!("<{}>", self.uri)
    }
}

/// Parses an OGC API Features CRS query-parameter value.
///
/// Accepts the three forms clients and client libraries produce:
/// - HTTP URIs such as `http://www.opengis.net/def/crs/EPSG/0/4326` or the
///   `OGC/1.3/CRS84` sentinel.
/// - OGC URNs such as `urn:ogc:def:crs:EPSG::4326` or `urn:ogc:def:crs:OGC:1.3:CRS84`.
/// - Shorthand `EPSG:<code>` or bare numeric `<code>` as emitted by older tools.
///
/// Returns the canonical `Crs` (CRS84 is normalised to SRID 4326 so the
/// PostGIS query layer only has to reason about EPSG codes).
///
/// # Errors
///
/// Returns [`TileServerError::InvalidTileRequest`] for any unrecognised or
/// malformed string. This is propagated as `400 Bad Request` to the caller
/// so OGC clients get a clear signal instead of a silent 500 from PostGIS.
pub(crate) fn parse_crs(raw: &str) -> Result<Crs, TileServerError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(TileServerError::InvalidTileRequest);
    }

    if trimmed.eq_ignore_ascii_case(CRS84_URI)
        || trimmed.eq_ignore_ascii_case("urn:ogc:def:crs:OGC:1.3:CRS84")
        || trimmed.eq_ignore_ascii_case("CRS84")
        || trimmed.eq_ignore_ascii_case("OGC:CRS84")
    {
        return Ok(Crs::crs84());
    }
    if trimmed.eq_ignore_ascii_case(CRS84H_URI)
        || trimmed.eq_ignore_ascii_case("urn:ogc:def:crs:OGC:0:CRS84h")
    {
        return Ok(Crs::crs84());
    }

    if let Some(code) = trimmed
        .strip_prefix("http://www.opengis.net/def/crs/EPSG/0/")
        .or_else(|| trimmed.strip_prefix("https://www.opengis.net/def/crs/EPSG/0/"))
    {
        return parse_epsg_code(code);
    }

    if let Some(code) = trimmed
        .strip_prefix("urn:ogc:def:crs:EPSG::")
        .or_else(|| trimmed.strip_prefix("urn:ogc:def:crs:EPSG:"))
    {
        return parse_epsg_code(code.trim_start_matches(':'));
    }

    if let Some(code) = trimmed.strip_prefix("EPSG:") {
        return parse_epsg_code(code);
    }

    if let Ok(srid) = trimmed.parse::<i32>() {
        return Ok(Crs::from_srid(srid));
    }

    Err(TileServerError::InvalidTileRequest)
}

fn parse_epsg_code(code: &str) -> Result<Crs, TileServerError> {
    code.trim()
        .parse::<i32>()
        .map(Crs::from_srid)
        .map_err(|_| TileServerError::InvalidTileRequest)
}

/// Returns the list of CRS URIs a table supports.
///
/// Every PostgresTableSource advertises both its storage CRS and CRS84 so
/// that OGC clients (QGIS, ArcGIS Pro, FME) can always request features in
/// the default lon/lat frame even when the underlying table is, for
/// example, EPSG:25832. Duplicates are stripped when the storage CRS is
/// already CRS84.
pub(crate) fn collection_supported_crs(storage_srid: i32) -> Vec<String> {
    if storage_srid == 4326 {
        vec![CRS84_URI.to_string()]
    } else {
        vec![
            CRS84_URI.to_string(),
            format!("http://www.opengis.net/def/crs/EPSG/0/{storage_srid}"),
        ]
    }
}

/// Returns the URI a collection advertises as its storage CRS.
pub(crate) fn storage_crs_uri(storage_srid: i32) -> String {
    if storage_srid == 4326 {
        CRS84_URI.to_string()
    } else {
        format!("http://www.opengis.net/def/crs/EPSG/0/{storage_srid}")
    }
}

/// Conformance class URI for OGC API Features Part 2.
pub(crate) const CONFORMANCE_CRS: &str =
    "http://www.opengis.net/spec/ogcapi-features-2/1.0/conf/crs";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_epsg_uri() {
        let crs = parse_crs("http://www.opengis.net/def/crs/EPSG/0/25832").unwrap();
        assert_eq!(crs.srid(), 25832);
        assert!(crs.uri.as_str().ends_with("/25832"));
    }

    #[test]
    fn parses_urn_epsg() {
        let crs = parse_crs("urn:ogc:def:crs:EPSG::3857").unwrap();
        assert_eq!(crs.srid(), 3857);
    }

    #[test]
    fn parses_shorthand_epsg() {
        let crs = parse_crs("EPSG:4326").unwrap();
        assert_eq!(crs.srid(), 4326);
    }

    #[test]
    fn parses_bare_numeric() {
        let crs = parse_crs("4326").unwrap();
        assert_eq!(crs.srid(), 4326);
    }

    #[test]
    fn crs84_normalizes_to_4326() {
        assert_eq!(parse_crs(CRS84_URI).unwrap().srid(), 4326);
        assert_eq!(parse_crs("CRS84").unwrap().srid(), 4326);
        assert_eq!(
            parse_crs("urn:ogc:def:crs:OGC:1.3:CRS84").unwrap().srid(),
            4326
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_crs("").is_err());
        assert!(parse_crs("not-a-crs").is_err());
        assert!(parse_crs("EPSG:abc").is_err());
    }

    #[test]
    fn header_value_wraps_in_brackets() {
        let crs = Crs::from_srid(25832);
        let header = crs.header_value();
        assert!(header.starts_with('<'));
        assert!(header.ends_with('>'));
        assert!(header.contains("/25832"));
    }

    #[test]
    fn collection_supported_crs_includes_storage_and_crs84() {
        let crs = collection_supported_crs(25832);
        assert!(crs.iter().any(|u| u == CRS84_URI));
        assert!(crs.iter().any(|u| u.ends_with("/25832")));
        assert_eq!(crs.len(), 2);
    }

    #[test]
    fn collection_supported_crs_deduplicates_4326() {
        let crs = collection_supported_crs(4326);
        assert_eq!(crs, vec![CRS84_URI.to_string()]);
    }
}
