# Use the official Rust image as the build stage
FROM rust:slim AS builder

# Install OpenSSL development packages and pkg-config
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /app
COPY . .

# Build the application with release optimizations
RUN cargo build --release

# Use a smaller base image for the runtime
FROM debian:bookworm-slim

# Install OpenSSL and CA certificates which are required for HTTPS requests
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder to this new stage
WORKDIR /app
COPY --from=builder /app/target/release/reconned-instagram .

# Expose the port the app runs on
EXPOSE 8080

# Command to run the executable
CMD ["./reconned-instagram"]
