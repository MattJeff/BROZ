#!/usr/bin/env bash
# ============================================================================
# LiveRelay SFU — VPS deployment script
# ============================================================================
# Run this on your VPS to set up LiveRelay from scratch.
#
# Prerequisites:
#   - Debian/Ubuntu VPS with root access
#   - Domain pointing to this VPS IP
#   - Docker + Docker Compose installed
#
# Usage:
#   chmod +x deploy/deploy.sh
#   sudo ./deploy/deploy.sh
# ============================================================================

set -euo pipefail

DOMAIN="${1:-sfu.example.com}"
INSTALL_DIR="/opt/liverelay"
TURN_USER="liverelay"
TURN_PASS=$(openssl rand -hex 16)
JWT_SECRET=$(openssl rand -hex 32)

echo "================================================"
echo "  LiveRelay SFU — Production Deployment"
echo "================================================"
echo "  Domain:      $DOMAIN"
echo "  Install dir: $INSTALL_DIR"
echo "================================================"
echo ""

# ── 1. Create system user ────────────────────────────────────────────────

echo "[1/7] Creating system user..."
if ! id -u liverelay &>/dev/null; then
    useradd -r -s /bin/false -m -d "$INSTALL_DIR" liverelay
fi

# ── 2. Create directory structure ────────────────────────────────────────

echo "[2/7] Setting up directory structure..."
mkdir -p "$INSTALL_DIR"/{certs,coturn,static}

# ── 3. Generate .env file ────────────────────────────────────────────────

echo "[3/7] Generating .env configuration..."
cat > "$INSTALL_DIR/.env" <<EOF
# LiveRelay SFU — Generated $(date -Iseconds)

LIVERELAY_BIND_ADDR=0.0.0.0:8080
LIVERELAY_PUBLIC_HOST=$DOMAIN

# TLS — disabled (Caddy handles HTTPS).
LIVERELAY_TLS_ENABLED=false

# TURN — embedded.
LIVERELAY_TURN_EMBEDDED=true
LIVERELAY_TURN_PORT=3478
LIVERELAY_TURN_USERNAME=$TURN_USER
LIVERELAY_TURN_PASSWORD=$TURN_PASS
LIVERELAY_TURN_REALM=$DOMAIN

LIVERELAY_STUN_URLS=stun:stun.l.google.com:19302,stun:stun1.l.google.com:19302
LIVERELAY_TURN_URLS=

LIVERELAY_JWT_SECRET=$JWT_SECRET

LIVERELAY_MAX_ROOMS=100
LIVERELAY_MAX_SUBSCRIBERS_PER_ROOM=1000

LIVERELAY_ALLOWED_ORIGINS=https://$DOMAIN
LIVERELAY_LOG_LEVEL=info
EOF

# ── 4. Generate Caddyfile ────────────────────────────────────────────────

echo "[4/7] Generating Caddyfile..."
cat > "$INSTALL_DIR/Caddyfile" <<EOF
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

# ── 5. Open firewall ports ───────────────────────────────────────────────

echo "[5/7] Configuring firewall..."
if command -v ufw &>/dev/null; then
    ufw allow 80/tcp    # HTTP (Let's Encrypt challenge)
    ufw allow 443/tcp   # HTTPS
    ufw allow 3478/udp  # TURN
    ufw allow 49152:65535/udp  # TURN relay ports
    echo "  UFW rules added."
elif command -v firewall-cmd &>/dev/null; then
    firewall-cmd --permanent --add-port=80/tcp
    firewall-cmd --permanent --add-port=443/tcp
    firewall-cmd --permanent --add-port=3478/udp
    firewall-cmd --permanent --add-port=49152-65535/udp
    firewall-cmd --reload
    echo "  firewalld rules added."
else
    echo "  WARNING: No firewall detected. Ensure ports 80, 443, 3478/udp are open."
fi

# ── 6. Set permissions ───────────────────────────────────────────────────

echo "[6/7] Setting permissions..."
chown -R liverelay:liverelay "$INSTALL_DIR"
chmod 600 "$INSTALL_DIR/.env"

# ── 7. Print summary ────────────────────────────────────────────────────

echo "[7/7] Done!"
echo ""
echo "================================================"
echo "  DEPLOYMENT SUMMARY"
echo "================================================"
echo "  Install dir:   $INSTALL_DIR"
echo "  TURN user:     $TURN_USER"
echo "  TURN password: $TURN_PASS"
echo "  JWT secret:    $JWT_SECRET"
echo ""
echo "  IMPORTANT: Save these credentials!"
echo ""
echo "  Next steps:"
echo "    1. Copy your binary + static files to $INSTALL_DIR/"
echo "    2. docker compose --profile caddy up -d"
echo "       OR: install systemd service + standalone Caddy"
echo "    3. Access: https://$DOMAIN"
echo "================================================"
