#!/usr/bin/env bash
# ============================================================================
# BROZ — App Node Setup (Hetzner CX33 — Phase 1)
# ============================================================================
# Installs K3s, deploys all infrastructure and microservices.
#
# Target: Hetzner CX33 (4 vCPU, 8 GB RAM, 80 GB SSD)
# OS: Ubuntu 22.04 LTS
#
# Usage:
#   scp -r broz-infra/ root@APP_NODE_IP:/root/
#   ssh root@APP_NODE_IP
#   chmod +x /root/broz-infra/scripts/setup-app-node.sh
#   ./broz-infra/scripts/setup-app-node.sh <SFU_PRIVATE_IP>
# ============================================================================

set -euo pipefail

SFU_PRIVATE_IP="${1:?Usage: $0 <SFU_PRIVATE_IP>}"
DOMAIN="${2:-brozr.com}"
REGISTRY="${3:-registry.brozr.com}"
K8S_DIR="/root/broz-infra/k8s"

echo "================================================"
echo "  BROZ App Node — Phase 1 Setup"
echo "================================================"
echo "  SFU Private IP: $SFU_PRIVATE_IP"
echo "  Domain:         $DOMAIN"
echo "  Registry:       $REGISTRY"
echo "================================================"
echo ""

# ── 1. System updates ──────────────────────────────────────────────────────

echo "[1/8] Updating system..."
apt-get update -qq && apt-get upgrade -y -qq
apt-get install -y -qq curl wget jq openssl

# ── 2. Install K3s ─────────────────────────────────────────────────────────

echo "[2/8] Installing K3s..."
curl -sfL https://get.k3s.io | INSTALL_K3S_EXEC="server \
  --disable=servicelb \
  --write-kubeconfig-mode=644 \
  --tls-san=$(curl -s ifconfig.me) \
  --tls-san=api.$DOMAIN" sh -

# Wait for K3s to be ready
echo "  Waiting for K3s..."
until kubectl get nodes &>/dev/null; do sleep 2; done
kubectl wait --for=condition=Ready node --all --timeout=120s
echo "  K3s ready."

# ── 3. Install Hetzner CSI driver ──────────────────────────────────────────

echo "[3/8] Installing Hetzner CSI driver..."
if [ -n "${HCLOUD_TOKEN:-}" ]; then
    kubectl create secret generic hcloud -n kube-system \
        --from-literal=token="$HCLOUD_TOKEN" \
        --dry-run=client -o yaml | kubectl apply -f -
    kubectl apply -f https://raw.githubusercontent.com/hetznercloud/csi-driver/main/deploy/kubernetes/hcloud-csi.yml
    echo "  Hetzner CSI installed."
else
    echo "  HCLOUD_TOKEN not set — skipping CSI driver (using local storage)."
fi

# ── 4. Install Traefik via Helm ────────────────────────────────────────────

echo "[4/8] Configuring Traefik..."
# K3s comes with Traefik pre-installed, configure it
if [ -f "$K8S_DIR/broz-system/traefik-values.yaml" ]; then
    # Update Traefik with custom values
    cat > /var/lib/rancher/k3s/server/manifests/traefik-config.yaml <<EOF
apiVersion: helm.cattle.io/v1
kind: HelmChartConfig
metadata:
  name: traefik
  namespace: kube-system
spec:
  valuesContent: |
    additionalArguments:
      - "--certificatesresolvers.letsencrypt.acme.email=admin@$DOMAIN"
      - "--certificatesresolvers.letsencrypt.acme.storage=/data/acme.json"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web"
      - "--providers.kubernetescrd.allowCrossNamespace=true"
      - "--entrypoints.web.http.redirections.entrypoint.to=websecure"
      - "--entrypoints.web.http.redirections.entrypoint.scheme=https"
    ports:
      web:
        exposedPort: 80
      websecure:
        exposedPort: 443
    logs:
      general:
        level: WARN
      access:
        enabled: true
EOF
    echo "  Traefik configured."
fi

# ── 5. Create namespaces ───────────────────────────────────────────────────

echo "[5/8] Creating namespaces..."
kubectl apply -f "$K8S_DIR/broz-system/namespaces.yaml"

# ── 6. Generate and apply secrets ──────────────────────────────────────────

echo "[6/8] Generating production secrets..."
JWT_SECRET=$(openssl rand -hex 32)
PG_PASSWORD=$(openssl rand -hex 16)
RMQ_PASSWORD=$(openssl rand -hex 16)
MINIO_SECRET=$(openssl rand -hex 16)
LIVERELAY_API_KEY="lr_$(openssl rand -hex 20)"
LIVERELAY_JWT=$(openssl rand -hex 32)

# Apply secrets to broz-data namespace
kubectl create secret generic broz-secrets -n broz-data \
    --from-literal=POSTGRES_USER=brozadmin \
    --from-literal=POSTGRES_PASSWORD="$PG_PASSWORD" \
    --from-literal=RABBITMQ_USER=broz \
    --from-literal=RABBITMQ_PASS="$RMQ_PASSWORD" \
    --from-literal=MINIO_ACCESS_KEY=minioadmin \
    --from-literal=MINIO_SECRET_KEY="$MINIO_SECRET" \
    --dry-run=client -o yaml | kubectl apply -f -

