#!/bin/bash
set -e

# Binance Provider User Deployment Script
# This script deploys the Binance Provider to ~/.local/binance-provider (no sudo required)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$HOME/.local/binance-provider"
BINARY_NAME="binance-provider"

echo "==================================="
echo "Binance Provider User Deployment"
echo "==================================="
echo ""

# Check if binary exists
if [ ! -f "$SCRIPT_DIR/target/release/$BINARY_NAME" ]; then
    echo "Error: Release binary not found at $SCRIPT_DIR/target/release/$BINARY_NAME"
    echo "Please run 'cargo build --release' first"
    exit 1
fi

echo "1. Creating deployment directory..."
mkdir -p "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR/data/analytics"
mkdir -p "$HOME/.config/systemd/user"

echo "2. Copying binary..."
cp "$SCRIPT_DIR/target/release/$BINARY_NAME" "$DEPLOY_DIR/"
chmod +x "$DEPLOY_DIR/$BINARY_NAME"

echo "3. Setting up environment configuration..."
if [ -f "$SCRIPT_DIR/.env" ]; then
    cp "$SCRIPT_DIR/.env" "$DEPLOY_DIR/.env"
    echo "   - Copied existing .env file"
else
    cp "$SCRIPT_DIR/.env.example" "$DEPLOY_DIR/.env"
    echo "   - Created .env from .env.example"
fi

# Update ANALYTICS_DATA_PATH in deployed .env to use absolute path
sed -i "s|ANALYTICS_DATA_PATH=./data/analytics|ANALYTICS_DATA_PATH=$DEPLOY_DIR/data/analytics|g" "$DEPLOY_DIR/.env"
chmod 600 "$DEPLOY_DIR/.env"

echo "4. Creating user systemd service..."
cat > "$HOME/.config/systemd/user/binance-provider.service" <<EOF
[Unit]
Description=Binance Provider - MCP server for cryptocurrency market data analysis
After=network.target

[Service]
Type=simple
WorkingDirectory=$DEPLOY_DIR
ExecStart=$DEPLOY_DIR/$BINARY_NAME --grpc --port 50053
EnvironmentFile=$DEPLOY_DIR/.env
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Resource limits
LimitNOFILE=65536

[Install]
WantedBy=default.target
EOF

echo "5. Reloading systemd user daemon..."
systemctl --user daemon-reload

echo "6. Enabling service..."
systemctl --user enable binance-provider.service

echo ""
echo "==================================="
echo "Deployment Complete!"
echo "==================================="
echo ""
echo "Binary installed to: $DEPLOY_DIR/$BINARY_NAME"
echo "Configuration: $DEPLOY_DIR/.env"
echo "Data directory: $DEPLOY_DIR/data/analytics"
echo ""
echo "Service commands:"
echo "  Start:   systemctl --user start binance-provider"
echo "  Stop:    systemctl --user stop binance-provider"
echo "  Status:  systemctl --user status binance-provider"
echo "  Logs:    journalctl --user -u binance-provider -f"
echo "  Restart: systemctl --user restart binance-provider"
echo ""
echo "To enable linger (service runs without login):"
echo "  sudo loginctl enable-linger $USER"
echo ""
echo "Service will listen on:"
echo "  - gRPC: 0.0.0.0:50053"
echo ""
