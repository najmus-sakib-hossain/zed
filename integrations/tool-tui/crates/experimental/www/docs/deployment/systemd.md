
# Systemd Service Configuration

This guide covers deploying dx-www applications as a systemd service on Linux.

## Systemd Unit File

Create `/etc/systemd/system/dx-www.service`:
```ini
[Unit]
Description=dx-www Server Documentation=https://github.com/example/dx-www After=network.target postgresql.service Wants=network-online.target
[Service]
Type=simple User=dxwww Group=dxwww WorkingDirectory=/opt/dx-www


# Environment configuration


EnvironmentFile=/etc/dx-www/environment Environment=RUST_LOG=info Environment=DX_ENV=production


# Binary path


ExecStart=/opt/dx-www/bin/dx-www-server


# Restart configuration


Restart=always RestartSec=5 StartLimitIntervalSec=60 StartLimitBurst=3


# Security hardening


NoNewPrivileges=true ProtectSystem=strict ProtectHome=true PrivateTmp=true PrivateDevices=true ProtectKernelTunables=true ProtectKernelModules=true ProtectControlGroups=true RestrictRealtime=true RestrictSUIDSGID=true MemoryDenyWriteExecute=true LockPersonality=true


# Resource limits


LimitNOFILE=65535 LimitNPROC=4096


# Logging


StandardOutput=journal StandardError=journal SyslogIdentifier=dx-www
[Install]
WantedBy=multi-user.target ```


## Service Management Commands


```bash

# Reload systemd after creating/modifying unit file

sudo systemctl daemon-reload

# Enable service to start on boot

sudo systemctl enable dx-www

# Start the service

sudo systemctl start dx-www

# Stop the service

sudo systemctl stop dx-www

# Restart the service

sudo systemctl restart dx-www

# Check service status

sudo systemctl status dx-www

# View logs

sudo journalctl -u dx-www -f

# View logs since last boot

sudo journalctl -u dx-www -b

# View last 100 lines

sudo journalctl -u dx-www -n 100 ```

## Installation Steps

- Create the service user:
```bash
sudo useradd -r -s /bin/false -d /opt/dx-www dxwww ```
- Create directories:
```bash
sudo mkdir -p /opt/dx-www/bin sudo mkdir -p /opt/dx-www/dist sudo mkdir -p /etc/dx-www sudo mkdir -p /var/log/dx-www ```
- Copy the binary:
```bash
sudo cp target/release/dx-www-server /opt/dx-www/bin/ sudo chmod +x /opt/dx-www/bin/dx-www-server ```
- Create environment file `/etc/dx-www/environment`:
```bash
DX_BIND_ADDRESS=127.0.0.1:3000 DX_AUTH_SECRET=your-secret-key-here DATABASE_URL=postgres://user:pass@localhost/dxwww DX_LOG_LEVEL=info ```
- Set permissions:
```bash
sudo chown -R dxwww:dxwww /opt/dx-www sudo chown -R dxwww:dxwww /var/log/dx-www sudo chmod 600 /etc/dx-www/environment ```
- Install and start:
```bash
sudo cp dx-www.service /etc/systemd/system/ sudo systemctl daemon-reload sudo systemctl enable dx-www sudo systemctl start dx-www ```

## Logging Configuration

### Journald Configuration

Edit `/etc/systemd/journald.conf`:
```ini
[Journal]
Storage=persistent Compress=yes SystemMaxUse=1G MaxRetentionSec=1month ```


### Log Rotation


t:0(/var/log/dx-www/*.log,{)[]


## Health Monitoring


Create a simple health check script `/opt/dx-www/bin/health-check.sh`:
```bash

#!/bin/bash

curl -sf http://localhost:3000/health > /dev/null exit $?
```
Add a systemd timer for periodic health checks: `/etc/systemd/system/dx-www-health.timer`:
```ini
[Unit]
Description=dx-www Health Check Timer
[Timer]
OnBootSec=1min OnUnitActiveSec=1min
[Install]
WantedBy=timers.target ```
`/etc/systemd/system/dx-www-health.service`:
```ini
[Unit]
Description=dx-www Health Check
[Service]
Type=oneshot ExecStart=/opt/dx-www/bin/health-check.sh ```


## Troubleshooting



### Service won't start


```bash

# Check for errors

sudo journalctl -u dx-www -n 50 --no-pager

# Check binary permissions

ls -la /opt/dx-www/bin/dx-www-server

# Test binary directly

sudo -u dxwww /opt/dx-www/bin/dx-www-server ```

### Permission denied errors

```bash


# Check SELinux (if enabled)


sudo ausearch -m avc -ts recent


# Allow network binding


sudo setsebool -P httpd_can_network_connect 1 ```


### Port already in use


```bash

# Find process using port

sudo ss -tlnp | grep 3000

# Kill process if needed

sudo fuser -k 3000/tcp ```
