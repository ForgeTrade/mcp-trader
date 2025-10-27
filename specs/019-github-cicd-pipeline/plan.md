# Implementation Plan: GitHub CI/CD Pipeline with Container Registry

**Branch**: `019-github-cicd-pipeline` | **Date**: 2025-10-27 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/019-github-cicd-pipeline/spec.md`

## Summary

Implement a complete CI/CD pipeline using GitHub Actions that automatically builds Docker images for all services (binance-provider, mcp-gateway, traefik), pushes them to GitHub Container Registry (ghcr.io), and enables server deployment by pulling pre-built images. This eliminates server-side builds, reduces deployment time, and ensures consistent, reproducible builds across environments.

**Technical Approach**: Use GitHub Actions workflows triggered on push to build multi-architecture Docker images, authenticate with GHCR using GitHub tokens, tag images with commit SHA and semantic versions, then update docker-compose.yml to reference registry images instead of local build contexts. Server deployment pulls images and restarts services without any local compilation.

## Technical Context

**Language/Version**:
- GitHub Actions (workflow orchestration)
- Shell/Bash scripting (deployment automation on server)
- Docker 20.10+ and Docker Compose V2 (containerization)
- Rust 1.75+ (binance-provider - already established)
- Python 3.11+ (mcp-gateway - already established)

**Primary Dependencies**:
- GitHub Actions (`actions/checkout`, `docker/login-action`, `docker/build-push-action`, `docker/setup-buildx-action`)
- GitHub Container Registry (ghcr.io - image storage)
- Docker Compose V2 (service orchestration)
- SSH (for deployment trigger communication - standard)

**Storage**:
- GitHub Container Registry (ghcr.io) for container images
- Local server volumes for runtime data (binance-analytics, traefik-certs - already configured)

**Testing**:
- GitHub Actions workflow validation (test workflow syntax)
- Integration testing (verify built images can be pulled and run)
- Health checks in docker-compose (already configured - binance-provider, mcp-gateway)

**Target Platform**:
- CI: GitHub-hosted runners (Linux AMD64)
- Deployment: Linux server (AMD64 - based on existing setup)
- Container runtime: Docker 20.10+ with Docker Compose V2

**Project Type**: Multi-service containerized application (microservices architecture)

**Performance Goals**:
- CI builds complete within 5 minutes for incremental changes
- Server deployments complete within 2 minutes (excluding first pull)
- Image pull operations retry automatically on failure
- Zero downtime for services during deployment (docker-compose up strategy)

**Constraints**:
- Must not build on server (CPU/memory constraints assumed from requirement)
- Must use GitHub Container Registry (specified in requirements)
- Must preserve existing volumes and configurations during updates
- Must support both commit SHA and semantic version tagging
- Images must be publicly accessible or server must have GHCR credentials configured

**Scale/Scope**:
- 3 services to build and deploy (traefik uses official image, binance-provider and mcp-gateway need builds)
- Single deployment environment (production server)
- Image sizes: ~100MB for binance-provider (Rust), ~50MB for mcp-gateway (Python)
- Expected push frequency: multiple deployments per day

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Evaluation Against Core Principles

**I. Simplicity and Readability** ✅
- GitHub Actions workflows use declarative YAML syntax (industry standard)
- Deployment scripts will be straightforward shell scripts with clear variable names
- No complex abstractions needed - linear CI/CD pipeline flow
- **Passes**: Solution is simple and follows established patterns

**II. Library-First Development** ✅
- Using official GitHub Actions (actions/checkout, docker/build-push-action, etc.)
- Using Docker official build tools (buildx, build-push-action)
- Not reinventing container registry or CI/CD infrastructure
- **Passes**: Leveraging existing, battle-tested GitHub Actions and Docker tooling

**III. Justified Abstractions** ✅
- No speculative abstractions planned
- Using concrete, standard CI/CD patterns (build → tag → push → deploy)
- Each workflow step maps directly to a functional requirement
- **Passes**: No premature abstractions, only concrete implementation steps

**IV. DRY Principle** ✅
- GitHub Actions supports workflow reuse and composite actions
- Will extract common build patterns if duplication emerges across multiple service builds
- Deployment scripts will use variables for repeated paths/commands
- **Passes**: Plan accounts for eliminating duplication when it appears

**V. Service and Repository Patterns** N/A
- This feature is infrastructure/DevOps focused, not data persistence
- No business logic or data access layers involved
- **Not Applicable**: No data persistence or business logic in this feature

**VI. 12-Factor Methodology** ✅
- **Config**: GitHub secrets for credentials, environment variables for configuration
- **Build, release, run**: Strict separation (CI builds, registry stores releases, server runs)
- **Processes**: Stateless container deployment (existing design)
- **Disposability**: Fast startup with pre-built images (removes build time)
- **Dev/prod parity**: Same images used across environments
- **Logs**: Logs to stdout (already configured in existing services)
- **Passes**: Feature enhances 12-factor compliance, especially build/release/run separation

**VII. Minimal Object-Oriented Programming** ✅
- Implementation uses declarative workflows (YAML) and procedural shell scripts
- No OOP needed for CI/CD pipeline automation
- **Passes**: No unnecessary OOP, uses appropriate tools (YAML configs, shell scripts)

### Gate Decision

**RESULT**: ✅ **ALL GATES PASS** - Proceed to Phase 0 research

No constitutional violations detected. The CI/CD pipeline implementation aligns with all applicable principles and uses industry-standard, simple, library-first approaches.

## Project Structure

### Documentation (this feature)

```
specs/019-github-cicd-pipeline/
├── plan.md              # This file
├── research.md          # Phase 0 output (best practices, patterns)
├── quickstart.md        # Phase 1 output (setup and deployment guide)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

