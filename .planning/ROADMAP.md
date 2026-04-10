# Roadmap: ClipMan

## Overview

ClipMan already ships the core desktop clipboard workflow, so this brownfield milestone focuses on making the shipped contract reliable and honest before adding broader product surfaces. The roadmap starts by fixing search and retention truth, then hardens storage migration and permission recovery, makes clipboard fidelity explicit across content types, and finishes by making verification and release gates reproducible from a clean checkout.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions if needed later

- [ ] **Phase 1: Search Contract & Retention Truth** - Make search scope, persistence, and history-limit behavior match the real product contract.
- [ ] **Phase 2: Safe Storage Migration & Encryption Messaging** - Make live data-directory changes safe and clarify what local protection actually means.
- [ ] **Phase 3: Permissions & Recoverable Failure UX** - Surface runtime permission state and backend failures with actionable recovery paths.
- [ ] **Phase 4: Clipboard Fidelity & Image Storage Controls** - Define truthful restore behavior for each content type and add explicit media tradeoff controls.
- [ ] **Phase 5: Verification & Release Safety** - Make critical flows testable, reproducible, and gated before shipping binaries.

## Phase Details

### Phase 1: Search Contract & Retention Truth
**Goal**: Make ClipMan's search and retention behavior truthful, predictable, and aligned with the UI and README across restarts and settings changes.
**Depends on**: Nothing (first phase)
**Requirements**: [SRCH-01, SRCH-02, SRCH-03]
**Success Criteria** (what must be TRUE):
  1. User can search all retained text history that the app says is searchable, without undocumented recent-history limits.
  2. User can restart or reload the app and still find retained items as long as they remain inside the configured retention policy.
  3. User can change the history limit and see loading, pruning, and search scope follow that setting consistently.
  4. User-facing settings copy and README language describe the actual searchable scope and retention behavior truthfully.
**Plans**: 3 plans

Plans:
- [ ] 01-01: Audit current search and retention behavior and define the truthful product contract
- [ ] 01-02: Implement retention-aware search scope and history-limit consistency in backend and UI
- [ ] 01-03: Align settings copy, documentation, and migration assumptions with the finalized search contract

### Phase 2: Safe Storage Migration & Encryption Messaging
**Goal**: Make data-directory migration safe at runtime while clarifying the real protection boundary of ClipMan's local encrypted storage.
**Depends on**: Phase 1
**Requirements**: [STOR-01, STOR-02, STOR-03]
**Success Criteria** (what must be TRUE):
  1. User can change the storage directory without losing history, pinned items, or settings.
  2. User can continue using the app from the new directory without stale database handles, duplicate monitors, or silent fallback to the old path.
  3. User can see accurate in-app copy that explains what local encryption protects and what it does not protect.
  4. User receives a clear failure or rollback path if migration cannot complete safely.
**Plans**: 3 plans

Plans:
- [ ] 02-01: Define runtime ownership of database, monitor, and settings state during storage migration
- [ ] 02-02: Implement pause-migrate-verify-reopen flow for safe directory changes
- [ ] 02-03: Align encryption and storage messaging in settings, documentation, and migration UX

### Phase 3: Permissions & Recoverable Failure UX
**Goal**: Turn permission checks and backend failures into explicit desktop workflows with actionable status and recovery guidance.
**Depends on**: Phase 2
**Requirements**: [PERM-01, PERM-02, PERM-03]
**Success Criteria** (what must be TRUE):
  1. User can see whether clipboard or accessibility permission is granted, missing, denied, or needs re-checking.
  2. User receives platform-specific recovery guidance when permissions block capture or restore behavior.
  3. User sees actionable error feedback when migration, settings updates, or clipboard operations fail.
  4. User can retry failed operations after fixing the underlying problem without restarting into an unknown state.
**Plans**: 3 plans

Plans:
- [ ] 03-01: Define permission and command failure state models across Rust services and Tauri IPC
- [ ] 03-02: Implement frontend status surfaces, recovery guidance, and retry flows
- [ ] 03-03: Cover migration, settings, and clipboard failures with explicit diagnostics and regression checks

### Phase 4: Clipboard Fidelity & Image Storage Controls
**Goal**: Make clipboard fidelity truthful for text, HTML, RTF, files, and images, then expose explicit image storage and compression tradeoffs.
**Depends on**: Phase 3
**Requirements**: [MED-01, MED-02, MED-03]
**Success Criteria** (what must be TRUE):
  1. User can restore plain-text clipboard entries exactly as copied.
  2. User can restore or inspect HTML, RTF, file, and image entries with a documented fidelity level for each content type.
  3. User can choose image storage or compression behavior through settings that clearly explain fidelity-versus-size tradeoffs.
  4. Rich clipboard content that cannot round-trip perfectly is either preserved correctly or clearly marked as degraded by design.
**Plans**: 3 plans

Plans:
- [ ] 04-01: Define the current and target fidelity matrix for every supported clipboard content type
- [ ] 04-02: Implement truthful capture and restore handling plus fidelity metadata for rich content
- [ ] 04-03: Add explicit image storage and compression controls without hiding lossy behavior

### Phase 5: Verification & Release Safety
**Goal**: Make the milestone releasable from a clean checkout by codifying verification commands, automated coverage, and packaging gates.
**Depends on**: Phase 4
**Requirements**: [QUAL-01, QUAL-02, QUAL-03]
**Success Criteria** (what must be TRUE):
  1. Maintainer can run the documented verification commands from a clean checkout without hidden setup steps.
  2. Core clipboard capture, search, retention, migration, permission, and restore flows are covered by automated checks before release.
  3. Release packaging refuses to ship binaries until required verification gates pass.
  4. Release and contributor documentation match the actual verification path used to validate the app.
**Plans**: 3 plans

Plans:
- [ ] 05-01: Define clean-checkout verification and contributor setup contracts
- [ ] 05-02: Add automated coverage for the critical desktop flows this milestone stabilizes
- [ ] 05-03: Enforce verification gates in packaging and release workflows

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Search Contract & Retention Truth | 0/3 | Not started | - |
| 2. Safe Storage Migration & Encryption Messaging | 0/3 | Not started | - |
| 3. Permissions & Recoverable Failure UX | 0/3 | Not started | - |
| 4. Clipboard Fidelity & Image Storage Controls | 0/3 | Not started | - |
| 5. Verification & Release Safety | 0/3 | Not started | - |
