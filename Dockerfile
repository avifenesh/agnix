# Multi-stage build for minimal image size
FROM rust:1.83-alpine AS builder

# Install musl-dev for static linking
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY knowledge-base ./knowledge-base

# Build release binary with static linking
RUN cargo build --release --bin agnix \
    && strip /build/target/release/agnix

# Runtime stage - minimal image
FROM alpine:3.21

# Add ca-certificates for any future HTTPS needs
RUN apk add --no-cache ca-certificates

# Copy binary from builder
COPY --from=builder /build/target/release/agnix /usr/local/bin/agnix

# Set working directory for mounted volumes
WORKDIR /workspace

# Default command validates current directory
ENTRYPOINT ["agnix"]
CMD ["."]
