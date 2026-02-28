# Use the official Rust image as the build environment
FROM rust:1.91.1-slim as builder

# Set the working directory
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src

# Build the project in release mode
RUN cargo build --release --bin forge-cli

# Use a smaller base image for the runtime
FROM debian:bookworm-slim

# Install necessary runtime dependencies (if any)
# For this project, rusqlite is bundled, so no additional deps needed

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/forge-cli /usr/local/bin/forge-cli

# Set the binary as the entrypoint
ENTRYPOINT ["/usr/local/bin/forge-cli"]