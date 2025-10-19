#!/bin/bash
set -e

SERVER="root@198.13.46.14"
DEPLOY_DIR="/opt/mcp-trader"

echo "🚀 Starting quick deployment to $SERVER"

# 1. Build locally if needed
echo "🦀 Building binance provider locally..."
cd providers/binance-rs
cargo build --release --features 'orderbook,orderbook_analytics'
cd ../..

# 2. Stop old service
echo "⏹️  Stopping old services..."
ssh $SERVER "systemctl stop mcp-binance.service 2>/dev/null || true"
ssh $SERVER "systemctl disable mcp-binance.service 2>/dev/null || true"
ssh $SERVER "systemctl stop binance-provider.service 2>/dev/null || true"

# 3. Create deployment directory
echo "📁 Creating deployment directory..."
ssh $SERVER "mkdir -p $DEPLOY_DIR/providers/binance-rs/target/release"

# 4. Sync only necessary files
echo "📤 Syncing files..."
rsync -avz --progress ./providers/binance-rs/target/release/binance-provider $SERVER:$DEPLOY_DIR/providers/binance-rs/target/release/
rsync -avz --progress ./providers/binance-rs/.env.example $SERVER:$DEPLOY_DIR/providers/binance-rs/
rsync -avz --progress ./infra/binance-provider.service $SERVER:$DEPLOY_DIR/infra/
rsync -avz --progress ./infra/nginx-mcp-gateway.conf $SERVER:$DEPLOY_DIR/infra/

# 5. Create .env file if it doesn't exist
echo "🔐 Setting up environment file..."
ssh $SERVER "if [ ! -f $DEPLOY_DIR/providers/binance-rs/.env ]; then cp $DEPLOY_DIR/providers/binance-rs/.env.example $DEPLOY_DIR/providers/binance-rs/.env || touch $DEPLOY_DIR/providers/binance-rs/.env; fi"

# 6. Configure nginx
echo "🌐 Configuring nginx..."
ssh $SERVER "cp $DEPLOY_DIR/infra/nginx-mcp-gateway.conf /etc/nginx/sites-available/mcp-gateway"
ssh $SERVER "ln -sf /etc/nginx/sites-available/mcp-gateway /etc/nginx/sites-enabled/mcp-gateway"
ssh $SERVER "nginx -t && systemctl reload nginx"

# 7. Install systemd service
echo "⚙️  Installing systemd service..."
ssh $SERVER "cp $DEPLOY_DIR/infra/binance-provider.service /etc/systemd/system/"
ssh $SERVER "systemctl daemon-reload"
ssh $SERVER "systemctl enable binance-provider.service"

# 8. Start service
echo "▶️  Starting binance provider..."
ssh $SERVER "systemctl restart binance-provider.service"
sleep 5

# 9. Check service status
echo "✅ Checking service status..."
ssh $SERVER "systemctl status binance-provider.service --no-pager -l || true"

# 10. Check if service is listening on port 3000
echo ""
echo "🔍 Checking listening port 3000..."
ssh $SERVER "ss -tlnp | grep 3000 || echo 'Service not listening on port 3000'"

# 11. Configure HTTPS with certbot
echo ""
echo "🔒 Configuring HTTPS with certbot..."
ssh $SERVER "certbot --nginx -d mcp-gateway.thevibe.trading --non-interactive --agree-tos --redirect --email admin@thevibe.trading 2>&1 || echo 'Certbot may already be configured'"

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📊 View logs with:"
echo "   ssh $SERVER journalctl -u binance-provider.service -f"
echo ""
echo "🌐 Access the service at:"
echo "   https://mcp-gateway.thevibe.trading"
echo ""
echo "🔧 Test the HTTP API:"
echo "   curl https://mcp-gateway.thevibe.trading/sse"
