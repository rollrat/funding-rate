# Build stage
FROM rust:latest as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Add x86_64-unknown-linux-gnu target
RUN rustup target add x86_64-unknown-linux-gnu

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build for x86_64-unknown-linux-gnu target
RUN cargo build --release --target x86_64-unknown-linux-gnu

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/perp-scanner-server /usr/local/bin/perp-scanner-server

# Set the binary as executable
RUN chmod +x /usr/local/bin/perp-scanner-server

# Set working directory
WORKDIR /app

# Set entrypoint (users can pass arguments like: docker run perp-scanner-server)
ENTRYPOINT ["/usr/local/bin/perp-scanner-server"]

