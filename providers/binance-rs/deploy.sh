#!/bin/bash
set -e

# Binance Provider Deployment Script
# This script deploys the Binance Provider to /opt/binance-provider

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="/opt/binance-provider"
SERVICE_NAME="binance-provider.service"
BINARY_NAME="binance-provider"

echo "==================================="
echo "Binance Provider Deployment"
echo "==================================="
echo ""

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run as root or with sudo"
    echo "Usage: sudo ./deploy.sh"
    exit 1
fi

# Check if binary exists
if [ ! -f "$SCRIPT_DIR/target/release/$BINARY_NAME" ]; then
    echo "Error: Release binary not found at $SCRIPT_DIR/target/release/$BINARY_NAME"
    echo "Please run 'cargo build --release' first"
    exit 1
fi

echo "1. Creating deployment directory..."
mkdir -p "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR/data/analytics"
chown -R limerc:limerc "$DEPLOY_DIR"

echo "2. Copying binary..."
cp "$SCRIPT_DIR/target/release/$BINARY_NAME" "$DEPLOY_DIR/"
chmod +x "$DEPLOY_DIR/$BINARY_NAME"
chown limerc:limerc "$DEPLOY_DIR/$BINARY_NAME"

echo "3. Setting up environment configuration..."
if [ -f "$SCRIPT_DIR/.env" ]; then
    cp "$SCRIPT_DIR/.env" "$DEPLOY_DIR/.env"
    echo "   - Copied existing .env file"
else
    cp "$SCRIPT_DIR/.env.example" "$DEPLOY_DIR/.env"
    echo "   - Created .env from .env.example"
    echo "   - WARNING: Please edit /opt/binance-provider/.env with your configuration"
fi

# Update ANALYTICS_DATA_PATH in deployed .env to use absolute path
sed -i 's|ANALYTICS_DATA_PATH=./data/analytics|ANALYTICS_DATA_PATH=/opt/binance-provider/data/analytics|g' "$DEPLOY_DIR/.env"
chown limerc:limerc "$DEPLOY_DIR/.env"
chmod 600 "$DEPLOY_DIR/.env"

echo "4. Installing systemd service..."
cp "$SCRIPT_DIR/$SERVICE_NAME" "/etc/systemd/system/$SERVICE_NAME"
systemctl daemon-reload

echo "5. Enabling service..."
systemctl enable "$SERVICE_NAME"

echo ""
echo "==================================="
echo "Deployment Complete!"
echo "==================================="
echo ""
echo "Binary installed to: $DEPLOY_DIR/$BINARY_NAME"
echo "Configuration: $DEPLOY_DIR/.env"
echo "Data directory: $DEPLOY_DIR/data/analytics"
echo ""
echo "Next steps:"
echo "  1. Edit configuration (if needed):"
echo "     sudo nano $DEPLOY_DIR/.env"
echo ""
echo "  2. Start the service:"
echo "     sudo systemctl start $SERVICE_NAME"
echo ""
echo "  3. Check status:"
echo "     sudo systemctl status $SERVICE_NAME"
echo ""
echo "  4. View logs:"
echo "     sudo journalctl -u $SERVICE_NAME -f"
echo ""
echo "Service will listen on:"
echo "  - gRPC: 0.0.0.0:50053"
echo ""
