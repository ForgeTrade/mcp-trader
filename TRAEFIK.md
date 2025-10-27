# Traefik Reverse Proxy Configuration

This project uses Traefik v3.0 as a reverse proxy with automatic SSL certificate management via Let's Encrypt.

## Features

- **Automatic SSL Certificates**: Let's Encrypt integration with HTTP challenge
- **HTTP to HTTPS Redirect**: All HTTP traffic automatically redirected to HTTPS
- **Domain Routing**:
  - `api.context8.markets` → MCP Gateway (SSE server)
  - `traefik.context8.markets` → Traefik Dashboard (optional)
- **CORS Support**: Pre-configured CORS middleware for SSE endpoints
- **Persistent Storage**: Certificates stored in Docker volume

## Prerequisites

Before starting, ensure:

1. **DNS Configuration**: Point your domain to the server
   ```
   api.context8.markets      A     YOUR_SERVER_IP
   traefik.context8.markets  A     YOUR_SERVER_IP
   ```

2. **Firewall Rules**: Open ports 80 and 443
   ```bash
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   ```

3. **Environment Variables**: Copy `.env.example` to `.env` and configure
   ```bash
   cp .env.example .env
   nano .env
   ```

## Configuration

### Required Environment Variables

Edit `.env` file:

```bash
# Let's Encrypt email for certificate notifications
ACME_EMAIL=your-email@domain.com

# Traefik dashboard authentication (optional)
TRAEFIK_BASIC_AUTH=admin:$$apr1$$8EVjn/nj$$GiLUZqcbueTFeD23SuB6x0
```

### Generate Dashboard Password

To create a new password for Traefik dashboard:

```bash
# Install htpasswd
sudo apt-get install apache2-utils

# Generate password (replace 'your_password')
htpasswd -nb admin your_password

# Copy output to .env file as TRAEFIK_BASIC_AUTH
# Note: Escape $ symbols with $$ in docker-compose
```

## Deployment

### Start Services

```bash
docker compose up -d
```

### Verify SSL Certificates

Traefik will automatically request certificates on first HTTPS request. Monitor logs:

```bash
docker compose logs -f traefik
```

Look for:
```
time="..." level=info msg="Certificates obtained for domains [api.context8.markets]"
```

### Check Certificate Status

```bash
# View certificate storage
docker compose exec traefik ls -la /letsencrypt/

# Check acme.json
docker compose exec traefik cat /letsencrypt/acme.json
```

## Access Points

Once deployed:

- **MCP Gateway API**: https://api.context8.markets
- **Traefik Dashboard**: https://traefik.context8.markets (requires basic auth)

### Test MCP Gateway

```bash
# Test SSL connection
curl -I https://api.context8.markets/health

# Test SSE stream
curl -N https://api.context8.markets/sse
```

## Architecture

```
Internet
   ↓
Traefik (ports 80, 443)
   ├→ HTTP Challenge (Let's Encrypt)
   ├→ api.context8.markets → mcp-gateway:3001
   └→ traefik.context8.markets → Traefik Dashboard
```

## Middleware

### CORS Middleware (mcp-cors)

Pre-configured for SSE compatibility:
- Allow all origins (`*`)
- Allow methods: GET, POST, OPTIONS
- Allow all headers
- Max age: 100s

To restrict origins, edit `compose.yml`:

```yaml
- "traefik.http.middlewares.mcp-cors.headers.accesscontrolalloworiginlist=https://your-domain.com,https://another-domain.com"
```

## Certificate Renewal

Let's Encrypt certificates are automatically renewed by Traefik:
- Certificates valid for 90 days
- Auto-renewal starts 30 days before expiration
- No manual intervention required

### Force Certificate Renewal

If needed, remove `acme.json` to force re-issue:

```bash
docker compose down
docker volume rm mcp-trader_traefik-certs
docker compose up -d
```

## Troubleshooting

### Certificates Not Issued

1. **Check DNS**: Ensure domain points to server
   ```bash
   dig +short api.context8.markets
   ```

2. **Check Ports**: Verify 80/443 are accessible
   ```bash
   netstat -tlnp | grep -E ':(80|443)'
   ```

3. **Check Logs**:
   ```bash
   docker compose logs traefik | grep -i error
   ```

### Common Issues

**Issue**: "acme: error: 403 :: urn:ietf:params:acme:error:unauthorized"
- **Cause**: DNS not pointing to server or port 80 not accessible
- **Fix**: Verify DNS and firewall rules

**Issue**: "too many certificates already issued"
- **Cause**: Let's Encrypt rate limit (5 per week for same domain)
- **Fix**: Wait one week or use staging environment for testing

### Testing with Let's Encrypt Staging

For testing, use staging environment to avoid rate limits:

```yaml
# In compose.yml, add to traefik command:
- "--certificatesresolvers.letsencrypt.acme.caserver=https://acme-staging-v02.api.letsencrypt.org/directory"
```

## Security Considerations

1. **Change default dashboard password** in `.env`
2. **Restrict dashboard access** or disable if not needed
3. **Configure firewall** to allow only ports 80/443
4. **Backup certificates**:
   ```bash
   docker run --rm -v mcp-trader_traefik-certs:/data -v $(pwd):/backup busybox tar czf /backup/traefik-certs-backup.tar.gz -C /data .
   ```

## Adding New Services

To route additional services through Traefik:

```yaml
services:
  my-service:
    image: my-image
    networks:
      - mcp-network
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.my-service.rule=Host(`my-service.context8.markets`)"
      - "traefik.http.routers.my-service.entrypoints=websecure"
      - "traefik.http.routers.my-service.tls.certresolver=letsencrypt"
      - "traefik.http.services.my-service.loadbalancer.server.port=8080"
```

## Monitoring

### View Real-time Logs

```bash
# All Traefik logs
docker compose logs -f traefik

# Access logs only
docker compose logs -f traefik | grep "accesslog"
```

### Dashboard

Access Traefik dashboard at https://traefik.context8.markets to:
- View active routers and services
- Monitor certificate status
- Check middleware configuration
- View real-time metrics

## References

- [Traefik Documentation](https://doc.traefik.io/traefik/)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [Docker Provider Reference](https://doc.traefik.io/traefik/providers/docker/)
