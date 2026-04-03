# =============================================================================
# Stage 1: Download pre-built MapLibre Native static libraries
# =============================================================================
FROM ubuntu:24.04 AS maplibre-downloader

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ARG TARGETARCH

# Extract mbgl-sys version from release-please manifest (immune to workspace version syncing)
COPY .release-please-manifest.json /tmp/release-manifest.json

WORKDIR /build
RUN MBGL_VERSION=$(grep 'mbgl-sys' /tmp/release-manifest.json | sed 's/.*: *"\(.*\)".*/\1/') && \
    case "$TARGETARCH" in \
      amd64) MBGL_TARGET="x86_64-unknown-linux-gnu" ;; \
      arm64) MBGL_TARGET="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported arch: $TARGETARCH" && exit 1 ;; \
    esac && \
    echo "Downloading mbgl-sys v${MBGL_VERSION} for ${MBGL_TARGET}..." && \
    curl -fSL "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/mbgl-sys-v${MBGL_VERSION}/mbgl-native-${MBGL_TARGET}.tar.gz" | tar xz

# =============================================================================
# Stage 2: Build Nuxt frontend (SPA) - skipped for headless builds
# =============================================================================
FROM oven/bun:1 AS node-builder

ARG FEATURES="frontend mlt"

# ulimit -s unlimited increases the POSIX thread stack size so Bun's JSC engine
# can handle the deep CJS resolver recursion caused by @mlc-ai/web-llm in Vite builds

WORKDIR /app

# Copy workspace files
COPY package.json bun.lock ./
COPY apps/client ./apps/client

# Only build frontend when the "frontend" feature is enabled
# For headless builds (FEATURES=""), create an empty output directory
# so the COPY --from=node-builder in the rust-builder stage still works
RUN if echo "$FEATURES" | grep -q "frontend"; then \
      bun install --filter '@tileserver-rs/client' && \
      cd apps/client && ulimit -s unlimited && bun run generate; \
    else \
      mkdir -p apps/client/.output/public; \
    fi

# =============================================================================
# Stage 3: Build Rust backend
# =============================================================================
# Use Ubuntu 24.04 to match glibc version with maplibre-builder stage
# (Ubuntu 24.04 has glibc 2.39 with C23 functions like __isoc23_strtol)
FROM ubuntu:24.04 AS rust-builder

ENV DEBIAN_FRONTEND=noninteractive

# Install Rust and deps needed for linking
RUN apt-get update && apt-get install -y --no-install-recommends --fix-missing \
    ca-certificates \
    curl \
    git \
    build-essential \
    pkg-config \
    libcurl4-openssl-dev \
    libpng-dev \
    libicu-dev \
    libjpeg-dev \
    libwebp-dev \
    libsqlite3-dev \
    libuv1-dev \
    libglfw3-dev \
    libgdal-dev \
    libclang-dev \
    libopengl-dev \
    libgl-dev \
    libegl-dev \
    libx11-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

ENV PATH="/root/.cargo/bin:${PATH}"
ENV MBGL_SYS_LIB_DIR=/app/lib

WORKDIR /app

COPY --from=maplibre-downloader /build/lib ./lib
COPY crates/mbgl-sys ./crates/mbgl-sys

# Copy Cargo files and build script for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source and bench files for dependency caching
RUN mkdir -p src benches && echo "fn main() {}" > src/main.rs && echo "fn main() {}" > benches/mlt.rs

# Copy the embedded SPA
COPY --from=node-builder /app/apps/client/.output/public ./apps/client/.output/public

# Features are now enabled by default in Cargo.toml (postgres, raster, mlt)
ARG FEATURES="frontend mlt"

# Build dependencies only (may fail on first try, that's ok)
RUN if [ -n "$FEATURES" ]; then \
      cargo build --release --features "$FEATURES" 2>/dev/null || true; \
    else \
      cargo build --release 2>/dev/null || true; \
    fi
RUN rm -rf src

# Copy actual source code and benchmarks
COPY src ./src
COPY benches ./benches

RUN touch src/main.rs && \
    if [ -n "$FEATURES" ]; then \
      cargo build --release --features "$FEATURES"; \
    else \
      cargo build --release; \
    fi

# =============================================================================
# Stage 4: Runtime (must match glibc version from build stage)
# =============================================================================
FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends --no-install-suggests --fix-missing \
    ca-certificates \
    curl \
    xvfb \
    libglfw3 \
    libuv1 \
    libjpeg-turbo8 \
    libicu74 \
    libcurl4t64 \
    libpng16-16t64 \
    libwebp7 \
    libsqlite3-0 \
    libopengl0 \
    libgles2 \
    libx11-6 \
    libegl1 \
    libgdal34t64 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Rust binary
COPY --from=rust-builder /app/target/release/tileserver-rs ./tileserver-rs

# Copy entrypoint script
COPY deploy/docker-entrypoint.sh ./docker-entrypoint.sh
RUN chmod +x ./docker-entrypoint.sh

# Copy example config
COPY configs/example.toml ./config.toml

# Create data directory
RUN mkdir -p /data

# Environment variables
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8080

# Expose port
EXPOSE 8080

# Volume for tile data
VOLUME ["/data"]

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Use entrypoint script to handle Xvfb setup
ENTRYPOINT ["./docker-entrypoint.sh"]
CMD ["./tileserver-rs", "--config", "/app/config.toml"]
