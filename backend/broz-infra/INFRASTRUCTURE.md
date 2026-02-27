# BROZ — Infrastructure & Deployment Guide

## Architecture Overview

```
Internet
    |
    +-- Cloudflare (Free) -- app.brozr.com / admin.brozr.com (SPA only, NO media)
    |
    +-- api.brozr.com -------> App Node (K3s + Traefik)
    |                              | private network 10.0.0.0/16
    |                              +-- broz-gateway -> microservices
    |                              +-- Redis, RabbitMQ, MinIO
    |                              |
    |                         DB Node (private network only, Phase 2+)
    |                              +-- PostgreSQL :5432
    |
    +-- sfu.brozr.com -------> SFU Node (direct public IP)
    |                              +-- :8080 (HTTP API via Caddy)
    |                              +-- :3478/UDP (TURN)
    |                              +-- :20000-20100/UDP (WebRTC media)
    |
    +-- media.brozr.com -----> Bunny CDN -> MinIO origin (user media)
```

## Hosting: Hetzner

Hetzner does NOT prohibit adult content. Content must be legal under German law.

**Cloudflare** is used ONLY for the SPA (JS/CSS/HTML), NOT for user media.
**Bunny CDN** (Volume tier, ~0.005 EUR/GB) handles media delivery via `media.brozr.com`.

## Scaling Phases

### Phase 1 — MVP (0-500 users, ~50 concurrent, 5-10 simultaneous calls)

| Server | Model | Specs | Price | Role |
|--------|-------|-------|-------|------|
| App Node | CX33 (Cloud) | 4 vCPU shared, 8 GB RAM, 80 GB SSD | 5.49 EUR | K3s mono-node |
| SFU Node | CPX21 (Cloud) | 3 vCPU dedicated, 4 GB RAM, 2 TB traffic | 9.49 EUR | LiveRelay standalone |

**Total: ~17 EUR/month**

### Phase 2 — Growth (500-5000 users, ~300 concurrent, 20-50 calls)

| Server | Model | Specs | Price | Role |
|--------|-------|-------|-------|------|
| App Node | CAX31 (ARM) | 8 vCPU Ampere, 16 GB RAM | 12.49 EUR | K3s services (replicas x2) |
| DB Node | CX33 (Cloud) | 4 vCPU, 8 GB RAM | 5.49 EUR | PostgreSQL dedicated |
| SFU Node | AX42 (Dedicated) | Ryzen 7 8c, 64 GB DDR5, unlimited traffic | ~49 EUR | LiveRelay + TURN |

**Total: ~74 EUR/month**

### Phase 3 — Scale (5000+ users, ~2000 concurrent, 100+ calls)

| Server | Model | Specs | Price | Role |
|--------|-------|-------|-------|------|
| App Node x2 | CAX41 (ARM) x2 | 16 vCPU, 32 GB RAM each | 24.49 EUR x2 | K3s HA multi-node |
| DB Node | AX42 (Dedicated) | Ryzen 7 8c, 64 GB ECC, NVMe RAID 1 | ~49 EUR | PostgreSQL dedicated |
| SFU Node | AX102 (Dedicated) | Ryzen 9 16c, 128 GB DDR5, unlimited traffic | ~110 EUR | LiveRelay high-capacity |

**Total: ~233 EUR/month**

### Migration Triggers

- **Phase 1 -> 2:** >15 regular simultaneous calls, App Node CPU >70%, slow PG queries
- **Phase 2 -> 3:** >50 simultaneous calls, need HA (zero-downtime), SFU >800 Mbps sustained

## Kubernetes Namespaces

| Namespace | Contents |
|-----------|----------|
| `broz-system` | Traefik ingress, storage classes |
| `broz-data` | PostgreSQL, Redis, RabbitMQ, MinIO |
| `broz-app` | 8 microservices (gateway, auth, user, matching, messaging, notification, moderation, analytics) |
| `broz-frontend` | Frontend SPA |
| `broz-monitoring` | Prometheus, Grafana |

## Resource Limits (Phase 1)

### Infrastructure

| Component | CPU req/limit | RAM req/limit |
|-----------|--------------|---------------|
| PostgreSQL 16 | 100m / 1000m | 256Mi / 1Gi |
| Redis 7 | 50m / 500m | 64Mi / 256Mi |
| RabbitMQ 3.13 | 100m / 500m | 256Mi / 512Mi |
| MinIO | 50m / 500m | 128Mi / 512Mi |
| Prometheus | 50m / 500m | 128Mi / 512Mi |
| Grafana | 50m / 250m | 128Mi / 256Mi |

### Microservices

| Service | CPU req/limit | RAM req/limit | Notes |
|---------|--------------|---------------|-------|
| broz-gateway | 50m / 200m | 32Mi / 128Mi | Stateless, Redis rate limiting |
| broz-auth | 100m / 500m | 64Mi / 256Mi | Argon2 = CPU-intensive |
| broz-user | 50m / 200m | 64Mi / 256Mi | MinIO uploads |
| broz-matching | 100m / 400m | 128Mi / 512Mi | Socket.IO in-memory state |
| broz-messaging | 100m / 300m | 128Mi / 512Mi | Socket.IO + DashMap calls |
| broz-analytics | 100m / 300m | 128Mi / 256Mi | High write throughput |
| broz-notification | 50m / 100m | 32Mi / 128Mi | Event-driven, lightweight |
| broz-moderation | 50m / 100m | 32Mi / 128Mi | Admin, low traffic |

## SFU Node (LiveRelay)

The SFU runs OUTSIDE K8s for these reasons:
- UDP ports 20000-20100 are incompatible with K8s CNI networking
- `set_nat_1to1_ips` requires the host's direct public IP
- Bandwidth isolation: call spikes must not impact the API
- WebRTC is ultra-sensitive to latency; container networking adds jitter