# Apply secrets to broz-app namespace (cross-namespace access)
kubectl create secret generic broz-secrets -n broz-app \
    --from-literal=POSTGRES_USER=brozadmin \
    --from-literal=POSTGRES_PASSWORD="$PG_PASSWORD" \
    --from-literal=RABBITMQ_USER=broz \
    --from-literal=RABBITMQ_PASS="$RMQ_PASSWORD" \
    --from-literal=MINIO_ACCESS_KEY=minioadmin \
    --from-literal=MINIO_SECRET_KEY="$MINIO_SECRET" \
    --dry-run=client -o yaml | kubectl apply -f -

kubectl create secret generic broz-app-secrets -n broz-app \
    --from-literal=JWT_SECRET="$JWT_SECRET" \
    --from-literal=RESEND_API_KEY="${RESEND_API_KEY:-re_CHANGE_ME}" \
    --from-literal=GOOGLE_CLIENT_ID="${GOOGLE_CLIENT_ID:-}" \
    --from-literal=GOOGLE_CLIENT_SECRET="${GOOGLE_CLIENT_SECRET:-}" \
    --from-literal=LIVERELAY_API_KEY="$LIVERELAY_API_KEY" \
    --from-literal=LIVERELAY_JWT_SECRET="$LIVERELAY_JWT" \
    --dry-run=client -o yaml | kubectl apply -f -

# Save credentials to file
cat > /root/broz-credentials.env <<EOF
# BROZ Production Credentials — Generated $(date -Iseconds)
# KEEP THIS FILE SAFE — DELETE AFTER NOTING CREDENTIALS

JWT_SECRET=$JWT_SECRET
POSTGRES_PASSWORD=$PG_PASSWORD
RABBITMQ_PASSWORD=$RMQ_PASSWORD
MINIO_SECRET_KEY=$MINIO_SECRET
LIVERELAY_API_KEY=$LIVERELAY_API_KEY
LIVERELAY_JWT_SECRET=$LIVERELAY_JWT
SFU_PRIVATE_IP=$SFU_PRIVATE_IP
EOF
chmod 600 /root/broz-credentials.env

# ── 7. Deploy infrastructure + services ───────────────────────────────────

echo "[7/8] Deploying infrastructure..."

# Create init-script ConfigMap for PostgreSQL
kubectl create configmap postgres-init -n broz-data \
    --from-file=/root/broz-infra/scripts/create-databases.sh \
    --dry-run=client -o yaml | kubectl apply -f -

# Storage (Hetzner or local)
if [ -n "${HCLOUD_TOKEN:-}" ]; then
    kubectl apply -f "$K8S_DIR/broz-system/hetzner-storage.yaml"
else
    # Use local-path for Phase 1 without CSI
    echo "  Using K3s local-path storage."
fi

# Data layer
kubectl apply -f "$K8S_DIR/broz-data/postgres.yaml"
kubectl apply -f "$K8S_DIR/broz-data/redis.yaml"
kubectl apply -f "$K8S_DIR/broz-data/rabbitmq.yaml"
kubectl apply -f "$K8S_DIR/broz-data/minio.yaml"

echo "  Waiting for data layer..."
kubectl wait --for=condition=Ready pod -l app=postgres -n broz-data --timeout=120s
kubectl wait --for=condition=Ready pod -l app=redis -n broz-data --timeout=60s
kubectl wait --for=condition=Ready pod -l app=rabbitmq -n broz-data --timeout=120s

# Patch broz-messaging to use SFU private IP
sed -i "s|SFU_NODE_PRIVATE_IP|$SFU_PRIVATE_IP|g" "$K8S_DIR/broz-app/broz-messaging.yaml"

# Microservices
for manifest in "$K8S_DIR/broz-app"/broz-*.yaml; do
    kubectl apply -f "$manifest"
done

# Frontend
kubectl apply -f "$K8S_DIR/broz-frontend/frontend.yaml"

# Ingress
kubectl apply -f "$K8S_DIR/broz-system/traefik-ingress.yaml"

# Monitoring
kubectl apply -f "$K8S_DIR/broz-monitoring/prometheus.yaml"

# ── 8. Summary ─────────────────────────────────────────────────────────────

echo "[8/8] Setup complete!"
echo ""
echo "================================================"
echo "  APP NODE DEPLOYMENT SUMMARY"
echo "================================================"
echo ""
echo "  K3s:         $(k3s --version | head -1)"
echo "  Namespaces:  broz-system, broz-data, broz-app, broz-frontend, broz-monitoring"
echo ""
echo "  Credentials saved to: /root/broz-credentials.env"
echo "  (DELETE after noting the values!)"
echo ""
echo "  SFU Node needs these credentials:"
echo "    LIVERELAY_API_KEY=$LIVERELAY_API_KEY"
echo "    LIVERELAY_JWT_SECRET=$LIVERELAY_JWT"
echo ""
echo "  DNS records to create:"
echo "    api.$DOMAIN    → $(curl -s ifconfig.me)"
echo "    app.$DOMAIN    → Cloudflare (proxied)"
echo "    admin.$DOMAIN  → Cloudflare (proxied)"
echo "    sfu.$DOMAIN    → SFU Node public IP (DNS only, NO proxy)"
echo "    media.$DOMAIN  → Bunny CDN CNAME"
echo ""
echo "  Next steps:"
echo "    1. Build and push images to $REGISTRY"
echo "    2. Setup SFU Node: ./setup-sfu-node.sh"
echo "    3. Configure DNS records"
echo "    4. Setup backup cron: ./setup-backup.sh"
echo "================================================"
