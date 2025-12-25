---
seo:
  title: Tileserver RS - High-performance Vector Tile Server
  description: High-performance vector tile server built in Rust. Serve PMTiles and MBTiles with raster rendering and static map images.
---

::u-page-hero
#title
Tileserver RS

#description
High-performance vector tile server built in Rust. Serve PMTiles and MBTiles with raster rendering and embeddable static map images.

#links
  :::u-button
  ---
  color: neutral
  size: xl
  to: /getting-started/installation
  trailing-icon: i-lucide-arrow-right
  ---
  Get started
  :::

  :::u-button
  ---
  color: neutral
  icon: i-simple-icons-github
  size: xl
  to: https://github.com/vinayakkulkarni/tileserver-rs
  variant: outline
  ---
  View on GitHub
  :::
::

::u-page-section
#title
Features

#features
  :::u-page-feature
  ---
  icon: i-simple-icons-rust
  ---
  #title
  Built in Rust

  #description
  High-performance tile serving with the safety and speed of Rust and Axum.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-map
  ---
  #title
  PMTiles Support

  #description
  Cloud-optimized tile archives with HTTP range request support.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-database
  ---
  #title
  MBTiles Support

  #description
  SQLite-based tile storage for easy local development.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-layers
  ---
  #title
  MapLibre GL JS

  #description
  Built-in map viewer and data inspector powered by MapLibre GL JS.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-globe
  ---
  #title
  TileJSON 3.0

  #description
  Full TileJSON metadata API for seamless integration.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-image
  ---
  #title
  Raster Rendering

  #description
  Generate PNG/JPEG/WebP tiles from vector styles using headless Chrome.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-camera
  ---
  #title
  Static Map Images

  #description
  Create embeddable map screenshots like Mapbox/Maptiler static API.
  :::

  :::u-page-feature
  ---
  icon: i-simple-icons-docker
  ---
  #title
  Docker Ready

  #description
  Easy deployment with Docker Compose v2 (includes Chromium).
  :::
::
