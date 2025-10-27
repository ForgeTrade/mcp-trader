# Tasks: GitHub CI/CD Pipeline with Container Registry

**Input**: Design documents from `/specs/019-github-cicd-pipeline/`
**Prerequisites**: plan.md, spec.md, research.md, quickstart.md

**Tests**: Not explicitly requested in specification - tests will be validation-based (verify workflows run, images push, deployments succeed)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions
- CI/CD workflows: `.github/workflows/` (standard GitHub Actions location)
- Deployment scripts: `scripts/deploy/` (new directory for server automation)
- Configuration: `compose.yml` (existing), `.env.example` (existing)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and GitHub repository configuration

- [x] T001 Create .github/workflows directory at repository root
- [x] T002 Create scripts/deploy directory at repository root
- [x] T003 [P] Verify GitHub repository workflow permissions (Settings → Actions → General → Read and write permissions)
- [x] T004 [P] Document repository setup steps in quickstart.md (verify section exists and is accurate)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core GitHub Actions infrastructure that MUST be complete before any CI/CD automation can work

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 Create base GitHub Actions workflow structure with checkout and Docker setup in .github/workflows/build-and-push.yml
- [x] T006 Configure Docker Buildx setup in .github/workflows/build-and-push.yml for caching support
- [x] T007 Configure GHCR authentication using GITHUB_TOKEN in .github/workflows/build-and-push.yml
- [x] T008 Define image naming convention with repository prefix in .github/workflows/build-and-push.yml (ghcr.io/${{ github.repository }}/...)
- [x] T009 Add workflow trigger configuration for push to main branch in .github/workflows/build-and-push.yml

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Automated Build and Registry Push (Priority: P1) 🎯 MVP

**Goal**: When developers push code changes to the repository, the system automatically builds container images in the CI environment and pushes them to GitHub Container Registry

**Independent Test**: Push a commit to main branch → verify GitHub Actions workflow triggers → verify binance-provider and mcp-gateway images appear in GHCR with correct tags (sha-, branch, latest)

### Implementation for User Story 1

- [x] T010 [P] [US1] Create build job for binance-provider in .github/workflows/build-and-push.yml with context and dockerfile parameters
- [x] T011 [P] [US1] Create build job for mcp-gateway in .github/workflows/build-and-push.yml with context and dockerfile parameters
- [x] T012 [P] [US1] Configure image tagging strategy (commit SHA, branch name, latest) for binance-provider in .github/workflows/build-and-push.yml
- [x] T013 [P] [US1] Configure image tagging strategy (commit SHA, branch name, latest) for mcp-gateway in .github/workflows/build-and-push.yml
- [x] T014 [P] [US1] Enable BuildKit layer caching for binance-provider build in .github/workflows/build-and-push.yml (cache-from, cache-to)
- [x] T015 [P] [US1] Enable BuildKit layer caching for mcp-gateway build in .github/workflows/build-and-push.yml (cache-from, cache-to)
- [x] T016 [US1] Configure matrix strategy for parallel builds of both services in .github/workflows/build-and-push.yml
- [x] T017 [US1] Add build failure notification in workflow (GitHub Actions native status reporting)
- [x] T018 [US1] Test workflow by pushing to feature branch first, verify builds complete and images push to GHCR
- [x] T019 [US1] Merge workflow to main branch and verify production build

**Checkpoint**: At this point, pushing code to main automatically builds and pushes images to GHCR. Manual deployment is still required (US2).

---

## Phase 4: User Story 2 - Server Deployment via Image Pull (Priority: P2)

**Goal**: When new container images are available in the registry, the server can pull these pre-built images and deploy them using docker-compose, without needing to build anything locally

**Independent Test**: Manually run deployment script on server → verify images pull from GHCR → verify services restart with new images → verify health checks pass

### Implementation for User Story 2

