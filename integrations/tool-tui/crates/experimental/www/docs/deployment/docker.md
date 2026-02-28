
# Docker Deployment Guide

This guide covers deploying dx-www applications using Docker.

## Production-Optimized Dockerfile

Create a `Dockerfile` in your project root:
```dockerfile


# Build stage


FROM rust:1.75-slim-bookworm AS builder


# Install build dependencies


RUN apt-get update && apt-get install -y \ pkg-config \ libssl-dev \ && rm -rf /var/lib/apt/lists/* WORKDIR /app


# Copy manifests first for better caching


COPY Cargo.toml Cargo.lock ./ COPY server/Cargo.toml server/ COPY core/Cargo.toml core/ COPY auth/Cargo.toml auth/ COPY query/Cargo.toml query/ COPY sync/Cargo.toml sync/ COPY packet/Cargo.toml packet/


# Create dummy source files for dependency caching


RUN mkdir -p server/src core/src auth/src query/src sync/src packet/src && \ echo "fn main() {}" > server/src/main.rs && \ echo "" > server/src/lib.rs && \ echo "" > core/src/lib.rs && \ echo "" > auth/src/lib.rs && \ echo "" > query/src/lib.rs && \ echo "" > sync/src/lib.rs && \ echo "" > packet/src/lib.rs


# Build dependencies only


RUN cargo build --release -p dx-www-server --features "auth,query,sync"


# Copy actual source code


COPY . .


# Touch source files to invalidate cache


RUN touch server/src/main.rs server/src/lib.rs


# Build the application


RUN cargo build --release -p dx-www-server --features "auth,query,sync"


# Runtime stage


FROM debian:bookworm-slim


# Install runtime dependencies


RUN apt-get update && apt-get install -y \ ca-certificates \ libssl3 \ && rm -rf /var/lib/apt/lists/*


# Create non-root user


RUN useradd -r -s /bin/false dxwww WORKDIR /app


# Copy binary from builder


COPY --from=builder /app/target/release/dx-www-server /app/


# Copy static assets (if any)


COPY --from=builder /app/dist /app/dist


# Set ownership


RUN chown -R dxwww:dxwww /app USER dxwww


# Expose port


EXPOSE 3000


# Health check


HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \ CMD curl -f http://localhost:3000/health || exit 1


# Run the server


CMD ["./dx-www-server"]
```

## Multi-Stage Build Process

The Dockerfile uses a multi-stage build for optimal image size: -Builder Stage: Uses the full Rust toolchain to compile the application -Runtime Stage: Uses a minimal Debian image with only runtime dependencies Benefits: -Final image is ~50MB instead of ~1GB -No build tools in production image -Faster deployment and startup

## Docker Compose Example

Create a `docker-compose.yml` for local development and production:
```yaml
version: '3.8' services:
dx-server:
build:
context: .
dockerfile: Dockerfile ports:
- "3000:3000"
environment:
- DX_ENV=production
- DX_LOG_LEVEL=info
- DX_BIND_ADDRESS=0.0.0.0:3000
- DX_AUTH_SECRET=${DX_AUTH_SECRET}
- DATABASE_URL=${DATABASE_URL}
volumes:
- ./dist:/app/dist:ro
restart: unless-stopped healthcheck:
test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
interval: 30s timeout: 3s retries: 3 networks:
- dx-network


# Optional: PostgreSQL database


postgres:
image: postgres:16-alpine environment:
- POSTGRES_USER=dxwww
- POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
- POSTGRES_DB=dxwww
volumes:
- postgres_data:/var/lib/postgresql/data
networks:
- dx-network


# Optional: Redis for caching/sessions


redis:
image: redis:7-alpine command: redis-server --appendonly yes volumes:
- redis_data:/data
networks:
- dx-network
networks:
dx-network:
driver: bridge volumes:
postgres_data:
redis_data:
```

## Building and Running

```bash


# Build the image


docker build -t dx-www-server:latest .


# Run the container


docker run -d \
- name dx-server \
- p 3000:3000 \
- e DX_ENV=production \
- e DX_AUTH_SECRET=your-secret-key \
dx-www-server:latest


# Using docker-compose


docker-compose up -d


# View logs


docker-compose logs -f dx-server


# Stop services


docker-compose down ```


## Production Considerations



### Security


- Always use non-root user in containers
- Set `DX_AUTH_SECRET` via environment variable, not in Dockerfile
- Use Docker secrets for sensitive data in Swarm mode
- Scan images for vulnerabilities: `docker scan dx-www-server:latest`


### Performance


- Use `--cpus` and `--memory` flags to limit resources
- Enable BuildKit for faster builds: `DOCKER_BUILDKIT=1 docker build`
- Use `.dockerignore` to exclude unnecessary files


### Monitoring


- The health check endpoint `/health` is used by Docker
- Export metrics to Prometheus via `/metrics` endpoint
- Use `docker stats` for basic resource monitoring ##.dockerignore Create a `.dockerignore` file:
```
target/ .git/ .gitignore *.md
!README.md Dockerfile* docker-compose* .env* *.log node_modules/ ```