Note: `data-model.md` and `contracts/` are not applicable for this infrastructure feature as there are no domain entities or API contracts to define.

### Source Code (repository root)

```
.github/
└── workflows/
    ├── build-and-push.yml       # Main CI workflow (build on push)
    └── deploy.yml               # Optional: Deployment workflow (manual or webhook-triggered)

scripts/
└── deploy/
    ├── pull-and-restart.sh      # Server-side deployment script
    └── rollback.sh              # Rollback to previous image tags

compose.yml                       # Updated with ghcr.io image references
compose.override.yml              # Optional: Local development override (build context)

.env.example                      # Updated with registry credentials template
```

**Structure Decision**: Added `.github/workflows/` for CI/CD automation (standard GitHub Actions location) and `scripts/deploy/` for server-side deployment automation. Existing `compose.yml` will be updated to reference registry images. Using `compose.override.yml` for local development to preserve local build capability for developers.

## Complexity Tracking

*No violations identified - section left empty as per constitution compliance.*

This feature introduces no constitutional violations. The implementation uses simple, library-first approaches with standard GitHub Actions and Docker tooling.

## Phase 0: Research & Technical Decisions

See [research.md](./research.md) for detailed research findings on:
- GitHub Actions best practices for Docker multi-service builds
- GitHub Container Registry authentication patterns
- Image tagging strategies (commit SHA, semantic versioning, environment tags)
- Deployment triggering mechanisms (webhook, manual, SSH-based)
- Docker Compose image pull and restart strategies
- Rollback procedures for failed deployments

## Phase 1: Design Artifacts

### Quickstart Guide

See [quickstart.md](./quickstart.md) for:
- GitHub repository secrets configuration (GHCR credentials)
- CI workflow setup and testing
- Server deployment preparation (Docker login to GHCR, deployment script setup)
- First deployment walkthrough
- Verification and health check procedures

### Data Model

**Not Applicable**: This feature is infrastructure-focused with no domain entities or data persistence requirements.

### API Contracts

**Not Applicable**: This feature does not introduce new APIs. It automates existing container deployment workflows.

## Implementation Notes

### GitHub Actions Workflow Structure

The main CI workflow will:
1. Trigger on push to `main` branch (and optionally feature branches for testing)
2. Check out repository code
3. Set up Docker Buildx for multi-architecture support
4. Authenticate with GitHub Container Registry using `GITHUB_TOKEN`
5. Build images for each service (binance-provider, mcp-gateway)
6. Tag images with:
   - Commit SHA (e.g., `ghcr.io/owner/repo/binance-provider:sha-abc1234`)
   - Branch name (e.g., `ghcr.io/owner/repo/binance-provider:main`)
   - `latest` tag for main branch
7. Push images to GitHub Container Registry
8. Report build status (success/failure) in GitHub UI

### Deployment Strategy

Server-side deployment will:
1. Receive notification (manual trigger, webhook, or scheduled poll)
2. Authenticate with GHCR (one-time setup using Personal Access Token or GitHub App)
3. Pull latest images for specified tags
4. Run `docker-compose pull` to fetch new images
5. Run `docker-compose up -d` to restart services with new images
6. Verify health checks pass for all services
7. Log deployment status to server logs and report to GitHub (optional)

### Configuration Management

- **CI Configuration**: GitHub repository secrets for GHCR authentication
- **Server Configuration**: Environment variables in `.env` file for registry credentials
- **Service Configuration**: Preserved through docker-compose volumes (existing setup)

### Rollback Plan

If deployment fails or issues are detected:
1. Identify previous working image tag (commit SHA or version)
2. Update docker-compose.yml or environment variables with previous tag
3. Run `docker-compose pull` and `docker-compose up -d`
4. Verify service health checks
