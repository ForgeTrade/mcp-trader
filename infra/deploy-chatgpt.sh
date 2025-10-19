#!/bin/bash
set -e

SERVER="root@198.13.46.14"
DEPLOY_DIR="/opt/mcp-trader"

echo "🚀 Starting ChatGPT MCP deployment to $SERVER"
echo ""

# 1. Build binance provider locally if needed
echo "🦀 Building binance provider locally..."
cd providers/binance-rs
cargo build --release --features 'orderbook,orderbook_analytics'
cd ../..

# 2. Stop old services
echo ""
echo "⏹️  Stopping old services..."
ssh $SERVER "systemctl stop mcp-gateway-sse.service 2>/dev/null || true"
ssh $SERVER "systemctl stop binance-provider.service 2>/dev/null || true"

# 3. Create deployment directories
echo ""
echo "📁 Creating deployment directories..."
ssh $SERVER "mkdir -p $DEPLOY_DIR/providers/binance-rs/target/release"
ssh $SERVER "mkdir -p $DEPLOY_DIR/mcp-gateway"
ssh $SERVER "mkdir -p $DEPLOY_DIR/infra"

# 4. Sync binance provider binary
echo ""
echo "📤 Syncing binance provider binary..."
rsync -avz --progress ./providers/binance-rs/target/release/binance-provider $SERVER:$DEPLOY_DIR/providers/binance-rs/target/release/
rsync -avz --progress ./providers/binance-rs/.env.example $SERVER:$DEPLOY_DIR/providers/binance-rs/

# 5. Sync mcp-gateway Python package
echo ""
echo "📤 Syncing mcp-gateway Python package..."
rsync -avz --progress --exclude '__pycache__' --exclude '.venv' --exclude '*.pyc' \
    ./mcp-gateway/ $SERVER:$DEPLOY_DIR/mcp-gateway/

# 6. Sync infrastructure files
echo ""
echo "📤 Syncing infrastructure files..."
rsync -avz --progress ./infra/binance-provider.service $SERVER:$DEPLOY_DIR/infra/
rsync -avz --progress ./infra/mcp-gateway-sse.service $SERVER:$DEPLOY_DIR/infra/
rsync -avz --progress ./infra/nginx-mcp-gateway.conf $SERVER:$DEPLOY_DIR/infra/

# 7. Create .env file if it doesn't exist
echo ""
echo "🔐 Setting up environment file..."
ssh $SERVER "if [ ! -f $DEPLOY_DIR/providers/binance-rs/.env ]; then cp $DEPLOY_DIR/providers/binance-rs/.env.example $DEPLOY_DIR/providers/binance-rs/.env || touch $DEPLOY_DIR/providers/binance-rs/.env; fi"

# 8. Install uv if not present
echo ""
echo "📦 Ensuring uv is installed..."
ssh $SERVER "which uv || curl -LsSf https://astral.sh/uv/install.sh | sh"

# 9. Install Python dependencies
echo ""
echo "📦 Installing Python dependencies..."
ssh $SERVER "cd $DEPLOY_DIR/mcp-gateway && /root/.local/bin/uv sync"

# 10. Configure nginx
echo ""
echo "🌐 Configuring nginx..."
ssh $SERVER "cp $DEPLOY_DIR/infra/nginx-mcp-gateway.conf /etc/nginx/sites-available/mcp-gateway"
ssh $SERVER "ln -sf /etc/nginx/sites-available/mcp-gateway /etc/nginx/sites-enabled/mcp-gateway"
ssh $SERVER "nginx -t && systemctl reload nginx"

# 11. Install systemd services
echo ""
echo "⚙️  Installing systemd services..."
ssh $SERVER "cp $DEPLOY_DIR/infra/binance-provider.service /etc/systemd/system/"
ssh $SERVER "cp $DEPLOY_DIR/infra/mcp-gateway-sse.service /etc/systemd/system/"
ssh $SERVER "systemctl daemon-reload"
ssh $SERVER "systemctl enable binance-provider.service"
ssh $SERVER "systemctl enable mcp-gateway-sse.service"

# 12. Start services
echo ""
echo "▶️  Starting services..."
ssh $SERVER "systemctl restart binance-provider.service"
sleep 5
ssh $SERVER "systemctl restart mcp-gateway-sse.service"
sleep 5

# 13. Check service status
echo ""
echo "✅ Checking service status..."
echo ""
echo "=== Binance Provider Status ==="
ssh $SERVER "systemctl status binance-provider.service --no-pager -l || true"
echo ""
echo "=== MCP Gateway SSE Status ==="
ssh $SERVER "systemctl status mcp-gateway-sse.service --no-pager -l || true"

# 14. Check if services are listening on expected ports
echo ""
echo "🔍 Checking listening ports..."
ssh $SERVER "ss -tlnp | grep -E '(50053|3001)' || echo 'Services not listening on expected ports'"

# 15. Test health endpoint
echo ""
echo "🏥 Testing health endpoint..."
ssh $SERVER "curl -s http://localhost:3001/health || echo 'Health check failed'"

# 16. Configure HTTPS with certbot (if not already done)
echo ""
echo "🔒 Configuring HTTPS with certbot..."
ssh $SERVER "certbot --nginx -d mcp-gateway.thevibe.trading --non-interactive --agree-tos --redirect --email admin@thevibe.trading 2>&1 || echo 'Certbot may already be configured'"

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📊 View logs with:"
echo "   Binance Provider: ssh $SERVER journalctl -u binance-provider.service -f"
echo "   MCP Gateway SSE: ssh $SERVER journalctl -u mcp-gateway-sse.service -f"
echo ""
echo "🌐 Service URLs:"
echo "   SSE Endpoint: https://mcp-gateway.thevibe.trading/sse"
echo "   Health Check: https://mcp-gateway.thevibe.trading/health"
echo ""
echo "🔧 Test SSE endpoint:"
echo "   curl https://mcp-gateway.thevibe.trading/health"
echo ""
