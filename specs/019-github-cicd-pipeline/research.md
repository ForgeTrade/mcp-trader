# Research: GitHub CI/CD Pipeline Implementation

**Feature**: 019-github-cicd-pipeline
**Date**: 2025-10-27
**Purpose**: Research best practices, patterns, and technical decisions for GitHub Actions CI/CD with Container Registry

## Overview

This document consolidates research findings for implementing automated CI/CD using GitHub Actions, GitHub Container Registry, and Docker Compose deployment strategies.

## 1. GitHub Actions Best Practices for Docker Builds

### Decision: Use docker/build-push-action with BuildKit and cache layers

**Rationale**:
- `docker/build-push-action` is the official, maintained GitHub Action for Docker builds
- BuildKit provides parallel builds, improved caching, and better performance
- Layer caching significantly reduces build times (especially for Rust builds which can be slow)
- Supports multi-platform builds (if needed in future for ARM servers)

**Implementation**:
```yaml
- uses: docker/setup-buildx-action@v3  # Enables BuildKit
- uses: docker/build-push-action@v5
  with:
    context: .
    file: ./providers/binance-rs/Dockerfile
    push: true
    tags: ghcr.io/${{ github.repository }}/binance-provider:${{ github.sha }}
    cache-from: type=registry,ref=ghcr.io/${{ github.repository }}/binance-provider:buildcache
    cache-to: type=registry,ref=ghcr.io/${{ github.repository }}/binance-provider:buildcache,mode=max
```

**Alternatives Considered**:
- **Manual docker build + push commands**: Rejected because build-push-action provides better caching, parallel builds, and error handling
- **Third-party CI systems (CircleCI, Travis)**: Rejected because GitHub Actions is native to GitHub, requires no additional accounts, and provides free minutes for public repos

**Performance Benefits**:
- First build of binance-provider (Rust): ~5-7 minutes
- Cached incremental builds: ~2-3 minutes (dependency layers cached)
- mcp-gateway (Python): ~1-2 minutes (smaller, simpler builds)

---

## 2. GitHub Container Registry Authentication

### Decision: Use GITHUB_TOKEN for CI, Personal Access Token (PAT) for server

**Rationale**:
- `GITHUB_TOKEN` is automatically available in GitHub Actions with appropriate permissions
- No need to create or manage additional secrets for CI workflows
- For server deployment, PAT with `read:packages` scope provides secure, scoped access
- GHCR supports both token types seamlessly

**CI Authentication** (automatic):
```yaml
- uses: docker/login-action@v3
  with:
    registry: ghcr.io
    username: ${{ github.actor }}
    password: ${{ secrets.GITHUB_TOKEN }}
```

**Server Authentication** (one-time setup):
```bash
# On deployment server
echo $GITHUB_PAT | docker login ghcr.io -u USERNAME --password-stdin
# Credentials stored in ~/.docker/config.json for docker-compose to use
```

**Alternatives Considered**:
- **GitHub App tokens**: Rejected as unnecessarily complex for single-server deployment
- **Public images (no auth)**: Considered but rejected to maintain control over who can pull images (private repos)

**Security Notes**:
- GITHUB_TOKEN has automatic permissions management (no manual configuration needed)
- PAT for server should be stored securely (.env file, not committed)
- PAT should have minimal scopes (only `read:packages` for pull access)

---

## 3. Image Tagging Strategy

### Decision: Multi-tag approach with commit SHA, branch, and latest

**Rationale**:
- **Commit SHA tags** (`sha-abc1234`) provide immutable, traceable references to exact code versions
- **Branch tags** (`main`, `develop`) provide convenient "latest stable" references
- **Latest tag** simplifies pulling most recent production image
- Semantic versions (`v1.2.3`) can be added later when release process is formalized

**Tagging Implementation**:
```yaml
tags: |
  ghcr.io/${{ github.repository }}/binance-provider:sha-${{ github.sha }}
  ghcr.io/${{ github.repository }}/binance-provider:${{ github.ref_name }}
  ghcr.io/${{ github.repository }}/binance-provider:latest
```

