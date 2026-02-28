
# Monitoring Setup Guide

This guide covers setting up monitoring for dx-www applications using Prometheus and Grafana.

## Prometheus Metrics Endpoint

dx-www exposes metrics at `/metrics` in Prometheus format.

### Available Metrics

+--------+------+-------------+
| Metric | Type | Description |
+========+======+=============+
| `dx    | http | requests    |
+--------+------+-------------+



## Prometheus Configuration

Add to `prometheus.yml`:
```yaml
global:
scrape_interval: 15s evaluation_interval: 15s scrape_configs:
- job_name: 'dx-www'
static_configs:
- targets: ['localhost:3000']
metrics_path: /metrics scheme: http


# If using multiple instances


- job_name: 'dx-www-cluster'
static_configs:
- targets:
- 'dx-www-1:3000'
- 'dx-www-2:3000'
- 'dx-www-3:3000'
alerting:
alertmanagers:
- static_configs:
- targets: ['localhost:9093']
rule_files:
- 'dx-www-alerts.yml'
```

## Alert Rules

Create `dx-www-alerts.yml`:
```yaml
groups:
- name: dx-www
rules:


# High error rate


- alert: HighErrorRate
expr: | sum(rate(dx_http_requests_total{status=~"5.."}[5m]))
/ sum(rate(dx_http_requests_total[5m])) > 0.05 for: 5m labels:
severity: critical annotations:
summary: High error rate detected description: Error rate is {{ $value | humanizePercentage }}


# High latency


- alert: HighLatency
expr: | histogram_quantile(0.95, sum(rate(dx_http_request_duration_seconds_bucket[5m])) by (le)
) > 1 for: 5m labels:
severity: warning annotations:
summary: High latency detected description: P95 latency is {{ $value }}s


# Service down


- alert: ServiceDown
expr: up{job="dx-www"} == 0 for: 1m labels:
severity: critical annotations:
summary: dx-www service is down description: Instance {{ $labels.instance }} is down


# High memory usage


- alert: HighMemoryUsage
expr: process_resident_memory_bytes > 1e9 for: 5m labels:
severity: warning annotations:
summary: High memory usage description: Memory usage is {{ $value | humanize1024 }}B


# Rate limiting triggered


- alert: RateLimitingActive
expr: rate(dx_rate_limit_hits_total[5m]) > 10 for: 5m labels:
severity: info annotations:
summary: Rate limiting is active description: {{ $value }} rate limit hits per second


# Database connection pool exhausted


- alert: DBPoolExhausted
expr: dx_db_pool_connections >= dx_db_pool_max_connections * 0.9 for: 5m labels:
severity: warning annotations:
summary: Database connection pool nearly exhausted description: Pool is {{ $value | humanizePercentage }} full ```


## Grafana Dashboard


Import this dashboard JSON or create manually:
```json
{ "title": "dx-www Dashboard", "panels": [ { "title": "Request Rate", "type": "graph", "targets": [ { "expr": "sum(rate(dx_http_requests_total[5m])) by (status)", "legendFormat": "{{status}}"
}
]
}, { "title": "Request Latency (P95)", "type": "graph", "targets": [ { "expr": "histogram_quantile(0.95, sum(rate(dx_http_request_duration_seconds_bucket[5m])) by (le))", "legendFormat": "P95"
}
]
}, { "title": "Active Connections", "type": "stat", "targets": [ { "expr": "dx_active_connections"
}
]
}, { "title": "WebSocket Connections", "type": "stat", "targets": [ { "expr": "dx_websocket_connections"
}
]
}, { "title": "Error Rate", "type": "gauge", "targets": [ { "expr": "sum(rate(dx_http_requests_total{status=~\"5..\"}[5m])) / sum(rate(dx_http_requests_total[5m]))"
}
]
}, { "title": "Cache Hit Rate", "type": "gauge", "targets": [ { "expr": "sum(rate(dx_cache_hits_total[5m])) / (sum(rate(dx_cache_hits_total[5m])) + sum(rate(dx_cache_misses_total[5m])))"
}
]
}
]
}
```


## Key Metrics to Monitor



### Request Metrics


- Request rate: Overall traffic volume
- Error rate: Percentage of 5xx responses
- Latency percentiles: P50, P95, P99 response times


### Resource Metrics


- Memory usage: Process memory consumption
- CPU usage: Process CPU utilization
- Connection count: Active HTTP and WebSocket connections


### Business Metrics


- Auth token issuance: Login activity
- Rate limit hits: Potential abuse or misconfiguration
- Cache hit rate: Caching effectiveness


### Database Metrics


- Query duration: Database performance
- Pool utilization: Connection pool health
- Query errors: Database issues


## Docker Compose with Monitoring


```yaml
version: '3.8' services:
dx-server:
build: .
ports:
- "3000:3000"
environment:
- DX_METRICS_ENABLED=true
prometheus:
image: prom/prometheus:latest ports:
- "9090:9090"
volumes:
- ./prometheus.yml:/etc/prometheus/prometheus.yml
- prometheus_data:/prometheus
command:
- '--config.file=/etc/prometheus/prometheus.yml'
- '--storage.tsdb.path=/prometheus'
grafana:
image: grafana/grafana:latest ports:
- "3001:3000"
volumes:
- grafana_data:/var/lib/grafana
environment:
- GF_SECURITY_ADMIN_PASSWORD=admin
volumes:
prometheus_data:
grafana_data:
```


## Health Check Endpoint


The `/health` endpoint returns:
```json
{ "status": "healthy", "version": "0.1.0", "uptime_seconds": 3600, "checks": { "database": "ok", "cache": "ok", "websocket": "ok"
}
}
```
Use this for: -Load balancer health checks -Kubernetes liveness/readiness probes -External monitoring services
