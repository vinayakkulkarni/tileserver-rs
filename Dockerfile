# =============================================================================
# Stage 1: Build MapLibre Native (C++ library)
# =============================================================================
FROM ubuntu:jammy AS maplibre-builder

ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies for MapLibre Native (matches tileserver-gl)
RUN apt-get update && apt-get install -y --no-install-recommends --no-install-suggests \
    build-essential \
    cmake \
    ninja-build \
    ccache \
    pkg-config \
    git \
    curl \
    ca-certificates \
    libcurl4-openssl-dev \
    libglfw3-dev \
    libuv1-dev \
    libpng-dev \
    libicu-dev \
    libjpeg-turbo8-dev \
    libwebp-dev \
    libsqlite3-dev \
    xvfb \
    libopengl-dev \
    libgl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy MapLibre Native source
COPY maplibre-native-sys/vendor/maplibre-native ./maplibre-native

# Build MapLibre Native for Linux (headless OpenGL)
WORKDIR /build/maplibre-native
RUN cmake -B build-linux \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_CXX_COMPILER_LAUNCHER=ccache \
    -DMBGL_WITH_QT=OFF \
    -DMBGL_WITH_OPENGL=ON \
    -DMBGL_WITH_WERROR=OFF \
    && cmake --build build-linux --target mbgl-core mlt-cpp -j$(nproc)

# =============================================================================
# Stage 2: Build Nuxt frontend (SPA)
# =============================================================================
FROM oven/bun:1 AS node-builder

WORKDIR /app

# Copy workspace files
COPY package.json bun.lock ./
COPY apps/client ./apps/client

# Install dependencies
RUN bun install --frozen-lockfile

# Build the client as static SPA
RUN bun run --filter @tileserver-rs/client generate

# =============================================================================
# Stage 3: Build Rust backend
# =============================================================================
FROM rust:1.83-bookworm AS rust-builder

ENV DEBIAN_FRONTEND=noninteractive

# Install deps needed for linking (minimal set matching tileserver-gl runtime)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libcurl4-openssl-dev \
    libpng-dev \
    libicu-dev \
    libjpeg-dev \
    libwebp-dev \
    libsqlite3-dev \
    libuv1-dev \
    libglfw3-dev \
    libopengl-dev \
    libgl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy MapLibre Native build artifacts
COPY --from=maplibre-builder /build/maplibre-native/build-linux /app/maplibre-native-sys/vendor/maplibre-native/build-linux

# Copy MapLibre Native headers (needed for build.rs)
COPY maplibre-native-sys ./maplibre-native-sys

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source file for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Copy the embedded SPA
COPY --from=node-builder /app/apps/client/.output/public ./apps/client/.output/public

# Build dependencies only (may fail on first try, that's ok)
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual application
RUN touch src/main.rs && cargo build --release

# =============================================================================
# Stage 4: Runtime (minimal, matches tileserver-gl proven setup)
# =============================================================================
FROM ubuntu:jammy AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime dependencies (mirrors tileserver-gl for proven compatibility)
RUN apt-get update && \
    apt-get install -y --no-install-recommends --no-install-suggests \
    ca-certificates \
    curl \
    xvfb \
    libglfw3 \
    libuv1 \
    libjpeg-turbo8 \
    libicu70 \
    libcurl4 \
    libpng16-16 \
    libwebp7 \
    libsqlite3-0 \
    libopengl0 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Rust binary
COPY --from=rust-builder /app/target/release/tileserver-rs ./tileserver-rs

# Copy entrypoint script
COPY docker-entrypoint.sh ./docker-entrypoint.sh
RUN chmod +x ./docker-entrypoint.sh

# Copy example config
COPY config.example.toml ./config.toml

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
