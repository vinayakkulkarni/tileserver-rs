# Stage 1: Build Rust backend
FROM rust:1.83-bookworm AS rust-builder

WORKDIR /app

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source file for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual application
RUN touch src/main.rs && cargo build --release

# Stage 2: Build Nuxt frontend
FROM oven/bun:1 AS node-builder

WORKDIR /app

# Copy workspace files
COPY package.json bun.lockb ./
COPY apps/client ./apps/client

# Install dependencies
RUN bun install --frozen-lockfile

# Build the client
RUN bun run --filter @tileserver-rs/client build

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime

# Install required runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Rust binary
COPY --from=rust-builder /app/target/release/tileserver-rs ./tileserver-rs

# Copy Nuxt output
COPY --from=node-builder /app/apps/client/.output ./client/.output

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

# Run the server
CMD ["./tileserver-rs", "--config", "/app/config.toml"]
