---
title: Home
navigation: false
layout: page
main:
  fluid: false
---

:ellipsis{right=0px width=75% blur=150px}

::block-hero
---
cta:
  - Get started
  - /getting-started/installation
secondary:
  - Open on GitHub
  - https://github.com/vinayakkulkarni/tileserver-rs
---

#title
Tileserver RS

#description
High-performance vector tile server built in [Rust]{style="color: var(--color-primary-500)"}, designed to serve PMTiles and MBTiles with ease.

#extra
  ::list
  - **PMTiles Support** - Serve tiles from local and remote PMTiles archives
  - **MBTiles Support** - Serve tiles from SQLite-based MBTiles files
  - **TileJSON 3.0** - Full TileJSON metadata API
  - **MapLibre GL JS** - Built-in map viewer and data inspector
  - **Docker Ready** - Easy deployment with Docker Compose v2
  - **Fast** - Built in Rust with Axum for maximum performance
  ::

#support
  ::terminal
  ---
  content:
  - docker compose up -d
  ---
  ::
::

::card-grid
#title
Features

#root
:ellipsis{left=0px width=40rem top=10rem blur=140px}

#default
  ::card{icon=logos:rust}
  #title
  Built in Rust
  #description
  High-performance tile serving with the safety and speed of Rust and Axum.
  ::

  ::card{icon=simple-icons:maplibre}
  #title
  MapLibre GL JS
  #description
  Built-in map viewer and data inspector powered by MapLibre GL JS.
  ::

  ::card{icon=logos:docker-icon}
  #title
  Docker Ready
  #description
  Easy deployment with Docker Compose v2 and multi-stage builds.
  ::

  ::card{icon=heroicons-outline:cube-transparent}
  #title
  PMTiles
  #description
  Cloud-optimized tile archives with HTTP range request support.
  ::

  ::card{icon=heroicons-outline:database}
  #title
  MBTiles
  #description
  SQLite-based tile storage for easy local development.
  ::

  ::card{icon=heroicons-outline:globe-alt}
  #title
  TileJSON 3.0
  #description
  Full TileJSON metadata API for seamless integration.
  ::
::
