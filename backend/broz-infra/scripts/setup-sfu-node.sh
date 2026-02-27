#!/usr/bin/env bash
# ============================================================================
# BROZ — SFU Node Setup (Hetzner CPX21 — Phase 1)
# ============================================================================
# Deploys LiveRelay SFU as a standalone binary with Caddy reverse proxy.
# Runs OUTSIDE K8s for direct UDP access (WebRTC requirement).
#
# Target: Hetzner CPX21 (3 vCPU dedicated, 4 GB RAM, 2 TB traffic)
# OS: Ubuntu 22.04 LTS
#
# Usage:
#   scp WEB_RTC/target/release/liverelay root@SFU_NODE_IP:/tmp/
#   ssh root@SFU_NODE_IP
#   chmod +x setup-sfu-node.sh
#   ./setup-sfu-node.sh sfu.brozr.com <API_KEY> <JWT_SECRET> <APP_NODE_PRIVATE_IP>
# ============================================================================

set -euo pipefail

DOMAIN="${1:?Usage: $0 <DOMAIN> <API_KEY> <JWT_SECRET> <APP_NODE_PRIVATE_IP>}"
API_KEY="${2:?Missing API_KEY}"
JWT_SECRET="${3:?Missing JWT_SECRET}"
APP_PRIVATE_IP="${4:?Missing APP_NODE_PRIVATE_IP}"
INSTALL_DIR="/opt/liverelay"
PUBLIC_IP=$(curl -s ifconfig.me)

echo "================================================"
echo "  BROZ SFU Node — Phase 1 Setup"
echo "================================================"
echo "  Domain:         $DOMAIN"
echo "  Public IP:      $PUBLIC_IP"
echo "  App Node:       $APP_PRIVATE_IP"
echo "  Install dir:    $INSTALL_DIR"
echo "================================================"
echo ""

# ── 1. System setup ───────────────────────────────────────────────────────

echo "[1/7] Updating system..."
apt-get update -qq && apt-get upgrade -y -qq
apt-get install -y -qq curl wget jq

# ── 2. Create system user ─────────────────────────────────────────────────

echo "[2/7] Creating system user..."
if ! id -u liverelay &>/dev/null; then
    useradd -r -s /bin/false -m -d "$INSTALL_DIR" liverelay
fi
mkdir -p "$INSTALL_DIR"/{certs,static}

# ── 3. Install Caddy ──────────────────────────────────────────────────────

echo "[3/7] Installing Caddy..."
apt-get install -y -qq debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg 2>/dev/null
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list > /dev/null
apt-get update -qq && apt-get install -y -qq caddy

# ── 4. Configure LiveRelay ─────────────────────────────────────────────────

echo "[4/7] Configuring LiveRelay..."

# Copy binary
if [ -f /tmp/liverelay ]; then
    cp /tmp/liverelay "$INSTALL_DIR/liverelay"
    chmod +x "$INSTALL_DIR/liverelay"
else
    echo "  WARNING: Binary not found at /tmp/liverelay — copy it manually."
fi

# Generate .env
cat > "$INSTALL_DIR/.env" <<EOF
# LiveRelay SFU — Generated $(date -Iseconds)

LIVERELAY_BIND_ADDR=0.0.0.0:8080
LIVERELAY_PUBLIC_HOST=$DOMAIN

# TLS disabled — Caddy handles HTTPS
LIVERELAY_TLS_ENABLED=false

# Embedded TURN
LIVERELAY_TURN_EMBEDDED=true
LIVERELAY_TURN_PORT=3478
LIVERELAY_TURN_USERNAME=liverelay
LIVERELAY_TURN_PASSWORD=$(openssl rand -hex 16)
LIVERELAY_TURN_REALM=$DOMAIN

LIVERELAY_STUN_URLS=stun:stun.l.google.com:19302,stun:stun1.l.google.com:19302

LIVERELAY_JWT_SECRET=$JWT_SECRET
LIVERELAY_API_KEY=$API_KEY

