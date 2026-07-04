//! Pure logic for the embeddable iframe map endpoint (`GET /embed/{style}`).
//!
//! This module owns everything the [`crate::routes::embed`] handler needs:
//! typed query-parameter parsing ([`parse_embed_query`]), HTML escaping
//! ([`html_escape`]), and self-contained HTML page generation
//! ([`build_embed_html`]).
//!
//! Security posture: every query parameter is parsed into a typed value at the
//! boundary (`f64`, `u8`, `bool`, whitelisted enums). The two remaining string
//! values injected into the page — the style id and theme — are additionally
//! HTML-escaped via [`html_escape`]. Numeric values are serialised as JSON
//! number literals, so they can never carry markup. This is belt-and-braces:
//! typed parsing alone would suffice, but the escape provides defence in depth
//! for the most security-sensitive endpoint in the server.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::error::TileServerError;

/// Pinned MapLibre GL JS CDN version. Single source of truth for both the
/// script and stylesheet URLs injected into the embed page.
pub(crate) const MAPLIBRE_VERSION: &str = "5.6.1";

/// Default zoom when no `center`/`bounds` is supplied.
const DEFAULT_ZOOM: f64 = 2.0;

/// A single map marker as a validated `(lng, lat)` pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EmbedMarker {
    pub(crate) lng: f64,
    pub(crate) lat: f64,
}

/// Whitelisted map control kinds. Any unknown token in `?controls=` is dropped
/// silently rather than erroring, so a typo never 400s the whole request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    Navigation,
    Scale,
    Fullscreen,
}

impl Control {
    /// The JS-visible token for this control (used in the `CONTROLS` array).
    fn as_token(self) -> &'static str {
        match self {
            Self::Navigation => "navigation",
            Self::Scale => "scale",
            Self::Fullscreen => "fullscreen",
        }
    }

    /// Parse a single lowercase token into a [`Control`], if recognised.
    fn from_token(token: &str) -> Option<Self> {
        match token {
            "navigation" => Some(Self::Navigation),
            "scale" => Some(Self::Scale),
            "fullscreen" => Some(Self::Fullscreen),
            _ => None,
        }
    }
}

/// Requested color theme for the embed page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Theme {
    Light,
    Dark,
}