### SFU Resource Usage

| Metric | Per 1:1 call | 50 calls | 100 calls |
|--------|-------------|----------|-----------|
| RAM | ~10-15 MB | ~750 MB | ~1.5 GB |
| CPU | ~50-100m | 2.5-5 vCPU | 5-10 vCPU |
| Bandwidth | ~6-12 Mbps | 300-600 Mbps | 600 Mbps-1.2 Gbps |

**Bandwidth is the #1 limiting factor.** Cloud servers include 1-20 TB. Dedicated servers include UNLIMITED traffic (1 Gbps port). The Cloud -> Dedicated breakpoint for SFU is at ~15-20 simultaneous calls.

## Deployment

### Initial Setup (Phase 1)

```bash
# 1. Setup App Node (K3s + all services)
scp -r broz-infra/ root@APP_NODE_IP:/root/
ssh root@APP_NODE_IP
./broz-infra/scripts/setup-app-node.sh <SFU_PRIVATE_IP>

# 2. Build SFU binary
cd WEB_RTC && cargo build --release
scp target/release/liverelay root@SFU_NODE_IP:/tmp/

# 3. Setup SFU Node
scp broz-infra/scripts/setup-sfu-node.sh root@SFU_NODE_IP:/root/
ssh root@SFU_NODE_IP
./setup-sfu-node.sh sfu.brozr.com <API_KEY> <JWT_SECRET> <APP_PRIVATE_IP>
```

### Deploying Updates

```bash
# Deploy single service
./broz-infra/scripts/deploy.sh broz-auth --push

# Deploy all services
./broz-infra/scripts/deploy.sh all --push

# Deploy frontend
./broz-infra/scripts/deploy.sh frontend --push

# Build for ARM64 (Phase 2)
./broz-infra/scripts/deploy.sh all --arm64 --push
```

### SFU Updates

```bash
# Build locally
cd WEB_RTC && cargo build --release

# Deploy to SFU node
scp target/release/liverelay root@SFU_NODE_IP:/opt/liverelay/
ssh root@SFU_NODE_IP 'systemctl restart liverelay'
```

## DNS Records

| Record | Type | Value | Cloudflare Proxy |
|--------|------|-------|-----------------|
| `app.brozr.com` | A | App Node IP | Yes (orange cloud) |
| `admin.brozr.com` | A | App Node IP | Yes (orange cloud) |
| `api.brozr.com` | A | App Node IP | No (grey cloud) |
| `sfu.brozr.com` | A | SFU Node IP | No (grey cloud) |
| `media.brozr.com` | CNAME | Bunny CDN | No |

**Important:** `api.brozr.com` and `sfu.brozr.com` must NOT be proxied through Cloudflare (WebSocket/WebRTC incompatible).

## Backup Strategy

### PostgreSQL

```bash
# Manual backup
./broz-infra/scripts/backup-postgres.sh

# With upload to Object Storage
./broz-infra/scripts/backup-postgres.sh --upload

# Cron (daily at 03:00)
0 3 * * * /root/broz-infra/scripts/backup-postgres.sh --upload >> /var/log/broz-backup.log 2>&1
```

### Retention Policy

- 7 daily backups
- 4 weekly backups (Sundays)
- 3 monthly backups (1st of month)

### What to Backup

| Component | Backup? | Method |
|-----------|---------|--------|
| PostgreSQL (7 DBs) | Yes | pg_dump daily + Object Storage |
| MinIO (user media) | Yes | Bunny CDN acts as secondary, bucket versioning |
| Redis | No | Ephemeral by design (sessions, rate limits) |
| RabbitMQ | No | Ephemeral by design (message queues) |
| K8s manifests | Yes | Git repository |

## Storage Strategy

| Period | PostgreSQL | MinIO (media) | Action |
|--------|-----------|---------------|--------|
| 0-3 months | <5 GB | <10 GB | Default 20Gi PVC sufficient |
| 3-6 months | 5-20 GB | 10-50 GB | Expand PVC to 50Gi (online resize) |
| 6-12 months | 20-100 GB | 50-200 GB | PG on dedicated server, MinIO -> Object Storage |
| 12+ months | 100+ GB | 500+ GB | Dedicated NVMe RAID for PG |

`analytics_events` and `messages` are the fastest-growing tables. Plan retention/archiving policies.

## Monitoring

- **Prometheus:** scrapes all services at `/health` or `/metrics` every 15-30s
- **Grafana:** dashboards at `grafana.broz-monitoring.svc.cluster.local:3000`
- **Logs:** `kubectl logs -f deployment/broz-<service> -n broz-app`
- **SFU logs:** `journalctl -u liverelay -f`

## Security Checklist

- [ ] Change all `CHANGE_ME_IN_PRODUCTION` secrets
- [ ] Enable UFW on both nodes
- [ ] Configure SSH key-only auth (disable password)
- [ ] Set Cloudflare SSL mode to "Full (strict)"
- [ ] Configure MinIO bucket policies (private by default)
- [ ] Set `LIVERELAY_ALLOWED_ORIGINS` to production domains only
- [ ] Review Traefik security headers
- [ ] Enable Hetzner Cloud Firewall rules
- [ ] Setup fail2ban on both nodes

## Private Network (vSwitch)

App Node and SFU Node communicate via Hetzner vSwitch (free):
- Private subnet: `10.0.0.0/16`
- App Node: `10.0.0.1`
- SFU Node: `10.0.0.2`
- DB Node (Phase 2): `10.0.0.3`

The DB Node (Phase 2+) has NO public IP — accessible only via private network.