# NAT traversal — CRITICAL: use public IP for ICE candidates
LIVERELAY_NAT_1TO1_IPS=$PUBLIC_IP

# UDP port range for WebRTC media
LIVERELAY_UDP_PORT_MIN=20000
LIVERELAY_UDP_PORT_MAX=20100

# Limits (Phase 1: ~10 concurrent calls)
LIVERELAY_MAX_ROOMS=50
LIVERELAY_MAX_SUBSCRIBERS_PER_ROOM=10

LIVERELAY_ALLOWED_ORIGINS=https://app.$( echo "$DOMAIN" | sed 's/^sfu\.//' ),https://admin.$( echo "$DOMAIN" | sed 's/^sfu\.//' )
LIVERELAY_LOG_LEVEL=info
EOF

# ── 5. Configure Caddy ────────────────────────────────────────────────────

echo "[5/7] Configuring Caddy..."
cat > /etc/caddy/Caddyfile <<EOF
$DOMAIN {
    reverse_proxy localhost:8080

    header {
        X-Content-Type-Options nosniff
        X-Frame-Options DENY
        Referrer-Policy strict-origin-when-cross-origin
        Strict-Transport-Security "max-age=63072000; includeSubDomains; preload"
    }

    log {
        output stdout
        format json
    }
}
EOF

# ── 6. Configure firewall ─────────────────────────────────────────────────

echo "[6/7] Configuring firewall..."
if command -v ufw &>/dev/null; then
    ufw --force enable
    ufw default deny incoming
    ufw default allow outgoing
    ufw allow 22/tcp      # SSH
    ufw allow 80/tcp      # HTTP (Let's Encrypt)
    ufw allow 443/tcp     # HTTPS
    ufw allow 3478/udp    # TURN
    ufw allow 20000:20100/udp  # WebRTC media
    # Allow private network from App Node
    ufw allow from "$APP_PRIVATE_IP" to any port 8080 proto tcp
    echo "  UFW configured."
fi

# ── 7. Install systemd service ────────────────────────────────────────────

echo "[7/7] Installing systemd services..."

cat > /etc/systemd/system/liverelay.service <<'EOF'
[Unit]
Description=LiveRelay WebRTC SFU Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=liverelay
Group=liverelay
WorkingDirectory=/opt/liverelay
ExecStart=/opt/liverelay/liverelay
EnvironmentFile=/opt/liverelay/.env
Restart=always
RestartSec=5

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/opt/liverelay

# Resource limits
LimitNOFILE=65535
LimitNPROC=4096

AmbientCapabilities=CAP_NET_BIND_SERVICE

StandardOutput=journal
StandardError=journal
SyslogIdentifier=liverelay

[Install]
WantedBy=multi-user.target
EOF

# Set permissions
chown -R liverelay:liverelay "$INSTALL_DIR"
chmod 600 "$INSTALL_DIR/.env"

# Enable services
systemctl daemon-reload
systemctl enable caddy liverelay
systemctl restart caddy

if [ -f "$INSTALL_DIR/liverelay" ]; then
    systemctl start liverelay
    echo "  LiveRelay started."
else
    echo "  LiveRelay binary missing — start manually after copying."
fi

echo ""
echo "================================================"
echo "  SFU NODE DEPLOYMENT SUMMARY"
echo "================================================"
echo ""
echo "  Domain:      $DOMAIN"
echo "  Public IP:   $PUBLIC_IP"
echo "  API:         https://$DOMAIN"
echo "  TURN:        $PUBLIC_IP:3478/UDP"
echo "  Media:       $PUBLIC_IP:20000-20100/UDP"
echo ""
echo "  Services:"
echo "    caddy:     $(systemctl is-active caddy)"
echo "    liverelay: $(systemctl is-active liverelay 2>/dev/null || echo 'not started')"
echo ""
echo "  DNS record to create:"
echo "    sfu.brozr.com → $PUBLIC_IP (A record, NO Cloudflare proxy)"
echo ""
echo "  Logs:"
echo "    journalctl -u liverelay -f"
echo "    journalctl -u caddy -f"
echo "================================================"
