#!/usr/bin/env bash
# Deployment script for mcp-trader
# Pulls latest Docker images from GHCR and restarts services
#
# Usage: ./scripts/deploy/pull-and-restart.sh

set -euo pipefail

# T020: Script initialization with proper error handling
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "========================================="
echo "MCP Trader Deployment Script"
echo "========================================="
echo "Time: $(date)"
echo "Repository: $REPO_ROOT"
echo ""

# Change to repository root
cd "$REPO_ROOT"

# T024: Add deployment logging with timestamps
LOG_FILE="${REPO_ROOT}/deployment.log"
echo "$(date '+%Y-%m-%d %H:%M:%S') - Starting deployment" >> "$LOG_FILE"

# T021: Pull latest images from GitHub Container Registry
echo "[1/4] Pulling latest Docker images from GHCR..."
if docker-compose pull; then
    echo "✓ Images pulled successfully"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Images pulled successfully" >> "$LOG_FILE"
else
    echo "✗ Failed to pull images"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - ERROR: Failed to pull images" >> "$LOG_FILE"
    exit 1
fi

echo ""

# T022: Restart services with new images
echo "[2/4] Restarting services with new images..."
if docker-compose up -d; then
    echo "✓ Services restarted successfully"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Services restarted" >> "$LOG_FILE"
else
    echo "✗ Failed to restart services"
    echo "$(date '+%Y-%m-%%d %H:%M:%S') - ERROR: Failed to restart services" >> "$LOG_FILE"
    exit 1
fi

echo ""

# T023: Health check verification loop
echo "[3/4] Waiting for services to become healthy..."
echo "Waiting 15 seconds for containers to initialize..."
sleep 15

# Check service status
echo "Verifying service health..."
docker-compose ps

echo ""

# T023 continued: Verify health checks pass
echo "[4/4] Checking container health status..."
UNHEALTHY=$(docker-compose ps --filter "health=unhealthy" -q | wc -l)
STARTING=$(docker-compose ps --filter "health=starting" -q | wc -l)

if [ "$UNHEALTHY" -gt 0 ]; then
    echo "⚠ WARNING: $UNHEALTHY container(s) are unhealthy"
    echo "Run 'docker-compose ps' and 'docker-compose logs' to investigate"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - WARNING: Unhealthy containers detected" >> "$LOG_FILE"
elif [ "$STARTING" -gt 0 ]; then
    echo "⚠ NOTE: $STARTING container(s) are still starting up"
    echo "Health checks may take a few more seconds to complete"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Containers still starting" >> "$LOG_FILE"
else
    echo "✓ All containers are healthy"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - All containers healthy" >> "$LOG_FILE"
fi

echo ""
echo "========================================="
echo "Deployment Complete!"
echo "========================================="
echo "$(date '+%Y-%m-%d %H:%M:%S') - Deployment completed successfully" >> "$LOG_FILE"
echo ""
echo "To view logs:     docker-compose logs -f"
echo "To check status:  docker-compose ps"
echo "To view history:  cat deployment.log"
