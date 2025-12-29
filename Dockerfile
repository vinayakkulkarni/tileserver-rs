# =============================================================================
# Stage 1: Build MapLibre Native (C++ library)
# =============================================================================
FROM ubuntu:24.04 AS maplibre-builder

ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies for MapLibre Native
# Using clang as required by MapLibre Native CMake presets
RUN apt-get update && apt-get install -y --no-install-recommends --no-install-suggests \
    build-essential \
    clang \
    cmake \
    ninja-build \
    ccache \
    pkg-config \
    git \
    curl \
    ca-certificates \
    # Core libraries
    libcurl4-openssl-dev \
    libglfw3-dev \
    libuv1-dev \
    libpng-dev \
    libicu-dev \
    libjpeg-turbo8-dev \
    libwebp-dev \
    libsqlite3-dev \
    # OpenGL/EGL
    xvfb \
    libopengl-dev \
    libgl-dev \
    libegl-dev \
    # X11 (required for linux-opengl preset)
    libx11-dev \
    libxrandr-dev \
    libxinerama-dev \
    libxcursor-dev \
    libxi-dev \
    # Wayland (optional but included)
    libwayland-dev \
    libxkbcommon-dev \
    wayland-protocols \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy MapLibre Native source
COPY maplibre-native-sys/vendor/maplibre-native ./maplibre-native

# Build MapLibre Native for Linux using the official preset
# Build all static libraries required for linking
WORKDIR /build/maplibre-native
RUN cmake --preset linux-opengl \
    -DCMAKE_BUILD_TYPE=Release \
    -DMLN_WITH_WERROR=OFF \
    && cmake --build build-linux-opengl -j$(nproc)

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
# Use Ubuntu 24.04 to match glibc version with maplibre-builder stage
# (Ubuntu 24.04 has glibc 2.39 with C23 functions like __isoc23_strtol)
FROM ubuntu:24.04 AS rust-builder

ENV DEBIAN_FRONTEND=noninteractive

# Install Rust and deps needed for linking
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Required for rustup
    ca-certificates \
    curl \
    build-essential \
    # Core libraries
    libcurl4-openssl-dev \
    libpng-dev \
    libicu-dev \
    libjpeg-dev \
    libwebp-dev \
    libsqlite3-dev \
    libuv1-dev \
    libglfw3-dev \
    # OpenGL/EGL
    libopengl-dev \
    libgl-dev \
    libegl-dev \
    # X11 (required for linking)
    libx11-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    # Install Rust via rustup
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Copy MapLibre Native build artifacts (from linux-opengl preset)
# build.rs expects 'build-linux' directory, so copy to that name
COPY --from=maplibre-builder /build/maplibre-native/build-linux-opengl /app/maplibre-native-sys/vendor/maplibre-native/build-linux

# Copy MapLibre Native headers (needed for build.rs)
COPY maplibre-native-sys ./maplibre-native-sys

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source file for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Copy the embedded SPA
COPY --from=node-builder /app/apps/client/.output/public ./apps/client/.output/public

# Optional features (default: postgres enabled for PostGIS function sources)
ARG FEATURES="postgres"

# Build dependencies only (may fail on first try, that's ok)
RUN if [ -n "$FEATURES" ]; then \
      cargo build --release --features "$FEATURES" 2>/dev/null || true; \
    else \
      cargo build --release 2>/dev/null || true; \
    fi
RUN rm -rf src

# Copy actual source code
COPY src ./src

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
    apt-get install -y --no-install-recommends --no-install-suggests \
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
    libx11-6 \
    libegl1 \
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
