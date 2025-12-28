//! WMTS (Web Map Tile Service) Capabilities document generation
//!
//! Generates OGC WMTS 1.0.0 compliant GetCapabilities XML responses
//! for use with GIS software like QGIS and ArcGIS.

use std::fmt::Write;

/// Scale denominators for each zoom level in Web Mercator (EPSG:3857)
/// These are standard values for 256px tiles at 0.28mm/pixel (OGC standard)
const SCALE_DENOMINATORS_256: [f64; 19] = [
    559082264.02872,
    279541132.01436,
    139770566.00718,
    69885283.00359,
    34942641.501795,
    17471320.750897,
    8735660.3754487,
    4367830.1877244,
    2183915.0938622,
    1091957.5469311,
    545978.77346554,
    272989.38673277,
    136494.69336639,
    68247.346683193,
    34123.673341597,
    17061.836670798,
    8530.9183353991,
    4265.4591676996,
    2132.7295838498,
];

/// Generate WMTS GetCapabilities XML for a style
///
/// # Arguments
/// * `base_url` - Base URL of the server (e.g., "http://localhost:8080")
/// * `style_id` - Style identifier
/// * `style_name` - Human-readable style name
/// * `min_zoom` - Minimum zoom level
/// * `max_zoom` - Maximum zoom level
/// * `key` - Optional API key to append to all URLs as `?key=...`
pub fn generate_wmts_capabilities(
    base_url: &str,
    style_id: &str,
    style_name: &str,
    min_zoom: u8,
    max_zoom: u8,
    key: Option<&str>,
) -> String {
    let mut xml = String::with_capacity(32768);

    // Build query string for key parameter
    let key_query = key
        .map(|k| format!("?key={}", urlencoding::encode(k)))
        .unwrap_or_default();

    // XML declaration and root element
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<Capabilities xmlns="http://www.opengis.net/wmts/1.0" xmlns:ows="http://www.opengis.net/ows/1.1" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:gml="http://www.opengis.net/gml" xsi:schemaLocation="http://www.opengis.net/wmts/1.0 http://schemas.opengis.net/wmts/1.0/wmtsGetCapabilities_response.xsd" version="1.0.0">
"#);

    // Service Identification
    xml.push_str(
        r#"  <ows:ServiceIdentification>
    <ows:Title>TileServer RS</ows:Title>
    <ows:ServiceType>OGC WMTS</ows:ServiceType>
    <ows:ServiceTypeVersion>1.0.0</ows:ServiceTypeVersion>
  </ows:ServiceIdentification>
"#,
    );

    // Operations Metadata - include key in WMTS URL
    let wmts_url = format!("{}/styles/{}/wmts.xml{}", base_url, style_id, key_query);
    write!(
        xml,
        r#"  <ows:OperationsMetadata>
    <ows:Operation name="GetCapabilities">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="{}">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>RESTful</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
    <ows:Operation name="GetTile">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="{}">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>RESTful</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
  </ows:OperationsMetadata>
"#,
        wmts_url, wmts_url
    )
    .unwrap();

    // Contents section
    xml.push_str("  <Contents>\n");

    // Layer for 256px tiles
    write_layer(&mut xml, base_url, style_id, style_name, 256, &key_query);

    // Layer for 512px tiles (using @2x)
    write_layer(&mut xml, base_url, style_id, style_name, 512, &key_query);

    // TileMatrixSets
    write_tile_matrix_set_google_maps(&mut xml, 256, min_zoom, max_zoom);
    write_tile_matrix_set_google_maps(&mut xml, 512, min_zoom, max_zoom);

    xml.push_str("  </Contents>\n");

    // Service Metadata URL
    writeln!(xml, r#"  <ServiceMetadataURL xlink:href="{}"/>"#, wmts_url).unwrap();

    xml.push_str("</Capabilities>\n");

    xml
}

/// Write a Layer element for a specific tile size
fn write_layer(
    xml: &mut String,
    base_url: &str,
    style_id: &str,
    style_name: &str,
    tile_size: u16,
    key_query: &str,
) {
    let layer_id = format!("{}-{}", style_id, tile_size);
    let layer_title = format!("{}-{}", style_name, tile_size);
    let matrix_set = format!("GoogleMapsCompatible_{}", tile_size);

    // Build tile URL template with optional key query parameter
    // For 256px: /styles/{id}/{z}/{x}/{y}.png?key=...
    // For 512px: /styles/{id}/{z}/{x}/{y}@2x.png?key=... (which renders at 512px)
    let tile_template = if tile_size == 256 {
        format!(
            "{}/styles/{}/{{TileMatrix}}/{{TileCol}}/{{TileRow}}.png{}",
            base_url, style_id, key_query
        )
    } else {
        // 512px tiles use @2x scale factor
        format!(
            "{}/styles/{}/{{TileMatrix}}/{{TileCol}}/{{TileRow}}@2x.png{}",
            base_url, style_id, key_query
        )
    };

    write!(
        xml,
        r#"    <Layer>
      <ows:Title>{}</ows:Title>
      <ows:Identifier>{}</ows:Identifier>
      <ows:WGS84BoundingBox crs="urn:ogc:def:crs:OGC:2:84">
        <ows:LowerCorner>-180 -85.051128779807</ows:LowerCorner>
        <ows:UpperCorner>180 85.051128779807</ows:UpperCorner>
      </ows:WGS84BoundingBox>
      <Style isDefault="true">
        <ows:Identifier>default</ows:Identifier>
      </Style>
      <Format>image/png</Format>
      <TileMatrixSetLink>
        <TileMatrixSet>{}</TileMatrixSet>
      </TileMatrixSetLink>
      <ResourceURL format="image/png" resourceType="tile" template="{}"/>
    </Layer>
"#,
        layer_title, layer_id, matrix_set, tile_template
    )
    .unwrap();
}

/// Write a TileMatrixSet for Google Maps Compatible (EPSG:3857)
fn write_tile_matrix_set_google_maps(xml: &mut String, tile_size: u16, min_zoom: u8, max_zoom: u8) {
    let identifier = format!("GoogleMapsCompatible_{}", tile_size);

    write!(
        xml,
        r#"    <TileMatrixSet>
      <ows:Title>{}</ows:Title>
      <ows:Abstract>{} EPSG:3857</ows:Abstract>
      <ows:Identifier>{}</ows:Identifier>
      <ows:SupportedCRS>urn:ogc:def:crs:EPSG::3857</ows:SupportedCRS>
"#,
        identifier, identifier, identifier
    )
    .unwrap();

    // Write TileMatrix for each zoom level
    let max_z = (max_zoom as usize).min(SCALE_DENOMINATORS_256.len() - 1);
    for (z, &base_scale) in SCALE_DENOMINATORS_256
        .iter()
        .enumerate()
        .take(max_z + 1)
        .skip(min_zoom as usize)
    {
        // For 512px tiles, scale denominator is halved (same geographic extent, double pixels)
        let scale = if tile_size == 512 {
            base_scale / 2.0
        } else {
            base_scale
        };

        let matrix_size = 1u32 << z; // 2^z

        write!(
            xml,
            r#"      <TileMatrix>
        <ows:Identifier>{}</ows:Identifier>
        <ScaleDenominator>{}</ScaleDenominator>
        <TopLeftCorner>-20037508.34 20037508.34</TopLeftCorner>
        <TileWidth>{}</TileWidth>
        <TileHeight>{}</TileHeight>
        <MatrixWidth>{}</MatrixWidth>
        <MatrixHeight>{}</MatrixHeight>
      </TileMatrix>
"#,
            z, scale, tile_size, tile_size, matrix_size, matrix_size
        )
        .unwrap();
    }

    xml.push_str("    </TileMatrixSet>\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wmts_capabilities() {
        let xml = generate_wmts_capabilities(
            "http://localhost:8080",
            "osm-bright",
            "OSM Bright",
            0,
            18,
            None,
        );

        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("OGC WMTS"));
        assert!(xml.contains("osm-bright-256"));
        assert!(xml.contains("osm-bright-512"));
        assert!(xml.contains("GoogleMapsCompatible_256"));
        assert!(xml.contains("GoogleMapsCompatible_512"));
        assert!(xml.contains("http://localhost:8080/styles/osm-bright/wmts.xml"));
        // Without key, URLs should not have query params
        assert!(!xml.contains("?key="));
    }

    #[test]
    fn test_generate_wmts_capabilities_with_key() {
        let xml = generate_wmts_capabilities(
            "http://localhost:8080",
            "osm-bright",
            "OSM Bright",
            0,
            18,
            Some("my_api_key_123"),
        );

        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("OGC WMTS"));

        // With key, all URLs should include the key query parameter
        assert!(xml.contains("?key=my_api_key_123"));

        // WMTS URL should include key
        assert!(xml.contains("http://localhost:8080/styles/osm-bright/wmts.xml?key=my_api_key_123"));

        // Tile URLs should include key
        assert!(xml.contains("{TileRow}.png?key=my_api_key_123"));
        assert!(xml.contains("{TileRow}@2x.png?key=my_api_key_123"));
    }

    #[test]
    fn test_generate_wmts_capabilities_with_special_chars_key() {
        let xml = generate_wmts_capabilities(
            "http://localhost:8080",
            "osm-bright",
            "OSM Bright",
            0,
            18,
            Some("key with spaces & symbols="),
        );

        // Key should be URL-encoded
        assert!(xml.contains("?key=key%20with%20spaces%20%26%20symbols%3D"));
    }
}
