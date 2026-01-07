-- PostgreSQL/PostGIS initialization script for tileserver-rs benchmarks
-- Creates sample tables and tile functions for benchmark testing

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS postgis_topology;

-- =============================================================================
-- Sample data: Create a points table with random data for benchmarking
-- =============================================================================

CREATE TABLE IF NOT EXISTS benchmark_points (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100),
    category VARCHAR(50),
    geom GEOMETRY(Point, 4326)
);

-- Create spatial index
CREATE INDEX IF NOT EXISTS benchmark_points_geom_idx 
ON benchmark_points USING GIST (geom);

-- Insert random points in the Zurich area (matching our MBTiles test data)
-- Bounds: [8.45, 47.32, 8.63, 47.44]
INSERT INTO benchmark_points (name, category, geom)
SELECT 
    'Point ' || i,
    CASE (i % 5)
        WHEN 0 THEN 'restaurant'
        WHEN 1 THEN 'cafe'
        WHEN 2 THEN 'shop'
        WHEN 3 THEN 'park'
        ELSE 'other'
    END,
    ST_SetSRID(
        ST_MakePoint(
            8.45 + (random() * 0.18),  -- longitude: 8.45 to 8.63
            47.32 + (random() * 0.12)  -- latitude: 47.32 to 47.44
        ),
        4326
    )
FROM generate_series(1, 50000) AS i;

-- Update table statistics
ANALYZE benchmark_points;

-- =============================================================================
-- Tile function: Simple function returning MVT tiles
-- Signature: get_benchmark_tiles(z integer, x integer, y integer) RETURNS bytea
-- =============================================================================

CREATE OR REPLACE FUNCTION get_benchmark_tiles(z integer, x integer, y integer)
RETURNS bytea AS $$
DECLARE
    mvt bytea;
    bounds geometry;
    extent integer := 4096;
    buffer integer := 64;
BEGIN
    -- Get tile envelope
    bounds := ST_TileEnvelope(z, x, y);
    
    -- Generate MVT
    SELECT INTO mvt ST_AsMVT(tile, 'points', extent, 'geom') FROM (
        SELECT
            ST_AsMVTGeom(
                ST_Transform(geom, 3857),
                bounds,
                extent,
                buffer,
                true
            ) AS geom,
            id,
            name,
            category
        FROM benchmark_points
        WHERE ST_Transform(geom, 3857) && bounds
        LIMIT 10000  -- Limit features per tile for consistent performance
    ) AS tile
    WHERE geom IS NOT NULL;
    
    RETURN mvt;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE;

-- =============================================================================
-- Tile function with query parameters
-- Signature: get_filtered_tiles(z integer, x integer, y integer, query json) RETURNS bytea
-- =============================================================================

CREATE OR REPLACE FUNCTION get_filtered_tiles(z integer, x integer, y integer, query json)
RETURNS bytea AS $$
DECLARE
    mvt bytea;
    bounds geometry;
    extent integer := 4096;
    buffer integer := 64;
    category_filter text;
BEGIN
    -- Get tile envelope
    bounds := ST_TileEnvelope(z, x, y);
    
    -- Extract category filter from query params (if provided)
    category_filter := query->>'category';
    
    -- Generate MVT with optional filtering
    SELECT INTO mvt ST_AsMVT(tile, 'filtered_points', extent, 'geom') FROM (
        SELECT
            ST_AsMVTGeom(
                ST_Transform(geom, 3857),
                bounds,
                extent,
                buffer,
                true
            ) AS geom,
            id,
            name,
            category
        FROM benchmark_points
        WHERE ST_Transform(geom, 3857) && bounds
          AND (category_filter IS NULL OR category = category_filter)
        LIMIT 10000
    ) AS tile
    WHERE geom IS NOT NULL;
    
    RETURN mvt;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE;

-- =============================================================================
-- Out-DB Raster function with dynamic rescaling
-- Signature: get_raster_dynamic(z int, x int, y int, bounds geometry, params jsonb)
-- Returns: TABLE(filepath text, rescale_min float8, rescale_max float8)
-- =============================================================================

CREATE OR REPLACE FUNCTION get_raster_dynamic(
    z integer,
    x integer, 
    y integer,
    bounds geometry,
    params jsonb DEFAULT '{}'::jsonb
)
RETURNS TABLE(filepath text, rescale_min float8, rescale_max float8) AS $$
BEGIN
    RETURN QUERY
    SELECT
        '/data/raster/test-dem.cog.tif'::text AS filepath,
        (params->>'min')::float8 AS rescale_min,
        (params->>'max')::float8 AS rescale_max;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE;

-- =============================================================================
-- Verify setup
-- =============================================================================

DO $$
DECLARE
    point_count integer;
    pg_version text;
    postgis_version text;
BEGIN
    SELECT count(*) INTO point_count FROM benchmark_points;
    SELECT current_setting('server_version') INTO pg_version;
    SELECT PostGIS_Lib_Version() INTO postgis_version;
    
    RAISE NOTICE 'Benchmark database initialized:';
    RAISE NOTICE '  PostgreSQL version: %', pg_version;
    RAISE NOTICE '  PostGIS version: %', postgis_version;
    RAISE NOTICE '  Points inserted: %', point_count;
    RAISE NOTICE '  Functions created: get_benchmark_tiles, get_filtered_tiles, get_raster_dynamic';
END $$;
