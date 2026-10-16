#!/usr/bin/env bash
# Build data/raster/benchmark-rgb.cog.tif, the world RGB COG used by the
# `cog` benchmark (tileserver-rs `cog-rgb` source and titiler).
#
# Source: Natural Earth II 1:50m shaded relief with water (public domain),
# reprojected to Web Mercator (EPSG:3857) and written as a JPEG-compressed COG
# with internal overviews. Requires GDAL >= 3.1 (COG driver) and curl.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/data/raster/benchmark-rgb.cog.tif"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

URL="https://naciscdn.org/naturalearth/50m/raster/NE2_50M_SR_W.zip"

echo "Downloading $URL"
curl -fL --retry 3 -o "$WORK/ne.zip" "$URL"
unzip -q "$WORK/ne.zip" -d "$WORK"
SRC="$(find "$WORK" -name 'NE2_50M_SR_W.tif' | head -1)"

echo "Reprojecting to EPSG:3857"
gdalwarp -q -t_srs EPSG:3857 \
  -te -20037508.34 -20037508.34 20037508.34 20037508.34 \
  -r bilinear -multi -wo NUM_THREADS=ALL_CPUS \
  -co TILED=YES -co BIGTIFF=IF_SAFER \
  "$SRC" "$WORK/mercator.tif"

echo "Writing COG to $OUT"
mkdir -p "$(dirname "$OUT")"
gdal_translate -q -of COG \
  -co COMPRESS=JPEG -co QUALITY=90 \
  -co OVERVIEWS=IGNORE_EXISTING -co RESAMPLING=AVERAGE \
  -co BLOCKSIZE=512 -co NUM_THREADS=ALL_CPUS \
  "$WORK/mercator.tif" "$OUT"

gdalinfo "$OUT" | grep -E "Size is|LAYOUT|Overviews" || true
ls -lh "$OUT"
