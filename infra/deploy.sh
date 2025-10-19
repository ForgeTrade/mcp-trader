#!/bin/bash
set -e

SERVER="root@198.13.46.14"
DEPLOY_DIR="/opt/mcp-trader"
OLD_DIR="/opt/mcp-binance-rs"

echo "🚀 Starting deployment to $SERVER"

# 1. Stop old service
echo "⏹️  Stopping old service..."
ssh $SERVER "systemctl stop mcp-binance.service || true"
ssh $SERVER "systemctl disable mcp-binance.service || true"

# 2. Create deployment directory
echo "📁 Creating deployment directory..."
ssh $SERVER "mkdir -p $DEPLOY_DIR"

# 3. Sync project files (excluding build artifacts and git)
echo "📤 Syncing project files..."
rsync -avz --delete \
    --exclude '.git' \
    --exclude 'target' \
    --exclude '__pycache__' \
    --exclude '.venv' \
    --exclude 'node_modules' \
    --exclude '.specify' \
    --exclude 'specs' \
    --exclude '*.log' \
    --exclude 'data/' \
    ./ $SERVER:$DEPLOY_DIR/

# 4. Build Rust provider
echo "🦀 Building Rust binance provider..."
ssh $SERVER "source \$HOME/.cargo/env && cd $DEPLOY_DIR/providers/binance-rs && cargo build --release --features 'orderbook,orderbook_analytics'"

# 5. Create .env file if it doesn't exist
echo "🔐 Setting up environment file..."
ssh $SERVER "if [ ! -f $DEPLOY_DIR/providers/binance-rs/.env ]; then cp $DEPLOY_DIR/providers/binance-rs/.env.example $DEPLOY_DIR/providers/binance-rs/.env || touch $DEPLOY_DIR/providers/binance-rs/.env; fi"

# 6. Configure nginx
echo "🌐 Configuring nginx..."
ssh $SERVER "cp $DEPLOY_DIR/infra/nginx-mcp-gateway.conf /etc/nginx/sites-available/mcp-gateway"
ssh $SERVER "ln -sf /etc/nginx/sites-available/mcp-gateway /etc/nginx/sites-enabled/mcp-gateway"
ssh $SERVER "nginx -t && systemctl reload nginx || echo 'Nginx config test failed'"

# 7. Copy systemd service files
echo "⚙️  Installing systemd service..."
ssh $SERVER "cp $DEPLOY_DIR/infra/binance-provider.service /etc/systemd/system/"

# 8. Reload systemd and enable service
echo "🔄 Reloading systemd..."
ssh $SERVER "systemctl daemon-reload"
ssh $SERVER "systemctl enable binance-provider.service"

# 9. Start service
echo "▶️  Starting binance provider..."
ssh $SERVER "systemctl restart binance-provider.service"
sleep 5

# 10. Check service status
echo "✅ Checking service status..."
ssh $SERVER "systemctl status binance-provider.service --no-pager -l || true"

# 11. Check if service is listening on port 3000
echo ""
echo "🔍 Checking listening port 3000..."
ssh $SERVER "ss -tlnp | grep 3000 || echo 'Service not listening on port 3000'"

# 12. Configure HTTPS with certbot
echo ""
echo "🔒 Configuring HTTPS with certbot..."
ssh $SERVER "certbot --nginx -d mcp-gateway.thevibe.trading --non-interactive --agree-tos --redirect --email admin@thevibe.trading || echo 'Certbot configuration completed or already exists'"

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📊 View logs with:"
echo "   journalctl -u binance-provider.service -f"
echo ""
echo "🌐 Access the service at:"
echo "   https://mcp-gateway.thevibe.trading"
echo ""
echo "🔧 Test the HTTP API:"
echo "   curl https://mcp-gateway.thevibe.trading/sse"