impl Theme {
    /// Parse a whitelisted theme token. Unknown values yield `None`, which the
    /// page renders as "auto" (`prefers-color-scheme`).
    fn from_token(token: &str) -> Option<Self> {
        match token {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// The `data-theme` attribute value.
    fn as_attr(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// Typed, validated embed query parameters.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EmbedParams {
    /// `(lat, lng)` as stored (lat first, matching the `center=lat,lng` input).
    pub(crate) center: Option<[f64; 2]>,
    pub(crate) zoom: f64,
    pub(crate) bearing: f64,
    pub(crate) pitch: f64,
    /// `[min_lng, min_lat, max_lng, max_lat]` when a bounds query is present.
    pub(crate) bounds: Option<[f64; 4]>,
    pub(crate) markers: Vec<EmbedMarker>,
    pub(crate) controls: Vec<Control>,
    pub(crate) hash: bool,
    pub(crate) interactive: bool,
    pub(crate) theme: Option<Theme>,
}

impl Default for EmbedParams {
    fn default() -> Self {
        Self {
            center: None,
            zoom: DEFAULT_ZOOM,
            bearing: 0.0,
            pitch: 0.0,
            bounds: None,
            markers: Vec::new(),
            controls: vec![Control::Navigation],
            hash: false,
            interactive: true,
            theme: None,
        }
    }
}

/// HTML-escape a string, replacing the six characters that could break out of
/// an attribute or `<script>` context. Returns a borrowed `Cow` when the input
/// is already safe (the common case) to avoid an allocation.
pub(crate) fn html_escape(s: &str) -> Cow<'_, str> {
    if !s
        .bytes()
        .any(|b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\'' | b'`'))
    {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '`' => out.push_str("&#x60;"),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Parse a finite `f64`, rejecting NaN and infinities.
fn parse_finite(s: &str) -> Option<f64> {
    let v: f64 = s.trim().parse().ok()?;
    if v.is_finite() { Some(v) } else { None }
}

/// Parse `center=lat,lng` into a validated `[lat, lng]` pair.
fn parse_center(raw: &str) -> Result<[f64; 2], TileServerError> {
    let mut parts = raw.split(',');
    let (Some(lat_s), Some(lng_s), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(TileServerError::InvalidTileRequest);
    };
    let lat = parse_finite(lat_s).ok_or(TileServerError::InvalidTileRequest)?;
    let lng = parse_finite(lng_s).ok_or(TileServerError::InvalidTileRequest)?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
        return Err(TileServerError::InvalidTileRequest);
    }
    Ok([lat, lng])
}

/// Parse `bounds=min_lng,min_lat,max_lng,max_lat` into a validated array.
fn parse_bounds(raw: &str) -> Result<[f64; 4], TileServerError> {
    let parts: Vec<&str> = raw.split(',').collect();
    if parts.len() != 4 {
        return Err(TileServerError::InvalidTileRequest);
    }
    let mut vals = [0.0_f64; 4];
    for (slot, part) in vals.iter_mut().zip(parts.iter()) {
        *slot = parse_finite(part).ok_or(TileServerError::InvalidTileRequest)?;
    }
    let [min_lng, min_lat, max_lng, max_lat] = vals;
    if min_lng > max_lng || min_lat > max_lat {
        return Err(TileServerError::InvalidTileRequest);
    }
    Ok(vals)
}

/// Parse `markers=lng,lat|lng,lat|...` into validated pairs. Empty input is an
/// empty list (not an error); any malformed pair rejects the whole request.
fn parse_markers(raw: &str) -> Result<Vec<EmbedMarker>, TileServerError> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for chunk in raw.split('|') {
        let mut parts = chunk.split(',');
        let (Some(lng_s), Some(lat_s), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(TileServerError::InvalidTileRequest);
        };
        let lng = parse_finite(lng_s).ok_or(TileServerError::InvalidTileRequest)?;
        let lat = parse_finite(lat_s).ok_or(TileServerError::InvalidTileRequest)?;
        out.push(EmbedMarker { lng, lat });
    }
    Ok(out)
}

/// Parse `controls=navigation,scale,...`. Unknown tokens are dropped silently.
/// When absent, defaults to `[navigation]`.
fn parse_controls(raw: &str) -> Vec<Control> {
    let mut out = Vec::new();
    for token in raw.split(',') {
        if let Some(c) = Control::from_token(token.trim().to_ascii_lowercase().as_str())
            && !out.contains(&c)
        {
            out.push(c);
        }
    }
    out
}

/// Parse a loose boolean: `true`/`1` → true, `false`/`0` → false, anything
/// else → `default`.
fn parse_loose_bool(raw: &str, default: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" => true,
        "false" | "0" => false,
        _ => default,
    }
}

/// Parse the raw query map into typed, validated [`EmbedParams`].
///
/// # Errors
///
/// Returns [`TileServerError::InvalidTileRequest`] (HTTP 400) when `center`,
/// `bounds`, or `markers` fail to parse or fall outside their valid ranges.
pub(crate) fn parse_embed_query(
    raw: &HashMap<String, String>,
) -> Result<EmbedParams, TileServerError> {
    let mut params = EmbedParams::default();

    if let Some(v) = raw.get("center") {
        params.center = Some(parse_center(v)?);
    }
    if let Some(v) = raw.get("bounds") {
        params.bounds = Some(parse_bounds(v)?);
    }
    if let Some(v) = raw.get("markers") {
        params.markers = parse_markers(v)?;
    }
    if let Some(v) = raw.get("controls") {
        params.controls = parse_controls(v);
    }
    if let Some(z) = raw.get("zoom").and_then(|v| parse_finite(v)) {
        params.zoom = z.clamp(0.0, 22.0);
    }
    if let Some(b) = raw.get("bearing").and_then(|v| parse_finite(v)) {
        params.bearing = b.clamp(-360.0, 360.0);
    }
    if let Some(p) = raw.get("pitch").and_then(|v| parse_finite(v)) {
        params.pitch = p.clamp(0.0, 85.0);
    }
    if let Some(v) = raw.get("hash") {
        params.hash = parse_loose_bool(v, false);
    }
    if let Some(v) = raw.get("interactive") {
        params.interactive = parse_loose_bool(v, true);
    }
    if let Some(v) = raw.get("theme") {
        params.theme = Theme::from_token(v.trim().to_ascii_lowercase().as_str());
    }

    Ok(params)
}

