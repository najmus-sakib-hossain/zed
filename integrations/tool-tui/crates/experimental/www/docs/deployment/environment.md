
# Environment Variable Reference

This document lists all configuration options for dx-www applications.

## Core Configuration

+----------+-------------+-------------+----------+
| Variable | Description | Default     | Required |
+==========+=============+=============+==========+
| `DX      | ENV`        | Environment | mode     |
+----------+-------------+-------------+----------+



## Authentication

+----------+-------------+---------+----------+
| Variable | Description | Default | Required |
+==========+=============+=========+==========+
| `DX      | AUTH        | SECRET` | Secret   |
+----------+-------------+---------+----------+



## Database

+-----------+-------------+------------+------------+
| Variable  | Description | Default    | Required   |
+===========+=============+============+============+
| `DATABASE | URL`        | PostgreSQL | connection |
+-----------+-------------+------------+------------+



## Security

+----------+-------------+---------+----------+
| Variable | Description | Default | Required |
+==========+=============+=========+==========+
| `DX      | CSRF        | SECRET` | CSRF     |
+----------+-------------+---------+----------+



## SSL/TLS

+----------+-------------+---------+----------+
| Variable | Description | Default | Required |
+==========+=============+=========+==========+
| `DX      | TLS         | CERT`   | Path     |
+----------+-------------+---------+----------+



## Caching

+----------+-------------+---------+----------+
| Variable | Description | Default | Required |
+==========+=============+=========+==========+
| `DX      | CACHE       | TTL`    | Default  |
+----------+-------------+---------+----------+



## WebSocket/Sync

+----------+-------------+---------+-----------+
| Variable | Description | Default | Required  |
+==========+=============+=========+===========+
| `DX      | WS          | PING    | INTERVAL` |
+----------+-------------+---------+-----------+



## Monitoring

+----------+-------------+----------+----------+
| Variable | Description | Default  | Required |
+==========+=============+==========+==========+
| `DX      | METRICS     | ENABLED` | Enable   |
+----------+-------------+----------+----------+



## Example.env File

```bash


# Core


DX_ENV=production DX_BIND_ADDRESS=0.0.0.0:3000 DX_LOG_LEVEL=info


# Authentication


DX_AUTH_SECRET=your-very-long-secret-key-at-least-32-bytes DX_AUTH_ACCESS_TTL=900 DX_AUTH_REFRESH_TTL=604800


# Database


DATABASE_URL=postgres://dxwww:password@localhost:5432/dxwww DX_DB_POOL_SIZE=20 DX_DB_TIMEOUT=30


# Security


DX_RATE_LIMIT_AUTH=5 DX_RATE_LIMIT_API=100


# Caching


REDIS_URL=redis://localhost:6379


# Monitoring


DX_METRICS_ENABLED=true ```


## Environment-Specific Defaults



### Development Mode (`DX_ENV=development`)


- Relaxed CSP headers (allows hot reload)
- HSTS disabled
- Detailed error messages
- Debug logging enabled
- Rate limiting relaxed


### Production Mode (`DX_ENV=production`)


- Strict CSP headers
- HSTS enabled with 1-year max-age
- Generic error messages (no internal details)
- Info-level logging
- Full rate limiting enabled


## Security Best Practices


- Never commit secrets to version control
- Use `.env` files locally
- Use environment variables or secrets management in production
- Generate strong secrets
```bash

# Generate a 32-byte secret

openssl rand -base64 32 ```
- Rotate secrets regularly
- Auth secrets should be rotated at least annually
- CSRF secrets can be rotated more frequently
- Use different secrets per environment
- Development, staging, and production should have unique secrets
- Restrict access to environment files
```bash
chmod 600 .env ```
