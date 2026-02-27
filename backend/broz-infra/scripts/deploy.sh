#!/bin/bash
set -e

# ============================================================================
# BROZ Microservices — Build & Deploy Script
# ============================================================================
# Builds Docker images and deploys to K3s cluster.
#
# Usage:
#   ./deploy.sh [service|all|frontend|sfu] [--push] [--arm64]
#
# Examples:
#   ./deploy.sh all                    # Build + deploy all services (x86)
#   ./deploy.sh broz-auth --push       # Build, push, and deploy broz-auth
#   ./deploy.sh all --arm64 --push     # Build for ARM64 (Phase 2 CAX nodes)
#   ./deploy.sh frontend --push        # Build + deploy frontend
#   ./deploy.sh sfu                    # Build SFU binary (cross-compile if --arm64)
# ============================================================================

SERVICE=${1:-all}
PUSH=false
ARM64=false
REGISTRY="${REGISTRY:-registry.brozr.com}"
GIT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "latest")

shift || true
for arg in "$@"; do
    case $arg in
        --push) PUSH=true ;;
        --arm64) ARM64=true ;;
    esac
done

SERVICES="broz-gateway broz-auth broz-user broz-matching broz-messaging broz-notification broz-moderation broz-analytics"

echo "============================================"
echo "  BROZ Deployment"
echo "============================================"
echo "  Service:   $SERVICE"
echo "  Registry:  $REGISTRY"
echo "  Git SHA:   $GIT_SHA"
echo "  Push:      $PUSH"
echo "  ARM64:     $ARM64"
echo "============================================"
echo ""

# Docker buildx platform flag
PLATFORM=""
if [ "$ARM64" = true ]; then
    PLATFORM="--platform linux/arm64"
    echo "Building for ARM64 (Hetzner CAX / Ampere)..."
fi

build_service() {
    local svc=$1
    echo "Building $svc..."
    docker buildx build \
        $PLATFORM \
        --build-arg SERVICE=$svc \
        -t "$REGISTRY/$svc:latest" \
        -t "$REGISTRY/$svc:$GIT_SHA" \
        -f broz-infra/docker/Dockerfile \
        --load \
        .
    echo "  $svc built."
}

build_frontend() {
    echo "Building frontend..."
    docker buildx build \
        $PLATFORM \
        -t "$REGISTRY/broz-frontend:latest" \
        -t "$REGISTRY/broz-frontend:$GIT_SHA" \
        -f ../frontend/Dockerfile \
        --load \
        ../frontend/
    echo "  frontend built."
}

build_sfu() {
    echo "Building SFU (LiveRelay)..."
    docker buildx build \
        $PLATFORM \
        -t "$REGISTRY/liverelay:latest" \
        -t "$REGISTRY/liverelay:$GIT_SHA" \
        -f ../WEB_RTC/Dockerfile \
        --load \
        ../WEB_RTC/
    echo "  SFU built."
}

push_image() {
    local image=$1
    if [ "$PUSH" = true ]; then
        echo "  Pushing $image..."
        docker push "$REGISTRY/$image:latest"
        docker push "$REGISTRY/$image:$GIT_SHA"
    fi
}

deploy_service() {
    local svc=$1
    echo "  Deploying $svc..."
    kubectl set image "deployment/$svc" "$svc=$REGISTRY/$svc:$GIT_SHA" -n broz-app
    kubectl rollout status "deployment/$svc" -n broz-app --timeout=120s
    echo "  $svc deployed."
}

deploy_frontend() {
    echo "  Deploying frontend..."
    kubectl set image deployment/broz-frontend broz-frontend="$REGISTRY/broz-frontend:$GIT_SHA" -n broz-frontend
    kubectl rollout status deployment/broz-frontend -n broz-frontend --timeout=60s
    echo "  frontend deployed."
}

# ── Execute ────────────────────────────────────────────────────────────────

case "$SERVICE" in
    all)
        for svc in $SERVICES; do
            build_service "$svc"
            push_image "$svc"
            deploy_service "$svc"
        done
        ;;
    frontend)
        build_frontend
        push_image "broz-frontend"
        deploy_frontend
        ;;
    sfu)
        build_sfu
        push_image "liverelay"
        echo ""
        echo "SFU built. To deploy on SFU node:"
        echo "  docker save liverelay:$GIT_SHA | ssh root@SFU_NODE 'docker load'"
        echo "  OR: scp the binary and restart the systemd service"
        ;;
    *)
        build_service "$SERVICE"
        push_image "$SERVICE"
        deploy_service "$SERVICE"
        ;;
esac

echo ""
echo "============================================"
echo "  Deployment Complete"
echo "============================================"
