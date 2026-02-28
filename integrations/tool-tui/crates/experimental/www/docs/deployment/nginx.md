
# Nginx Reverse Proxy Configuration

This guide covers configuring Nginx as a reverse proxy for dx-www applications.

## Basic Configuration

Create `/etc/nginx/sites-available/dx-www`:
```nginx
upstream dx_backend { server 127.0.0.1:3000;
keepalive 32;
}
server { listen 80;
server_name example.com www.example.com;


# Redirect HTTP to HTTPS


return 301 https://$server_name$request_uri;
}
server { listen 443 ssl http2;
server_name example.com www.example.com;


# SSL Configuration


ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;
ssl_session_timeout 1d;
ssl_session_cache shared:SSL:50m;
ssl_session_tickets off;


# Modern SSL configuration


ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
ssl_prefer_server_ciphers off;


# HSTS (handled by dx-www, but can be added here too)


add_header Strict-Transport-Security "max-age=63072000" always;


# Logging


access_log /var/log/nginx/dx-www.access.log;
error_log /var/log/nginx/dx-www.error.log;


# Gzip compression (dx-www also compresses, but nginx can handle static files)


gzip on;
gzip_vary on;
gzip_proxied any;
gzip_comp_level 6;
gzip_types text/plain text/css text/xml application/json application/javascript application/xml;


# Client body size limit


client_max_body_size 10M;


# Proxy settings


location / { proxy_pass http://dx_backend;
proxy_http_version 1.1;
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";
proxy_set_header Host $host;
proxy_set_header X-Real-IP $remote_addr;
proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
proxy_set_header X-Forwarded-Proto $scheme;
proxy_set_header X-Request-ID $request_id;


# Timeouts


proxy_connect_timeout 60s;
proxy_send_timeout 60s;
proxy_read_timeout 60s;


# Buffering


proxy_buffering on;
proxy_buffer_size 4k;
proxy_buffers 8 4k;
}


# WebSocket support


location /ws { proxy_pass http://dx_backend;
proxy_http_version 1.1;
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";
proxy_set_header Host $host;
proxy_set_header X-Real-IP $remote_addr;
proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
proxy_set_header X-Forwarded-Proto $scheme;


# WebSocket timeouts


proxy_connect_timeout 7d;
proxy_send_timeout 7d;
proxy_read_timeout 7d;
}


# Binary streaming endpoint (disable buffering for streaming)


location /stream/ { proxy_pass http://dx_backend;
proxy_http_version 1.1;
proxy_set_header Host $host;
proxy_set_header X-Real-IP $remote_addr;
proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
proxy_set_header X-Forwarded-Proto $scheme;


# Disable buffering for streaming


proxy_buffering off;
proxy_cache off;
}


# Health check (for load balancers)


location /health { proxy_pass http://dx_backend;
proxy_http_version 1.1;
access_log off;
}


# Static files (if served separately)


location /static/ { alias /opt/dx-www/dist/static/;
expires 1y;
add_header Cache-Control "public, immutable";
}
}
```

## SSL/TLS Setup with Let's Encrypt

```bash


# Install certbot


sudo apt install certbot python3-certbot-nginx


# Obtain certificate


sudo certbot --nginx -d example.com -d www.example.com


# Auto-renewal is configured automatically



# Test renewal


sudo certbot renew --dry-run ```


## WebSocket Proxy Configuration


The configuration above includes WebSocket support. Key settings:
```nginx

# Required headers for WebSocket upgrade

proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";

# Long timeouts for persistent connections

proxy_connect_timeout 7d;
proxy_send_timeout 7d;
proxy_read_timeout 7d;
```


## Load Balancing


For multiple backend servers:
```nginx
upstream dx_backend { least_conn; # Use least connections algorithm server 127.0.0.1:3000 weight=5;
server 127.0.0.1:3001 weight=5;
server 127.0.0.1:3002 backup;
keepalive 32;
}
```


## Rate Limiting


Add rate limiting at the nginx level:
```nginx

# Define rate limit zones

limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
limit_req_zone $binary_remote_addr zone=auth:10m rate=5r/m;
server {

# Apply to API endpoints

location /api/ { limit_req zone=api burst=20 nodelay;
proxy_pass http://dx_backend;

# ... other settings

}

# Stricter limits for auth endpoints

location /api/auth/ { limit_req zone=auth burst=5 nodelay;
proxy_pass http://dx_backend;

# ... other settings

}
}
```


## Caching


Configure caching for static content:
```nginx

# Cache zone definition (in http block)

proxy_cache_path /var/cache/nginx/dx-www levels=1:2 keys_zone=dx_cache:10m max_size=1g inactive=60m;
server {

# Cache static assets

location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ { proxy_pass http://dx_backend;
proxy_cache dx_cache;
proxy_cache_valid 200 1d;
proxy_cache_use_stale error timeout updating;
add_header X-Cache-Status $upstream_cache_status;
}
}
```


## Testing Configuration


```bash

# Test nginx configuration

sudo nginx -t

# Reload nginx

sudo systemctl reload nginx

# Check status

sudo systemctl status nginx

# View access logs

sudo tail -f /var/log/nginx/dx-www.access.log

# View error logs

sudo tail -f /var/log/nginx/dx-www.error.log ```

## Security Headers

While dx-www adds security headers, you can add additional headers at the nginx level:
```nginx


# Additional security headers


add_header X-Frame-Options "SAMEORIGIN" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
```
