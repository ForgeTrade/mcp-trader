#!/usr/bin/env bash
# Rollback script for mcp-trader
# Reverts services to a previous image version by commit SHA
#
# Usage: ./scripts/deploy/rollback.sh <commit-sha>
# Example: ./scripts/deploy/rollback.sh abc1234

set -euo pipefail

# T026: Script initialization with commit SHA parameter
if [ $# -eq 0 ]; then
    echo "Usage: $0 <commit-sha>"
    echo ""
    echo "Example: $0 abc1234"
    echo ""
    echo "This will rollback both binance-provider and mcp-gateway to the"
    echo "images tagged with 'sha-<commit-sha>' in GitHub Container Registry."
    echo ""
    echo "To find available commit SHAs:"
    echo "  - Check deployment.log for previous deployments"
    echo "  - Run 'git log --oneline' to see recent commits"
    echo "  - Check GitHub Container Registry packages for available tags"
    exit 1
fi

ROLLBACK_SHA=$1
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "========================================="
echo "MCP Trader Rollback Script"
echo "========================================="
echo "Time: $(date)"
echo "Rollback to commit SHA: $ROLLBACK_SHA"
echo "Repository: $REPO_ROOT"
echo ""

# Change to repository root
cd "$REPO_ROOT"

# Log rollback attempt
LOG_FILE="${REPO_ROOT}/deployment.log"
echo "$(date '+%Y-%m-%d %H:%M:%S') - Starting rollback to sha-$ROLLBACK_SHA" >> "$LOG_FILE"

# T027: Add SHA tag switching logic (export IMAGE_TAG variables)
echo "[1/4] Setting image tags to sha-$ROLLBACK_SHA..."
export BINANCE_IMAGE_TAG="sha-$ROLLBACK_SHA"
export GATEWAY_IMAGE_TAG="sha-$ROLLBACK_SHA"

echo "✓ Image tags configured:"
echo "  - binance-provider: $BINANCE_IMAGE_TAG"
echo "  - mcp-gateway: $GATEWAY_IMAGE_TAG"
echo ""

# Pull the specific rollback images
echo "[2/4] Pulling rollback images from GHCR..."
if docker-compose pull; then
    echo "✓ Rollback images pulled successfully"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Rollback images pulled (sha-$ROLLBACK_SHA)" >> "$LOG_FILE"
else
    echo "✗ Failed to pull rollback images"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - ERROR: Failed to pull rollback images" >> "$LOG_FILE"
    echo ""
    echo "Possible reasons:"
    echo "  - Commit SHA '$ROLLBACK_SHA' does not exist in GHCR"
    echo "  - Authentication with GHCR failed"
    echo "  - Network connectivity issues"
    exit 1
fi

echo ""

# Restart services with rollback images
echo "[3/4] Restarting services with rollback images..."
if docker-compose up -d; then
    echo "✓ Services restarted with rollback images"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Services restarted with rollback" >> "$LOG_FILE"
else
    echo "✗ Failed to restart services"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - ERROR: Failed to restart with rollback" >> "$LOG_FILE"
    exit 1
fi

echo ""

# Health check verification
echo "[4/4] Verifying rollback success..."
echo "Waiting 15 seconds for containers to initialize..."
sleep 15

echo "Checking service status..."
docker-compose ps

echo ""

# Verify health checks
UNHEALTHY=$(docker-compose ps --filter "health=unhealthy" -q | wc -l)
STARTING=$(docker-compose ps --filter "health=starting" -q | wc -l)

if [ "$UNHEALTHY" -gt 0 ]; then
    echo "⚠ WARNING: $UNHEALTHY container(s) are unhealthy after rollback"
    echo "Run 'docker-compose logs' to investigate"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - WARNING: Unhealthy containers after rollback" >> "$LOG_FILE"
elif [ "$STARTING" -gt 0 ]; then
    echo "⚠ NOTE: $STARTING container(s) are still starting up"
    echo "Health checks may take a few more seconds"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Containers still starting after rollback" >> "$LOG_FILE"
else
    echo "✓ All containers are healthy after rollback"
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Rollback successful, all healthy" >> "$LOG_FILE"
fi

echo ""
echo "========================================="
echo "Rollback Complete!"
echo "========================================="
echo "$(date '+%Y-%m-%d %H:%M:%S') - Rollback to sha-$ROLLBACK_SHA completed" >> "$LOG_FILE"
echo ""
echo "Services are now running image tags: sha-$ROLLBACK_SHA"
echo ""
echo "To view logs:     docker-compose logs -f"
echo "To check status:  docker-compose ps"
echo "To view history:  cat deployment.log"
echo ""
echo "NOTE: To make this rollback persistent, update your .env file with:"
echo "  BINANCE_IMAGE_TAG=sha-$ROLLBACK_SHA"
echo "  GATEWAY_IMAGE_TAG=sha-$ROLLBACK_SHA"