- [x] T020 [US2] Create deployment script scripts/deploy/pull-and-restart.sh with shebang and set -euo pipefail
- [x] T021 [US2] Add docker-compose pull command to scripts/deploy/pull-and-restart.sh
- [x] T022 [US2] Add docker-compose up -d command to scripts/deploy/pull-and-restart.sh
- [x] T023 [US2] Add health check verification loop to scripts/deploy/pull-and-restart.sh (wait for services, check docker-compose ps)
- [x] T024 [US2] Add deployment logging with timestamps to scripts/deploy/pull-and-restart.sh
- [x] T025 [US2] Make deployment script executable (chmod +x scripts/deploy/pull-and-restart.sh)
- [x] T026 [P] [US2] Create rollback script scripts/deploy/rollback.sh with commit SHA parameter support
- [x] T027 [P] [US2] Add SHA tag switching logic to scripts/deploy/rollback.sh (export IMAGE_TAG variables)
- [x] T028 [P] [US2] Make rollback script executable (chmod +x scripts/deploy/rollback.sh)
- [x] T029 [US2] Update compose.yml to use ghcr.io image references for binance-provider (replace build: with image:)
- [x] T030 [US2] Update compose.yml to use ghcr.io image references for mcp-gateway (replace build: with image:)
- [x] T031 [US2] Add image tag environment variable support to compose.yml (${BINANCE_IMAGE_TAG:-latest}, ${GATEWAY_IMAGE_TAG:-latest})
- [x] T032 [P] [US2] Create compose.override.yml for local development with build: contexts
- [x] T033 [P] [US2] Update .env.example with GHCR credentials template (GHCR_USERNAME, GHCR_PAT, IMAGE_TAG variables)
- [x] T034 [US2] Document server authentication setup in quickstart.md (docker login to ghcr.io with PAT)
- [x] T035 [US2] Test deployment script locally or on staging server

**Checkpoint**: At this point, server can pull and deploy new images. Deployment is still manual (requires SSH + script execution).

---

## Phase 5: User Story 3 - Automated Deployment Trigger (Priority: P3)

**Goal**: When new images are successfully pushed to the registry, the server is automatically notified and triggers a deployment, completing the fully automated CI/CD pipeline

**Independent Test**: Push code to main → verify CI builds and pushes images → verify server automatically triggers deployment (if webhook/automation implemented) OR document manual trigger process

### Implementation for User Story 3

- [x] T036 [US3] Create optional GitHub Actions deployment workflow .github/workflows/deploy.yml with workflow_dispatch trigger for manual deploys
- [x] T037 [US3] Add SSH configuration to deployment workflow (if using SSH-based deployment) with server host, user, and deploy script path
- [x] T038 [US3] Configure GitHub repository secrets for deployment (DEPLOY_SSH_KEY if using automated SSH deployment) - optional, documented as manual alternative
- [x] T039 [US3] Add deployment success/failure reporting to GitHub Actions workflow status
- [x] T040 [US3] Document manual deployment process (SSH + script) in quickstart.md as primary method
- [x] T041 [US3] Document optional automated deployment setup in quickstart.md (if workflow_dispatch or webhook implemented)
- [x] T042 [US3] Add deployment history logging (create deployment.log with timestamps and commit SHAs) to scripts/deploy/pull-and-restart.sh
- [x] T043 [US3] Test complete pipeline end-to-end (push code → CI builds → manual or automated deployment → verify services)

**Checkpoint**: Complete CI/CD pipeline is operational. Deployments can be triggered manually (simple, reliable) or automatically (if configured).

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and production readiness

- [x] T044 [P] Add workflow status badge to README.md (GitHub Actions build status)
- [x] T045 [P] Document image cleanup strategy in quickstart.md (docker image prune commands for old tags)
- [x] T046 [P] Add troubleshooting section to quickstart.md for common CI/CD issues (permission errors, pull failures, build timeouts)
- [x] T047 [P] Verify all health checks work correctly in compose.yml for both services
- [x] T048 [P] Test rollback procedure using scripts/deploy/rollback.sh with previous commit SHA
- [x] T049 Validate quickstart.md setup instructions by following them step-by-step
- [x] T050 Create PR checklist template (.github/pull_request_template.md) with CI/CD verification steps
- [x] T051 Document GitHub Container Registry package visibility settings in quickstart.md (public vs private images)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion (T001-T004) - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion (T005-T009)
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3) - recommended for sequential development
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
  - Delivers: Automated builds and image publishing to GHCR
  - Independently testable: Push code, verify images in GHCR

- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Requires US1 images to exist for testing
  - Depends on US1 for testing (need images in GHCR to pull)
  - Can implement scripts and config independently
  - Delivers: Server deployment capability via pull + restart
  - Independently testable: Run deployment script, verify services use registry images

- **User Story 3 (P3)**: Can start after US2 is complete - Requires deployment scripts from US2
  - Depends on US2 (uses deployment scripts)
  - Delivers: Automated or documented manual deployment triggering
  - Independently testable: Trigger deployment, verify complete automation works

### Within Each User Story

**User Story 1**:
- T010-T011 (build jobs) can run in parallel
- T012-T013 (tagging) can run in parallel
- T014-T015 (caching) can run in parallel
- T016 (matrix strategy) integrates the parallel jobs
- T017-T019 (testing and validation) sequential after implementation

