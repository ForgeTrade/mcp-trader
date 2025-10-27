# Quickstart: GitHub CI/CD Pipeline Setup and Deployment

**Feature**: 019-github-cicd-pipeline
**Date**: 2025-10-27
**Audience**: Developers and DevOps engineers setting up or using the CI/CD pipeline

## Overview

This guide walks through setting up the GitHub Actions CI/CD pipeline, configuring GitHub Container Registry access, and performing your first deployment. After setup, every push to `main` will automatically build and push Docker images to GHCR.

## Prerequisites

- GitHub repository with push access
- GitHub account with repository admin permissions (for configuring secrets)
- Server with Docker and Docker Compose V2 installed
- SSH access to deployment server
- Basic familiarity with Docker, GitHub Actions, and shell scripts

## Part 1: GitHub Repository Setup

### Step 1: Verify Repository Permissions

GitHub Container Registry (GHCR) requires specific permissions for the `GITHUB_TOKEN` used in workflows.

1. Go to repository **Settings** → **Actions** → **General**
2. Scroll to **Workflow permissions**
3. Ensure **Read and write permissions** is selected
4. Enable **Allow GitHub Actions to create and approve pull requests** (optional, but recommended)
5. Click **Save**

**Why**: The `GITHUB_TOKEN` needs write permissions to push images to GHCR.

### Step 2: Enable Package Visibility (Optional)

If you want images to be publicly accessible (no authentication required to pull):

