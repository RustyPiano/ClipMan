# Requirements: ClipMan

**Defined:** 2026-04-11
**Core Value:** Clipboard history must stay fast, trustworthy, and instantly retrievable without turning a simple desktop utility into a fragile, heavyweight system.

## v1 Requirements

Requirements for the next brownfield milestone. These focus on reliability, product-contract alignment, and safe evolution of existing capabilities.

### Search & Retrieval

- [ ] **SRCH-01**: User can search all retained text history that the product claims is searchable, not just an undocumented recent subset.
- [ ] **SRCH-02**: User can find retained clipboard items consistently across app restarts and reloads when those items are still within the configured retention policy.
- [ ] **SRCH-03**: User can change the history limit and see loading, pruning, and search scope follow that setting consistently.

### Storage & Migration

- [ ] **STOR-01**: User can change the data storage directory without losing history, pinned items, or settings.
- [ ] **STOR-02**: User can continue using the app from the new data directory after migration without stale state, duplicate monitors, or silent fallback to the old location.
- [ ] **STOR-03**: User can understand the real protection level of local encrypted storage from the app's documentation and settings copy.

### Permissions & Recovery

- [ ] **PERM-01**: User can see whether clipboard or accessibility permission is granted, missing, denied, or needs re-checking.
- [ ] **PERM-02**: User receives platform-specific recovery steps when required permissions block clipboard capture or restore behavior.
- [ ] **PERM-03**: User sees actionable error feedback when migration, settings updates, or clipboard operations fail.

### Fidelity & Media

- [ ] **MED-01**: User can restore plain-text clipboard entries exactly as copied.
- [ ] **MED-02**: User can restore HTML, RTF, file, and image entries with a documented fidelity level for each content type.
- [ ] **MED-03**: User can choose image storage or compression behavior through settings that clearly explain fidelity versus size tradeoffs.

### Verification & Release Safety

- [ ] **QUAL-01**: Maintainer can run the documented verification commands from a clean checkout without hidden setup steps.
- [ ] **QUAL-02**: Core clipboard capture, search, retention, migration, permission, and restore flows are covered by automated checks before release.
- [ ] **QUAL-03**: Release packaging runs the project's required verification gates before shipping binaries.

## v2 Requirements

Deferred to future release. Tracked but not part of the current roadmap.

### Organization

- **ORG-01**: User can group clipboard items into named collections.
- **ORG-02**: User can define filter rules or app exclusions to reduce unwanted captures.

### Expansion

- **EXP-01**: User can sync selected clipboard history across devices.
- **EXP-02**: User can extend ClipMan through a supported plugin system.
- **EXP-03**: User can interact with ClipMan through a supported CLI.

## Out of Scope

Explicitly excluded from the current milestone.

| Feature | Reason |
|---------|--------|
| Always-on cloud account system | Changes the product from local-first utility to backend-dependent service |
| Hidden lossy compression defaults | Undermines trust in copied media fidelity |
| Plugin or CLI delivery in the current milestone | Core desktop contract is not stable enough yet |
| Unbounded history growth with implicit resource use | Conflicts with the product's lightweight performance promise |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SRCH-01 | TBD | Pending |
| SRCH-02 | TBD | Pending |
| SRCH-03 | TBD | Pending |
| STOR-01 | TBD | Pending |
| STOR-02 | TBD | Pending |
| STOR-03 | TBD | Pending |
| PERM-01 | TBD | Pending |
| PERM-02 | TBD | Pending |
| PERM-03 | TBD | Pending |
| MED-01 | TBD | Pending |
| MED-02 | TBD | Pending |
| MED-03 | TBD | Pending |
| QUAL-01 | TBD | Pending |
| QUAL-02 | TBD | Pending |
| QUAL-03 | TBD | Pending |

**Coverage:**
- v1 requirements: 15 total
- Mapped to phases: 0
- Unmapped: 15 ⚠️

---
*Requirements defined: 2026-04-11*
*Last updated: 2026-04-11 after initialization*
