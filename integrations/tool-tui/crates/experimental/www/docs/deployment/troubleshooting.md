
# Troubleshooting Guide

This guide covers common issues and solutions for dx-www applications.

## Common Issues

### Server Won't Start

Symptom: Server fails to start or exits immediately. Possible Causes & Solutions: -Port already in use ```bash

# Check what's using the port

ss -tlnp | grep 3000

# or on Windows

netstat -ano | findstr :3000

# Kill the process or use a different port

DX_BIND_ADDRESS=0.0.0.0:3001 ./dx-www-server ```
- Missing environment variables
```bash


# Check required variables are set


echo dx-style READMEX_AUTH_SECRET echo dx-style READMEATABASE_URL


# Set them


export DX_AUTH_SECRET=$(openssl rand -base64 32)
```
- Permission denied
```bash


# Check binary permissions


ls -la ./dx-www-server chmod +x ./dx-www-server


# Check if running as correct user


whoami ```
- Database connection failed
```bash

# Test database connection

psql dx-style READMEATABASE_URL -c "SELECT 1"

# Check if database is running

systemctl status postgresql ```

### Authentication Issues

Symptom: Users can't log in or tokens are rejected. Solutions: -Token signature mismatch -Ensure `DX_AUTH_SECRET` is the same across all instances -Check if secret was rotated without invalidating old tokens -Token expired -Check server time is synchronized (NTP) -Verify `DX_AUTH_ACCESS_TTL` is reasonable -CORS issues ```bash

# Check CORS headers in response

curl -I -X OPTIONS http://localhost:3000/api/auth/login \
- H "Origin: http://example.com"
```


### WebSocket Connection Issues


Symptom: WebSocket connections fail or disconnect frequently. Solutions: -Proxy not configured for WebSocket -Ensure nginx has `proxy_set_header Upgrade` and `Connection` headers -Check proxy timeouts are long enough -Firewall blocking WebSocket ```bash


# Test WebSocket connection


wscat -c ws://localhost:3000/ws ```
- Too many connections
- Check `DX_WS_MAX_CONNECTIONS` limit
- Monitor connection count in metrics


### High Memory Usage


Symptom: Server uses excessive memory. Solutions: -Memory leak in handlers -Check for unbounded caches -Review message buffer sizes -Too many cached items -Reduce cache TTL -Implement cache eviction -Large request bodies -Set `client_max_body_size` in nginx -Implement request size limits


### Slow Response Times


Symptom: Requests take too long to complete. Solutions: -Database queries slow ```bash


# Check slow query log


tail -f /var/log/postgresql/postgresql-*-main.log


# Add indexes


CREATE INDEX idx_users_email ON users(email);
```
- Missing caching
- Enable query caching
- Add Redis for distributed caching
- Too many concurrent requests
- Increase worker threads
- Add load balancing

### Rate Limiting Issues

Symptom: Legitimate users getting rate limited. Solutions: -Limits too strict ```bash

# Increase limits

export DX_RATE_LIMIT_API=200 ```
- Shared IP (NAT)
- Use user-based rate limiting instead of IP
- Whitelist known IPs
- Rate limit store issues
- Check Redis connection
- Verify in-memory store isn't full

## Debugging Tips

### Enable Debug Logging

```bash


# Set log level to debug


export RUST_LOG=dx_www=debug export DX_LOG_LEVEL=debug


# Or for specific modules


export RUST_LOG=dx_www_server::auth=debug,dx_www_server::handlers=info ```


### Check Request Flow


```bash

# Trace a request

curl -v http://localhost:3000/api/endpoint \
- H "Authorization: Bearer $TOKEN" \
- H "X-Request-ID: test-123"

# Check logs for request ID

journalctl -u dx-www | grep "test-123"
```


### Inspect Server State


```bash

# Health check

curl http://localhost:3000/health | jq

# Metrics

curl http://localhost:3000/metrics | grep dx_

# Active connections

curl http://localhost:3000/metrics | grep dx_active_connections ```

### Database Debugging

```bash


# Check connection pool


curl http://localhost:3000/metrics | grep dx_db_pool


# Test query


psql dx-style READMEATABASE_URL -c "EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'test@example.com'"
```

## FAQ

### Q: How do I rotate the auth secret?

- Generate new secret: `openssl rand
- base64 32`
- Update environment variable
- Restart server (existing tokens will be invalidated)
- Users will need to re-authenticate

### Q: How do I scale horizontally?

- Ensure `DX_AUTH_SECRET` is shared across instances
- Use Redis for session/cache storage
- Use a load balancer (nginx, HAProxy)
- Ensure WebSocket sticky sessions if needed

### Q: How do I backup the database?

```bash


# PostgreSQL backup


pg_dump dx-style READMEATABASE_URL > backup.sql


# Restore


psql dx-style READMEATABASE_URL < backup.sql ```


### Q: How do I update to a new version?


- Build new binary
- Stop current service: `systemctl stop dx-www`
- Replace binary
- Start service: `systemctl start dx-www`
- Verify health: `curl //localhost:3000/health`


### Q: How do I handle SSL certificate renewal?


With Let's Encrypt and nginx:
```bash

# Certbot handles renewal automatically

# Test renewal

sudo certbot renew --dry-run

# Force renewal

sudo certbot renew --force-renewal ```

### Q: How do I debug CORS issues?

```bash


# Check preflight response


curl -X OPTIONS http://localhost:3000/api/endpoint \
- H "Origin: http://example.com" \
- H "Access-Control-Request-Method: POST" \
- H "Access-Control-Request-Headers: Content-Type" \
- v


# Look for these headers in response:



# Access-Control-Allow-Origin



# Access-Control-Allow-Methods



# Access-Control-Allow-Headers


```

## Getting Help

- Check the logs: `journalctl
- u dx-www
- f`
- Review metrics: `curl //localhost:3000/metrics`
- Enable debug logging
- Check GitHub issues
- Join the community Discord