1. Go to your profile → **Packages**
2. Find packages created by this repo (they'll appear after first push)
3. Click on package → **Package settings**
4. Change visibility to **Public** (if desired)

**Note**: For private images, the server will need authentication (covered in Part 2).

### Step 3: No Additional Secrets Needed for CI

The GitHub Actions workflow uses `GITHUB_TOKEN` which is automatically available. No manual secret configuration is needed for the CI build process.

**Verification**: You're ready to proceed once workflow permissions are set correctly.

---

## Part 2: Server Deployment Setup

### Step 1: Create GitHub Personal Access Token (PAT)

The server needs credentials to pull images from GHCR.

1. Go to GitHub **Settings** (your profile, not repository)
2. Navigate to **Developer settings** → **Personal access tokens** → **Tokens (classic)**
3. Click **Generate new token** → **Generate new token (classic)**
4. Set token name: `mcp-trader-deployment-server`
5. Set expiration: **No expiration** (or set long expiration like 1 year)
6. Select scopes: **Only `read:packages`** (minimal permission for pulling images)
7. Click **Generate token**
8. **Copy the token immediately** (you won't see it again!)

**Security Note**: This token only allows reading/pulling packages, not writing or accessing code.

### Step 2: Configure Server Authentication

SSH into your deployment server and log in to GHCR:

```bash
# SSH to server
ssh user@your-server.com

# Login to GitHub Container Registry
echo "YOUR_PAT_TOKEN" | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin

# Verify login
docker pull ghcr.io/limerc/mcp-trader/binance-provider:latest || echo "No images yet - this is expected before first build"
```

**Verification**: You should see `Login Succeeded`. Credentials are stored in `~/.docker/config.json`.

### Step 3: Add Registry Credentials to .env (Optional)

For easier management, add credentials to your `.env` file:

```bash
# Edit .env on server
cd /opt/mcp-trader
nano .env
```

Add these lines:

```bash
# GitHub Container Registry credentials
GHCR_USERNAME=YOUR_GITHUB_USERNAME
GHCR_PAT=YOUR_PAT_TOKEN

# Image tags (for version pinning)
BINANCE_IMAGE_TAG=latest
GATEWAY_IMAGE_TAG=latest
```

**Note**: Keep `.env` out of version control (already in .gitignore).

### Step 4: Create Deployment Scripts Directory

```bash
# On server
cd /opt/mcp-trader
mkdir -p scripts/deploy
```

The actual deployment scripts will be created during implementation (see tasks.md).

---

## Part 3: First CI Build

### Step 1: Push Code to Trigger Workflow

After the CI workflow file is created (`.github/workflows/build-and-push.yml`), push to main:

```bash
git add .github/workflows/build-and-push.yml
git commit -m "feat: Add GitHub Actions CI/CD workflow"
git push origin main
```

### Step 2: Monitor Build Progress

1. Go to repository **Actions** tab
2. Click on the workflow run (e.g., "feat: Add GitHub Actions CI/CD workflow")
3. Watch the build matrix jobs for `binance-provider` and `mcp-gateway`

**Expected Timeline**:
- Setup: ~30 seconds
- binance-provider build: ~5-7 minutes (first build), ~2-3 minutes (cached)
- mcp-gateway build: ~1-2 minutes
- Total: ~7-9 minutes for first build, ~3-5 minutes for incremental builds

### Step 3: Verify Images in GHCR

After successful build:

1. Go to repository main page
2. Look for **Packages** section in the right sidebar
3. Click on packages to see available tags:
   - `latest` (most recent main branch build)
   - `main` (main branch)
   - `sha-<commit-hash>` (specific commit)

**Verification**: Images should be available at:
- `ghcr.io/OWNER/REPO/binance-provider:latest`
- `ghcr.io/OWNER/REPO/mcp-gateway:latest`

---

## Part 4: First Deployment

### Step 1: Update compose.yml for Registry Images

On the server, update `compose.yml` to use registry images instead of local builds:

```yaml
services:
  binance-provider:
    image: ghcr.io/OWNER/REPO/binance-provider:${BINANCE_IMAGE_TAG:-latest}
    # Remove or comment out 'build:' section
    container_name: binance-provider
    # ... rest of configuration unchanged

  mcp-gateway:
    image: ghcr.io/OWNER/REPO/mcp-gateway:${GATEWAY_IMAGE_TAG:-latest}
    # Remove or comment out 'build:' section
    container_name: mcp-gateway
    # ... rest of configuration unchanged
```

**Note**: Replace `OWNER/REPO` with your GitHub username/organization and repository name (e.g., `limerc/mcp-trader`).

### Step 2: Pull Images

```bash
cd /opt/mcp-trader
docker-compose pull binance-provider mcp-gateway
```

**Expected Output**:
```
Pulling binance-provider ... done
Pulling mcp-gateway ... done
```

### Step 3: Restart Services

```bash
docker-compose up -d
```

**Expected Output**:
```
Recreating binance-provider ... done
Recreating mcp-gateway ... done
```

### Step 4: Verify Services Are Running

```bash
# Check service status
docker-compose ps

# Check logs
docker-compose logs -f binance-provider mcp-gateway
```

**Expected Status**: All services should show `Up` and health checks should pass.

---

## Part 5: Verification and Health Checks

### Step 1: Verify Image Sources

Confirm services are running registry images, not locally-built ones:

```bash
docker-compose images
```

**Expected Output**: Image repositories should show `ghcr.io/OWNER/REPO/...`, not local names.

### Step 2: Test Service Endpoints

```bash
# Test binance-provider (gRPC health check)
# Assuming grpcurl is installed
grpcurl -plaintext localhost:50053 list

# Test mcp-gateway (HTTP health check)
curl http://localhost:3001/health || echo "Endpoint may not exist yet - check service logs"
```

### Step 3: Check Container Logs

```bash
# View recent logs
docker-compose logs --tail=50 binance-provider mcp-gateway

# Follow logs in real-time
docker-compose logs -f
```

**What to Look For**:
- No error messages during startup
- Services listening on expected ports
- Health checks passing

---

## Part 6: Ongoing Operations

### Deploying New Changes

After pushing new code to `main`:

1. **Wait for CI build to complete** (monitor in GitHub Actions tab)
2. **SSH to server**:
   ```bash
   ssh user@your-server.com
   cd /opt/mcp-trader
   ```
3. **Run deployment script** (created during implementation):
   ```bash
   ./scripts/deploy/pull-and-restart.sh
   ```

Or manually:

```bash
docker-compose pull
docker-compose up -d
docker-compose ps  # Verify health
```

### Rolling Back

If a deployment causes issues, rollback to a previous version:

```bash
# Option 1: Use rollback script (created during implementation)
./scripts/deploy/rollback.sh abc1234  # Use commit SHA from git log

# Option 2: Manual rollback
export BINANCE_IMAGE_TAG=sha-abc1234
export GATEWAY_IMAGE_TAG=sha-abc1234
docker-compose pull
docker-compose up -d
```

### Viewing Deployment History

Keep a log of deployments for easy rollback reference:

```bash
# Log deployments (add to deployment script)
echo "$(date): Deployed sha-$(git rev-parse --short HEAD)" >> /opt/mcp-trader/deployment.log

# View deployment history
cat /opt/mcp-trader/deployment.log
```

---

## Part 7: Troubleshooting

### Issue: CI Build Fails with Permission Error

**Symptom**: GitHub Actions fails with "permission denied" when pushing to GHCR

**Solution**:
1. Check repository workflow permissions (Settings → Actions → General)
2. Ensure "Read and write permissions" is enabled
3. Re-run workflow

### Issue: Server Can't Pull Images (Unauthorized)

**Symptom**: `docker-compose pull` fails with "unauthorized: authentication required"

**Solution**:
1. Verify PAT has `read:packages` scope
2. Re-login to GHCR:
   ```bash
   docker logout ghcr.io
   echo "YOUR_PAT" | docker login ghcr.io -u YOUR_USERNAME --password-stdin
   ```
3. Check if package is private (may need to make it public or ensure PAT is valid)

### Issue: Build Takes Too Long

**Symptom**: Rust builds take 10+ minutes

**Solution**:
1. Verify build caching is enabled in workflow (cache-from/cache-to settings)
2. Check if BuildKit is enabled (`docker/setup-buildx-action`)
3. Consider using GitHub Actions larger runners (for paid accounts)

### Issue: Service Doesn't Start After Deployment

**Symptom**: Container exits immediately after `docker-compose up -d`

**Solution**:
1. Check logs: `docker-compose logs binance-provider`
2. Verify environment variables in `.env` are still correct
3. Check if volumes have correct permissions
4. Try running container with previous image tag to isolate issue

### Issue: Old Images Not Cleaned Up

**Symptom**: Server disk space fills up with old images

**Solution**:
```bash
# Remove unused images
docker image prune -a

# Or remove specific old tags
docker rmi ghcr.io/OWNER/REPO/binance-provider:sha-old1234
```

---

## Summary Checklist

Setup (one-time):
- [  ] Configured GitHub repository workflow permissions
- [  ] Created GitHub PAT with `read:packages` scope
- [  ] Authenticated server with GHCR (`docker login ghcr.io`)
- [  ] Updated `compose.yml` to use registry images
- [  ] Created deployment scripts directory

Per Deployment:
- [  ] Code pushed to `main` branch
- [  ] GitHub Actions build completed successfully
- [  ] Images visible in GHCR packages
- [  ] Pulled latest images on server
- [  ] Restarted services with `docker-compose up -d`
- [  ] Verified services are healthy
- [  ] Logged deployment for rollback reference

Troubleshooting Reference:
- GitHub Actions logs: Repository → Actions tab
- Container logs: `docker-compose logs <service>`
- Image verification: `docker-compose images`
- Health checks: `docker-compose ps`

For detailed implementation steps, see [tasks.md](./tasks.md) (generated by `/speckit.tasks`).
