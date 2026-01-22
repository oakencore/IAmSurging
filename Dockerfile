# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy source to cache dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/surge-server.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    cargo build --release --bin surge-server && \
    rm -rf src

# Copy actual source
COPY src ./src
COPY feedIds.json ./

# Build for release
RUN touch src/lib.rs src/bin/surge-server.rs && \
    cargo build --release --bin surge-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false surge

WORKDIR /app

# Copy binary and feed data
COPY --from=builder /app/target/release/surge-server /usr/local/bin/
COPY --from=builder /app/feedIds.json ./

# Set ownership
RUN chown -R surge:surge /app

USER surge

# Expose port
EXPOSE 9000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9000/health || exit 1

# Run the server
CMD ["surge-server"]
