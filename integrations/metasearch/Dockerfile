# --- Build stage ---
FROM rust:1.85-slim AS builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin metasearch

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/metasearch /usr/local/bin/metasearch
COPY config/ /app/config/
COPY templates/ /app/templates/
COPY static/ /app/static/

WORKDIR /app

EXPOSE 8888

CMD ["metasearch", "serve"]