**Tag Usage Patterns**:
- **Production deployment**: Use `latest` or specific `sha-*` tags
- **Rollback**: Reference previous `sha-*` tag explicitly
- **Feature testing**: Use branch name tags (e.g., `019-github-cicd-pipeline`)

**Alternatives Considered**:
- **Date-based tags** (`2025-10-27-1`): Rejected as less traceable than commit SHAs
- **Build number tags** (`build-123`): Rejected because git commit SHA already provides unique, meaningful identifier

---

## 4. Deployment Triggering Mechanisms

### Decision: Manual trigger via SSH + deployment script (simple, reliable)

**Rationale**:
- Manual trigger provides explicit control over when deployments happen
- SSH is already set up for server access (no additional infrastructure needed)
- Deployment script can be version-controlled and tested locally
- Simple to understand and debug - no webhooks, no polling services

**Implementation**:
```bash
# On developer machine or CI (after successful build)
ssh user@server '/opt/mcp-trader/scripts/deploy/pull-and-restart.sh'

# Or manual deployment on server
cd /opt/mcp-trader
./scripts/deploy/pull-and-restart.sh
```

**Future Enhancements** (if needed):
- GitHub Actions workflow_dispatch for manual deployment from GitHub UI
- Webhook receiver on server to auto-deploy on successful builds
- GitHub repository dispatch for programmatic deployment triggers

**Alternatives Considered**:
- **Automatic deployment on every push**: Rejected because production should have explicit deployment approval
- **Webhook to server endpoint**: Rejected as over-engineered for single-server setup (requires HTTPS endpoint, auth, etc.)
- **Pull-based polling**: Rejected as wasteful (constant polling vs. explicit trigger)

**Decision Criteria**:
- Simplicity: Manual SSH trigger is simplest to implement and understand
- Reliability: No additional failure points (webhook receivers, polling services)
- Security: Uses existing SSH authentication (no new attack surfaces)

---

## 5. Docker Compose Image Pull and Restart Strategy

### Decision: docker-compose pull + docker-compose up -d (standard approach)

**Rationale**:
- `docker-compose pull` downloads new images without affecting running services
- `docker-compose up -d` restarts only services with updated images (minimal disruption)
- Preserves volumes and networks (existing data retained)
- Built-in health checks verify services start correctly

**Deployment Script** (`pull-and-restart.sh`):
```bash
#!/usr/bin/env bash
set -euo pipefail

cd /opt/mcp-trader
echo "Pulling latest images..."
docker-compose pull

echo "Restarting services with new images..."
docker-compose up -d

echo "Waiting for health checks..."
sleep 10

echo "Verifying service health..."
docker-compose ps

echo "Deployment complete!"
```

**Health Check Integration**:
- Existing health checks in docker-compose.yml verify services start correctly
- If health check fails, docker-compose marks service as unhealthy
- Manual inspection required for failed health checks (logged in `docker-compose ps`)

