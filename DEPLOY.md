# Deployment Guide

Quick deployment guide for mcp-trader with Traefik and SSL.

## Prerequisites

1. **Server Requirements**
   - Docker and Docker Compose installed
   - Ports 80, 443 open in firewall
   - Domain DNS configured

2. **DNS Configuration**
   ```
   api.context8.markets      A    YOUR_SERVER_IP
   traefik.context8.markets  A    YOUR_SERVER_IP
   ```

3. **Firewall**
   ```bash
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   sudo ufw enable
   ```

## Deployment Steps

### 1. Clone Repository

```bash
git clone <repository-url>
cd mcp-trader
```

### 2. Configure Environment

```bash
# Copy environment template
cp .env.example .env

# Edit configuration
nano .env
```

**Required settings in .env:**
- `ACME_EMAIL` - Your email for Let's Encrypt notifications
- `TRAEFIK_BASIC_AUTH` - Already configured (see TRAEFIK_CREDENTIALS.txt)
- `BINANCE_API_KEY` - Your Binance API key (optional for testing)
- `BINANCE_API_SECRET` - Your Binance API secret (optional for testing)

### 3. Build and Start Services

```bash
# Pull images and build
docker compose up -d

# Check logs
docker compose logs -f
```

**Note:** First build will take 5-10 minutes as it compiles Rust code.

### 4. Verify Deployment

```bash
# Check all services are running
docker compose ps

# Check Traefik logs for certificate acquisition
docker compose logs traefik | grep -i certificate

# Test API endpoint
curl -I https://api.context8.markets/health
```

## Access Points

- **MCP Gateway API**: https://api.context8.markets
- **Traefik Dashboard**: https://traefik.context8.markets
  - Credentials in `TRAEFIK_CREDENTIALS.txt`

## Common Commands

```bash
# View logs
docker compose logs -f [service_name]

# Restart services
docker compose restart

# Stop services
docker compose down

# Update and rebuild
git pull
docker compose up -d --build

# View resource usage
docker stats
```

## Troubleshooting

### Certificates Not Issued

1. Check DNS is propagated:
   ```bash
   dig +short api.context8.markets
   ```

2. Check Traefik logs:
   ```bash
   docker compose logs traefik | grep -i error
   ```

3. Verify ports are accessible:
   ```bash
   netstat -tlnp | grep -E ':(80|443)'
   ```

### Build Fails

If Rust build fails due to memory constraints:

```bash
# Add swap space
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Service Won't Start

```bash
# Check service logs
docker compose logs [service_name]

# Check service health
docker compose ps
docker inspect [container_name]
```

## Monitoring

```bash
# Real-time logs
docker compose logs -f

# Service status
docker compose ps

# Resource usage
docker stats

# Disk usage
docker system df
```

## Updates

```bash
# Pull latest changes
git pull

# Rebuild and restart
docker compose down
docker compose up -d --build

# Clean old images
docker image prune -a
```

## Security Checklist

- [ ] Changed Traefik dashboard password
- [ ] Configured firewall (only 80, 443 open)
- [ ] Set proper ACME_EMAIL in .env
- [ ] Secured Binance API credentials
- [ ] HTTPS working with valid certificates
- [ ] Regular backups configured

## Backup

### Backup Certificates

```bash
docker run --rm \
  -v mcp-trader_traefik-certs:/data \
  -v $(pwd):/backup \
  busybox tar czf /backup/certs-backup.tar.gz -C /data .
```

### Backup Analytics Data

```bash
docker run --rm \
  -v mcp-trader_binance-analytics:/data \
  -v $(pwd):/backup \
  busybox tar czf /backup/analytics-backup.tar.gz -C /data .
```

## Support

- [Traefik Documentation](TRAEFIK.md)
- [Docker Documentation](DOCKER.md)
- Project Issues: [GitHub Issues](https://github.com/your-repo/issues)