/// Serialise a `f64` as a compact JSON number literal (no HTML-unsafe chars).
fn num(v: f64) -> String {
    // serde_json guarantees a plain numeric literal for finite f64.
    serde_json::to_string(&v).unwrap_or_else(|_| "0".to_string())
}

/// Build the self-contained embed HTML page for the given params + style.
///
/// `style_id` is the (already looked-up) style identifier, `base_url` is the
/// server's public base URL, and `style_name` is used only in the `<title>`.
/// Both `style_id` and `style_name`/`theme` are HTML-escaped before injection.
pub(crate) fn build_embed_html(
    params: &EmbedParams,
    style_id: &str,
    base_url: &str,
    style_name: &str,
) -> String {
    let esc_id = html_escape(style_id);
    let esc_name = html_escape(style_name);

    let style_url = format!("{}/styles/{}/style.json", base_url, esc_id);
    let style_url_json = serde_json::to_string(&style_url).unwrap_or_else(|_| "\"\"".to_string());

    let center_json = match params.center {
        // JS expects [lng, lat]; stored order is [lat, lng].
        Some([lat, lng]) => format!("[{}, {}]", num(lng), num(lat)),
        None => "null".to_string(),
    };
    let zoom_json = num(params.zoom);
    let bearing_json = num(params.bearing);
    let pitch_json = num(params.pitch);
    let bounds_json = match params.bounds {
        Some([min_lng, min_lat, max_lng, max_lat]) => format!(
            "[[{}, {}], [{}, {}]]",
            num(min_lng),
            num(min_lat),
            num(max_lng),
            num(max_lat)
        ),
        None => "null".to_string(),
    };

    let markers_json = {
        let pairs: Vec<String> = params
            .markers
            .iter()
            .map(|m| format!("[{}, {}]", num(m.lng), num(m.lat)))
            .collect();
        format!("[{}]", pairs.join(", "))
    };

    let controls_json = {
        let tokens: Vec<String> = params
            .controls
            .iter()
            .map(|c| format!("{:?}", c.as_token()))
            .collect();
        format!("[{}]", tokens.join(", "))
    };

    let hash_json = if params.hash { "true" } else { "false" };
    let interactive_json = if params.interactive { "true" } else { "false" };

    let (theme_attr, color_scheme) = match params.theme {
        Some(t) => (t.as_attr().to_string(), t.as_attr().to_string()),
        None => (String::new(), "light dark".to_string()),
    };
    let theme_attr = html_escape(&theme_attr).into_owned();

    // Auto-theme script: only injected when no explicit theme is requested.
    let auto_theme_script = if params.theme.is_none() {
        r#"<script>(function(){try{if(matchMedia("(prefers-color-scheme: dark)").matches){document.documentElement.setAttribute("data-theme","dark");}else{document.documentElement.setAttribute("data-theme","light");}}catch(e){}})();</script>"#
    } else {
        ""
    };

    // Interactive=false: disable every input handler on the map.
    let disable_interaction = if params.interactive {
        String::new()
    } else {
        "        map.dragPan.disable();\n        map.scrollZoom.disable();\n        map.doubleClickZoom.disable();\n        map.keyboard.disable();\n        map.touchZoomRotate.disable();\n        map.boxZoom.disable();\n        map.dragRotate.disable();\n".to_string()
    };

    let css = MAPLIBRE_VERSION;

    format!(
        r##"<!doctype html>
<html lang="en" data-theme="{theme_attr}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>{esc_name} — embed</title>
  <link rel="stylesheet" href="https://unpkg.com/maplibre-gl@{css}/dist/maplibre-gl.css">
  {auto_theme_script}
  <style>
    html,body{{margin:0;padding:0;height:100%;width:100%;overflow:hidden;background:#0a0a0a;color-scheme:{color_scheme};}}
    #m{{position:absolute;inset:0;}}
    .maplibregl-ctrl-bottom-right,.maplibregl-ctrl-bottom-left{{z-index:2;}}
  </style>
</head>
<body>
  <div id="m"></div>
  <script src="https://unpkg.com/maplibre-gl@{css}/dist/maplibre-gl.js"></script>
  <script>
    (function () {{
      const STYLE_URL  = {style_url_json};
      const CENTER     = {center_json};
      const ZOOM       = {zoom_json};
      const BEARING    = {bearing_json};
      const PITCH      = {pitch_json};
      const BOUNDS     = {bounds_json};
      const MARKERS    = {markers_json};
      const CONTROLS   = {controls_json};
      const HASH       = {hash_json};
      const INTERACTIVE= {interactive_json};
      const POST_TARGET= "*";

      const map = new maplibregl.Map({{
        container: "m",
        style: STYLE_URL,
        hash: HASH,
        interactive: INTERACTIVE,
        attributionControl: {{ compact: true }},
      }});
{disable_interaction}      if (CONTROLS.includes("navigation")) map.addControl(new maplibregl.NavigationControl({{ visualizePitch: true }}), "top-right");
      if (CONTROLS.includes("scale"))      map.addControl(new maplibregl.ScaleControl({{ unit: "metric" }}), "bottom-left");
      if (CONTROLS.includes("fullscreen")) map.addControl(new maplibregl.FullscreenControl(), "top-right");

      map.on("load", function () {{
        if (BOUNDS) {{
          map.fitBounds(BOUNDS, {{ padding: 32, animate: false, duration: 0 }});
        }} else if (CENTER) {{
          map.jumpTo({{ center: CENTER, zoom: ZOOM, bearing: BEARING, pitch: PITCH }});
        }}
        for (const m of MARKERS) {{
          new maplibregl.Marker({{ color: "#5b21b6" }}).setLngLat(m).addTo(map);
        }}
        if (window.parent && window.parent !== window) {{
          window.parent.postMessage({{ type: "ready", style: STYLE_URL }}, POST_TARGET);
        }}
        let lastMove = 0;
        map.on("move", function () {{
          const now = Date.now();
          if (now - lastMove < 200) return;
          lastMove = now;
          const c = map.getCenter();
          if (window.parent && window.parent !== window) {{
            window.parent.postMessage({{
              type: "move",
              lng: c.lng, lat: c.lat,
              zoom: map.getZoom(), bearing: map.getBearing(), pitch: map.getPitch(),
            }}, POST_TARGET);
          }}
        }});
        map.on("click", function (e) {{
          if (window.parent && window.parent !== window) {{
            const feats = map.queryRenderedFeatures(e.point);
            window.parent.postMessage({{
              type: "click", lng: e.lngLat.lng, lat: e.lngLat.lat,
              features: feats.map(function (f) {{ return f.layer.id }}),
            }}, POST_TARGET);
          }}
        }});
      }});

      window.addEventListener("message", function (ev) {{
        const d = ev.data || {{}};
        if (!d || typeof d !== "object") return;
        if (d.type === "flyTo")       {{ map.flyTo({{ center:[d.lng,d.lat], zoom:d.zoom, bearing:d.bearing, pitch:d.pitch, essential:true }}); }}
        else if (d.type === "fitBounds"){{ if (Array.isArray(d.bounds)) map.fitBounds(d.bounds, {{ padding: d.padding || 32 }}); }}
        else if (d.type === "setFilter"){{ if (d.layerId) map.setFilter(d.layerId, d.filter); }}
        else if (d.type === "setLayoutProperty"){{ if (d.layerId) map.setLayoutProperty(d.layerId, d.property, d.value); }}
      }});
    }})();
  </script>
</body>
</html>
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    // ---- parse_embed_query: defaults & core parsing ----

    #[test]
    fn parse_query_minimal_defaults() {
        let p = parse_embed_query(&HashMap::new()).unwrap();
        assert_eq!(p.center, None);
        assert_eq!(p.zoom, 2.0);
        assert_eq!(p.bearing, 0.0);
        assert_eq!(p.pitch, 0.0);
        assert_eq!(p.bounds, None);
        assert!(p.markers.is_empty());
        assert_eq!(p.controls, vec![Control::Navigation]);
        assert!(!p.hash);
        assert!(p.interactive);
        assert_eq!(p.theme, None);
    }

    #[test]
    fn parse_query_center_comma() {
        let p = parse_embed_query(&map_of(&[("center", "37.8,-122.4")])).unwrap();
        assert_eq!(p.center, Some([37.8, -122.4]));
    }

    #[test]
    fn parse_query_center_lat_out_of_range() {
        let err = parse_embed_query(&map_of(&[("center", "91,0")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_center_lng_out_of_range() {
        let err = parse_embed_query(&map_of(&[("center", "0,181")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_center_non_numeric() {
        let err = parse_embed_query(&map_of(&[("center", "not,a")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_center_wrong_arity() {
        let err = parse_embed_query(&map_of(&[("center", "1,2,3")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_center_nan_rejected() {
        let err = parse_embed_query(&map_of(&[("center", "NaN,0")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_bounds_inverted() {
        // min_lng > max_lng
        let err = parse_embed_query(&map_of(&[("bounds", "10,10,5,5")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_bounds_wrong_arity() {
        let err = parse_embed_query(&map_of(&[("bounds", "1,2,3")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_bounds_valid() {
        let p = parse_embed_query(&map_of(&[("bounds", "-10,-10,10,10")])).unwrap();
        assert_eq!(p.bounds, Some([-10.0, -10.0, 10.0, 10.0]));
    }

    #[test]
    fn parse_query_markers_pipe_separated() {
        let p = parse_embed_query(&map_of(&[("markers", "-122.4,37.8|0,0")])).unwrap();
        assert_eq!(p.markers.len(), 2);
        assert_eq!(
            p.markers[0],
            EmbedMarker {
                lng: -122.4,
                lat: 37.8
            }
        );
        assert_eq!(p.markers[1], EmbedMarker { lng: 0.0, lat: 0.0 });
    }

    #[test]
    fn parse_query_markers_bad_pair() {
        let err = parse_embed_query(&map_of(&[("markers", "abc")])).unwrap_err();
        assert!(matches!(err, TileServerError::InvalidTileRequest));
    }

    #[test]
    fn parse_query_markers_empty_is_empty_list() {
        let p = parse_embed_query(&map_of(&[("markers", "")])).unwrap();
        assert!(p.markers.is_empty());
    }

    #[test]
    fn parse_query_controls_default_when_absent() {
        let p = parse_embed_query(&HashMap::new()).unwrap();
        assert_eq!(p.controls, vec![Control::Navigation]);
    }

    #[test]
    fn parse_query_controls_unknown_token_filtered() {
        let p = parse_embed_query(&map_of(&[("controls", "navigation,banana,scale")])).unwrap();
        assert_eq!(p.controls, vec![Control::Navigation, Control::Scale]);
    }

    #[test]
    fn parse_query_controls_explicit_set() {
        let p = parse_embed_query(&map_of(&[("controls", "scale,fullscreen")])).unwrap();
        assert_eq!(p.controls, vec![Control::Scale, Control::Fullscreen]);
    }

    #[test]
    fn parse_query_controls_empty_yields_empty() {
        let p = parse_embed_query(&map_of(&[("controls", "banana")])).unwrap();
        assert!(p.controls.is_empty());
    }

    #[test]
    fn parse_query_zoom_clamps_to_22() {
        let p = parse_embed_query(&map_of(&[("zoom", "99")])).unwrap();
        assert_eq!(p.zoom, 22.0);
    }

    #[test]
    fn parse_query_zoom_negative_clamps_to_zero() {
        let p = parse_embed_query(&map_of(&[("zoom", "-5")])).unwrap();
        assert_eq!(p.zoom, 0.0);
    }

    #[test]
    fn parse_query_bearing_clamps_to_360() {
        let p = parse_embed_query(&map_of(&[("bearing", "999")])).unwrap();
        assert_eq!(p.bearing, 360.0);
    }

    #[test]
    fn parse_query_pitch_clamps_to_85() {
        let p = parse_embed_query(&map_of(&[("pitch", "120")])).unwrap();
        assert_eq!(p.pitch, 85.0);
    }

    #[test]
    fn parse_query_hash_true() {
        let p = parse_embed_query(&map_of(&[("hash", "true")])).unwrap();
        assert!(p.hash);
    }

    #[test]
    fn parse_query_hash_one() {
        let p = parse_embed_query(&map_of(&[("hash", "1")])).unwrap();
        assert!(p.hash);
    }

    #[test]
    fn parse_query_hash_garbage_defaults_false() {
        let p = parse_embed_query(&map_of(&[("hash", "yeah")])).unwrap();
        assert!(!p.hash);
    }

    #[test]
    fn parse_query_interactive_false() {
        let p = parse_embed_query(&map_of(&[("interactive", "false")])).unwrap();
        assert!(!p.interactive);
    }

    #[test]
    fn parse_query_interactive_zero() {
        let p = parse_embed_query(&map_of(&[("interactive", "0")])).unwrap();
        assert!(!p.interactive);
    }

    #[test]
    fn parse_query_interactive_garbage_defaults_true() {
        let p = parse_embed_query(&map_of(&[("interactive", "maybe")])).unwrap();
        assert!(p.interactive);
    }

    #[test]
    fn parse_query_theme_light() {
        let p = parse_embed_query(&map_of(&[("theme", "light")])).unwrap();
        assert_eq!(p.theme, Some(Theme::Light));
    }

    #[test]
    fn parse_query_theme_dark() {
        let p = parse_embed_query(&map_of(&[("theme", "dark")])).unwrap();
        assert_eq!(p.theme, Some(Theme::Dark));
    }

    #[test]
    fn parse_query_theme_unknown_filtered_to_none() {
        let p = parse_embed_query(&map_of(&[("theme", "neon")])).unwrap();
        assert_eq!(p.theme, None);
    }

    // ---- html_escape ----

    #[test]
    fn escape_passthrough_when_safe() {
        assert_eq!(html_escape("osm-bright"), Cow::Borrowed("osm-bright"));
    }

    #[test]
    fn escape_amp_lt_gt_quot_apos_backtick() {
        assert_eq!(html_escape("&"), "&amp;");
        assert_eq!(html_escape("<"), "&lt;");
        assert_eq!(html_escape(">"), "&gt;");
        assert_eq!(html_escape("\""), "&quot;");
        assert_eq!(html_escape("'"), "&#x27;");
        assert_eq!(html_escape("`"), "&#x60;");
    }

    #[test]
    fn escape_xss_payload() {
        let out = html_escape("\"><script>alert(1)</script>");
        assert!(!out.contains('<'));
        assert!(!out.contains('>'));
        assert!(!out.contains('"'));
        assert!(out.contains("&lt;script&gt;"));
    }

    #[test]
    fn escape_empty_string() {
        assert_eq!(html_escape(""), "");
    }

    #[test]
    fn escape_unicode_preserved() {
        assert_eq!(html_escape("café 🗺️"), "café 🗺️");
    }

    // ---- build_embed_html ----

    fn params_with_center() -> EmbedParams {
        EmbedParams {
            center: Some([37.8, -122.4]),
            ..EmbedParams::default()
        }
    }

    fn build(params: &EmbedParams, style_id: &str) -> String {
        build_embed_html(params, style_id, "http://example.test", "Example Style")
    }

    #[test]
    fn html_contains_doctype_and_map_div() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains(r#"id="m""#));
    }

    #[test]
    fn html_contains_maplibre_pinned_cdn() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.contains("maplibre-gl@5.6.1/dist/maplibre-gl.js"));
        assert!(html.contains("maplibre-gl@5.6.1/dist/maplibre-gl.css"));
    }

    #[test]
    fn html_contains_style_url_absolute() {
        let html = build(&params_with_center(), "bright");
        assert!(html.contains("http://example.test/styles/bright/style.json"));
    }

    #[test]
    fn html_escapes_style_id() {
        let html = build(&params_with_center(), "x\"><script>");
        assert!(!html.contains("x\"><script>"));
        assert!(html.contains("&quot;&gt;&lt;script&gt;"));
    }

    #[test]
    fn html_escapes_theme() {
        let params = EmbedParams {
            // Theme only accepts whitelisted values, so a raw payload never
            // reaches the page as a theme; assert the injected attr is empty
            // and the escape helper still guards any string we do inject.
            theme: None,
            ..EmbedParams::default()
        };
        let html = build(&params, "bright");
        assert!(html.contains(r#"data-theme="""#));
        // Directly exercise the escape used for the theme attribute:
        assert!(!html_escape("dark\"><img onerror=1>").contains('<'));
    }

    #[test]
    fn html_center_present_when_set() {
        let html = build(&params_with_center(), "bright");
        // JS order is [lng, lat].
        assert!(html.contains("const CENTER     = [-122.4, 37.8]"));
    }

    #[test]
    fn html_bounds_branch_overrides_center() {
        let params = EmbedParams {
            center: Some([0.0, 0.0]),
            bounds: Some([-10.0, -10.0, 10.0, 10.0]),
            ..EmbedParams::default()
        };
        let html = build(&params, "bright");
        assert!(html.contains("const BOUNDS     = [[-10.0, -10.0], [10.0, 10.0]]"));
    }

    #[test]
    fn html_markers_rendered_as_json_array() {
        let params = EmbedParams {
            markers: vec![
                EmbedMarker {
                    lng: -122.4,
                    lat: 37.8,
                },
                EmbedMarker { lng: 0.0, lat: 0.0 },
            ],
            ..EmbedParams::default()
        };
        let html = build(&params, "bright");
        assert!(html.contains("[[-122.4, 37.8], [0.0, 0.0]]"));
    }

    #[test]
    fn html_controls_default_includes_navigation() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.contains("\"navigation\""));
    }

    #[test]
    fn html_controls_scale_only() {
        let params = EmbedParams {
            controls: vec![Control::Scale],
            ..EmbedParams::default()
        };
        let html = build(&params, "bright");
        assert!(html.contains("const CONTROLS   = [\"scale\"]"));
        assert!(!html.contains("[\"navigation\""));
    }

    #[test]
    fn html_post_message_handler_present() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.contains(r#"addEventListener("message""#));
        assert!(html.contains("flyTo"));
        assert!(html.contains("fitBounds"));
        assert!(html.contains("setFilter"));
        assert!(html.contains("setLayoutProperty"));
    }

    #[test]
    fn html_ready_posted_on_load() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.contains(r#"type: "ready""#));
    }

    #[test]
    fn html_color_scheme_dark_when_theme_dark() {
        let params = EmbedParams {
            theme: Some(Theme::Dark),
            ..EmbedParams::default()
        };
        let html = build(&params, "bright");
        assert!(html.contains("color-scheme:dark"));
        assert!(html.contains(r#"data-theme="dark""#));
    }

    #[test]
    fn html_color_scheme_auto_when_theme_none() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.contains("color-scheme:light dark"));
    }

    #[test]
    fn html_prefers_dark_script_when_theme_none() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.contains(r#"matchMedia("(prefers-color-scheme: dark)")"#));
    }

    #[test]
    fn html_no_prefers_script_when_theme_set() {
        let params = EmbedParams {
            theme: Some(Theme::Light),
            ..EmbedParams::default()
        };
        let html = build(&params, "bright");
        assert!(!html.contains("prefers-color-scheme"));
    }

    #[test]
    fn html_interactive_true_disables_nothing() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(!html.contains("dragPan.disable()"));
    }

    #[test]
    fn html_interactive_false_disables_inputs() {
        let params = EmbedParams {
            interactive: false,
            ..EmbedParams::default()
        };
        let html = build(&params, "bright");
        assert!(html.contains("map.dragPan.disable()"));
        assert!(html.contains("map.scrollZoom.disable()"));
        assert!(html.contains("map.doubleClickZoom.disable()"));
        assert!(html.contains("map.keyboard.disable()"));
        assert!(html.contains("map.touchZoomRotate.disable()"));
        assert!(html.contains("map.boxZoom.disable()"));
    }

    #[test]
    fn html_default_params_under_8kb() {
        let html = build(&EmbedParams::default(), "bright");
        assert!(html.len() <= 8192, "default embed html must be <= 8KB");
    }
}