**Alternatives Considered**:
- **docker-compose down + up**: Rejected because it causes unnecessary downtime
- **Blue-green deployment**: Rejected as over-engineered for current scale (single server, low traffic)
- **Rolling updates**: Not applicable (Docker Compose doesn't support true rolling updates)

---

## 6. Rollback Procedures

### Decision: Manual rollback by specifying previous image tag in .env or compose file

**Rationale**:
- Simple, explicit control over which version to run
- Uses same deployment mechanism as forward deployments (consistent process)
- Git history provides record of which commits/SHAs worked previously

**Rollback Script** (`rollback.sh`):
```bash
#!/usr/bin/env bash
set -euo pipefail

if [ $# -eq 0 ]; then
    echo "Usage: $0 <commit-sha>"
    echo "Example: $0 abc1234"
    exit 1
fi

ROLLBACK_SHA=$1
cd /opt/mcp-trader

echo "Rolling back to commit SHA: $ROLLBACK_SHA"

# Update compose.yml with specific SHA tags (or use environment variables)
export BINANCE_IMAGE_TAG="sha-$ROLLBACK_SHA"
export GATEWAY_IMAGE_TAG="sha-$ROLLBACK_SHA"

docker-compose pull
docker-compose up -d

echo "Rollback complete! Services running image tags: sha-$ROLLBACK_SHA"
```

**Best Practices**:
- Keep a log of deployed SHAs with timestamps (`deployment.log`)
- Test rollback procedure during implementation phase
- Document rollback steps in quickstart.md

**Alternatives Considered**:
- **Automated rollback on health check failure**: Rejected as premature (need baseline deployment process first)
- **Keep previous containers running**: Rejected because volumes can't be shared (data conflicts)

---

## 7. Multi-Service Build Optimization

### Decision: Build services in parallel using matrix strategy

**Rationale**:
- GitHub Actions supports matrix builds for parallel execution
- Binance-provider (Rust) and mcp-gateway (Python) can build independently
- Reduces total CI time from sequential (7 + 2 = 9 min) to parallel (max(7, 2) = 7 min)

**Matrix Strategy**:
```yaml
strategy:
  matrix:
    service:
      - name: binance-provider
        context: .
        dockerfile: ./providers/binance-rs/Dockerfile
      - name: mcp-gateway
        context: ./mcp-gateway
        dockerfile: ./mcp-gateway/Dockerfile
```

**Alternatives Considered**:
- **Sequential builds**: Rejected because it unnecessarily prolongs CI time
- **Separate workflows per service**: Rejected because it duplicates workflow configuration (violates DRY)

---

## 8. Configuration Management

### Decision: Environment variables in .env file for runtime config, GitHub Secrets for CI credentials

**Rationale**:
- Follows 12-factor methodology (config in environment, not code)
- Existing setup already uses .env for BINANCE_API_KEY, etc.
- GitHub Secrets provide secure storage for CI credentials
- Separation of concerns: CI config (GitHub) vs. runtime config (server .env)

**CI Secrets** (configured in GitHub repository settings):
- `GITHUB_TOKEN` (automatic, no manual configuration)
- Optional: `DEPLOY_SSH_KEY` if implementing automated deployment

**Server .env** (additions):
```bash
# GitHub Container Registry authentication
GHCR_USERNAME=your-github-username
GHCR_PAT=your-github-personal-access-token

# Image tags (optional, for version pinning)
BINANCE_IMAGE_TAG=latest
GATEWAY_IMAGE_TAG=latest
```

**compose.yml updates**:
```yaml
services:
  binance-provider:
    image: ghcr.io/owner/repo/binance-provider:${BINANCE_IMAGE_TAG:-latest}
    # Remove 'build:' section
```

---

## Summary of Technical Decisions

| Decision Area | Choice | Rationale |
|---------------|--------|-----------|
| CI Platform | GitHub Actions | Native to GitHub, free, well-documented |
| Registry | GitHub Container Registry (ghcr.io) | Free, integrated, reliable |
| Build Tool | docker/build-push-action | Official, optimized, caching support |
| Authentication (CI) | GITHUB_TOKEN | Automatic, secure, scoped |
| Authentication (Server) | Personal Access Token | Simple, scoped (read:packages) |
| Image Tags | SHA + branch + latest | Traceable, flexible, production-ready |
| Deployment Trigger | Manual SSH + script | Simple, reliable, explicit control |
| Deployment Strategy | pull + up -d | Standard Docker Compose pattern |
| Rollback | Manual with specific SHA tags | Explicit control, uses existing tooling |
| Build Optimization | Parallel matrix builds | Faster CI, no duplication |
| Configuration | .env (server) + GitHub Secrets (CI) | 12-factor compliant, secure |

All decisions prioritize simplicity, use industry-standard tools, and align with the project's constitution principles (library-first, minimal abstraction, 12-factor methodology).