**User Story 2**:
- T020-T025 (deployment script) sequential (building one script)
- T026-T028 (rollback script) can run in parallel with deployment script
- T029-T031 (compose.yml updates) sequential (same file)
- T032-T033 (local dev overrides) can run in parallel
- T034-T035 (documentation and testing) after implementation complete

**User Story 3**:
- T036-T039 (deployment workflow) sequential
- T040-T041 (documentation) can run in parallel with workflow development
- T042-T043 (logging and testing) sequential after workflow complete

### Parallel Opportunities

- **Setup Phase**: T003 and T004 can run in parallel
- **Foundational Phase**: T006-T008 can run in parallel after T005
- **User Story 1**: T010-T011, T012-T013, T014-T015 can each run in parallel within their groups
- **User Story 2**: T026-T028 parallel with T020-T025, T032-T033 parallel
- **User Story 3**: T040-T041 parallel with T036-T039
- **Polish Phase**: T044-T048 all can run in parallel, T049-T051 sequential validation

---

## Parallel Example: User Story 1

```bash
# Launch build job configurations in parallel:
Task: "Create build job for binance-provider in .github/workflows/build-and-push.yml"
Task: "Create build job for mcp-gateway in .github/workflows/build-and-push.yml"

# Launch tagging configurations in parallel:
Task: "Configure image tagging for binance-provider"
Task: "Configure image tagging for mcp-gateway"

# Launch caching configurations in parallel:
Task: "Enable BuildKit caching for binance-provider"
Task: "Enable BuildKit caching for mcp-gateway"
```

## Parallel Example: User Story 2

```bash
# Launch script development in parallel:
Task: "Create deployment script scripts/deploy/pull-and-restart.sh"
Task: "Create rollback script scripts/deploy/rollback.sh"

# Launch configuration in parallel:
Task: "Create compose.override.yml for local development"
Task: "Update .env.example with GHCR credentials"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T009) - CRITICAL
3. Complete Phase 3: User Story 1 (T010-T019)
4. **STOP and VALIDATE**: Push code to main, verify images build and appear in GHCR
5. **MVP ACHIEVED**: Automated CI builds working, manual deployment still required

**Value at this checkpoint**: Developers no longer need to build locally. All builds are consistent and cached. Manual deployment with pre-built images is still faster than before.

### Incremental Delivery

1. **Foundation (T001-T009)** → CI infrastructure ready
2. **Add US1 (T010-T019)** → Automated builds working → **Deploy** (images available, manual deployment)
3. **Add US2 (T020-T035)** → Server automation ready → **Deploy** (automated pull + restart)
4. **Add US3 (T036-T043)** → Full automation optional → **Deploy** (documented manual or automated trigger)
5. **Polish (T044-T051)** → Production-ready → **Final deployment**

Each phase adds value without breaking previous functionality.

### Parallel Team Strategy

With multiple developers:

**Phase 1-2 (Foundation)**: Team completes together (T001-T009)

**Phase 3+ (User Stories)**: Once foundation is ready:
- **Developer A**: User Story 1 (T010-T019) - CI workflow development
- **Developer B**: User Story 2 (T020-T035) - Server scripts and config (can start setup work, needs US1 images for full testing)
- **Developer C**: Polish tasks (T044-T048) - Documentation, testing harness

**Phase 5 (US3)**: Requires US2 completion, typically one developer

**Recommendation for this project**: Sequential implementation (US1 → US2 → US3) is most practical since US2 needs US1 images for testing, and US3 builds on US2 scripts.

---

## Notes

- [P] tasks = different files or independent configurations, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each logical group of tasks (e.g., complete workflow file, complete deployment script)
- Stop at any checkpoint to validate story independently
- GitHub Actions workflow files use YAML - syntax errors will fail CI, test on feature branch first (T018)
- Server scripts should be tested locally or on staging before production deployment
- GHCR authentication is automatic in CI (GITHUB_TOKEN), manual setup required on server (PAT)
- Image tags strategy: use `sha-*` for rollbacks, `latest` for convenience, branch tags for environment-specific deployments

## Verification Checklist

After completing all tasks:

- [ ] GitHub Actions workflow builds both services successfully
- [ ] Images push to GHCR with correct tags (sha-, branch, latest)
- [ ] Build failures are visible in GitHub Actions UI
- [ ] Server can authenticate with GHCR using PAT
- [ ] Deployment script pulls latest images and restarts services
- [ ] Health checks pass after deployment
- [ ] Rollback script can revert to previous image tags
- [ ] Local development still works using compose.override.yml
- [ ] Documentation in quickstart.md is accurate and complete
- [ ] Complete pipeline tested end-to-end (code push → build → deploy → verify)
