# Pitfalls Research

**Domain:** Local-first desktop clipboard manager (brownfield Tauri/Rust/Svelte app)
**Researched:** 2026-04-11
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: Product claims drift away from the real search and retention contract

**What goes wrong:**
Teams keep shipping UI copy and README promises such as "full-history FTS search" or "configurable history limits" while the runtime still searches only a subset of recent text rows, ignores some retention settings, or loads fewer items than the user configured.

**Why it happens:**
Search and retention start as "good enough" MVP behaviors, then encryption, dedupe, and performance shortcuts accumulate faster than the product contract is rewritten. Search quality degrades silently because the app still appears to work for small histories.

**How to avoid:**
Write down the exact observable contract before touching implementation: searchable item types, searchable time horizon, how encrypted data participates, how retention limits prune pinned vs unpinned items, and what the UI should say when limits apply. Roadmap search and retention together, not as unrelated tweaks. Add regression checks for old-item recall, non-text behavior, and settings semantics before shipping new search work.

**Warning signs:**
- README or UI mentions FTS/full-history behavior that the backend cannot prove
- Search only feels correct on small or recent datasets
- Settings exist in the UI but are not enforced in one authoritative place
- Support feedback says "I know I copied this before, but search cannot find it"

**Phase to address:**
Phase 1: Contract Alignment and Baseline
Phase 2: Search and Retention Foundation

---

### Pitfall 2: Retrofitting better search without an encrypted indexing strategy

**What goes wrong:**
Teams add "better search" by scanning decrypted rows in memory, building ad hoc shadow columns, or partially indexing plaintext without deciding the privacy model. The result is a search path that is still slow, still incomplete, and now harder to reason about from a privacy standpoint.

**Why it happens:**
Search feels like a UI feature, but in a local-first clipboard app it is really a storage-model decision. Once encryption is optional and history can become large, search quality, latency, and secrecy trade against each other.

**How to avoid:**
Choose one explicit model and document it: plaintext FTS mirror, tokenized shadow fields, opt-in searchable index, or "encrypted items are not fully searchable." Benchmark p50/p95 latency on realistic datasets before claiming improvement. Treat recall and privacy as first-class acceptance criteria, not follow-up polish.

**Warning signs:**
- Search code decrypts rows on every keystroke
- Search scope is limited by arbitrary row caps instead of documented behavior
- No benchmark exists for 10k+ text items or image-heavy histories
- Team discussions about search focus on UI components instead of storage/index shape

**Phase to address:**
Phase 2: Search and Retention Foundation

---

### Pitfall 3: Live storage migration is shipped before runtime ownership is cleaned up

**What goes wrong:**
Changing the storage location at runtime appears to succeed, but the app keeps old database handles, stale crypto state, duplicate monitor threads, or inconsistent tray/event wiring. Users end up with split histories, partial copies, or data loss when the original directory is deleted too early.

**Why it happens:**
Migration work is framed as file copy plus config update, while a clipboard manager is actually a live process with long-lived monitor, storage, crypto, tray, and shortcut state. Teams underestimate teardown and reinitialization complexity because the app has no server and feels "simple."

**How to avoid:**
Make runtime migration a lifecycle feature, not a filesystem feature. Introduce explicit stop/restart control for clipboard monitoring, rebuild storage/crypto state after path changes, and stage migration as copy -> verify -> swap -> optional cleanup. If the app cannot guarantee safe hot migration yet, force a restart-based workflow and say so in the UX.

**Warning signs:**
- Migration code copies files but does not rebuild the live storage object
- Clipboard monitoring has no cancellation channel or join/teardown path
- Old data is deleted before post-copy verification passes
- Settings changes can alter data paths while background workers are still running

**Phase to address:**
Phase 3: Storage Migration and Runtime Lifecycle

---

### Pitfall 4: Permissions UX is treated as a boolean gate instead of a recoverable state machine

**What goes wrong:**
Clipboard or accessibility permission failures surface as "nothing happens," stale booleans, or log-only errors. Users cannot tell whether the app is broken, misconfigured, or blocked by the OS, and they do not get clear retry or recovery steps.

**Why it happens:**
Desktop permissions are often tested on already-approved developer machines. Teams wire a simple `hasPermission` check and stop there, missing intermediate states like denied, restricted, not-yet-requested, or requires restart.

**How to avoid:**
Model permission state explicitly in the backend and frontend. Return structured status plus platform-specific guidance. Design the UX around first-run onboarding, denial recovery, manual re-check, and degraded mode messaging. Test on clean macOS/Windows/Linux environments rather than only on dev boxes.

**Warning signs:**
- Permission APIs return strings or ad hoc booleans with no typed status model
- UI banners never appear in denial scenarios
- Backend logs mention denial while the frontend still shows "healthy"
- QA verifies permissions only on machines that already have access granted

**Phase to address:**
Phase 4: Permissions UX and Recoverable Errors

---

### Pitfall 5: "Encrypted local storage" is marketed without a truthful threat model

**What goes wrong:**
The roadmap says clipboard data is encrypted, but the actual key lives next to the database, follows the user-selected storage path, or lands in syncable folders. The feature creates confidence without meaningfully improving security against the most likely local attacker who can read that directory.

**Why it happens:**
Encryption at rest is easy to market and relatively easy to add, but key management is the hard part. Local-first desktop apps often stop at file encryption because OS keychain integration and migration semantics feel like later work.

**How to avoid:**
State the threat model in plain language before extending the feature. Separate "deters casual file inspection" from "protects against local account compromise." Move key material to OS credential storage where feasible, keep it outside user-chosen sync paths, and align README/settings copy with the real guarantees. Treat custom storage and migration flows as part of the crypto design.

**Warning signs:**
- Key file lives beside the encrypted database
- Marketing language implies strong protection without caveats
- Custom data directory and encryption were designed independently
- Security review focuses on cipher choice, not key lifecycle or storage topology

**Phase to address:**
Phase 5: Security Posture and Claim Alignment

---

### Pitfall 6: Compression is added directly into the clipboard capture hot path

**What goes wrong:**
Teams bolt on AVIF/WebP/MozJPEG or resizing during clipboard capture, causing slower capture, UI stalls, extra conversions, and surprising fidelity loss. Large images may be recompressed multiple times before storage or preview generation is complete.

**Why it happens:**
Compression looks like a storage-size feature, but in a clipboard manager it sits on the highest-frequency path in the whole product. Developers optimize for disk savings first and discover too late that they also changed capture latency, CPU spikes, battery impact, and restore fidelity.

**How to avoid:**
Separate the pipeline into capture, canonical storage, derived preview, and optional background compression. Decide when original bytes must be preserved, when previews are sufficient, and how users can opt into lossy behavior. Add metrics around large-image capture latency, memory use, and restore fidelity before exposing new compression settings.

**Warning signs:**
- Image code does decode -> resize -> encode multiple times on the main capture path
- "Store original" still resizes or recompresses under some conditions
- Compression settings do not explain quality vs fidelity tradeoffs
- No tests compare restored output against the original clipboard payload

**Phase to address:**
Phase 6: Media Fidelity and Compression

---

### Pitfall 7: Rich clipboard formats are silently degraded while roadmap work assumes fidelity

**What goes wrong:**
HTML, RTF, and file-list clipboard content are stored as if they are first-class, but restore paths flatten them to plain text or path strings. Teams then build search, migration, or compression improvements on top of a data model that already lost meaning.

**Why it happens:**
Text and images cover most demos, so richer formats remain half-supported for a long time. Once type enums exist, contributors assume fidelity already exists even if restore semantics do not match the type labels.

**How to avoid:**
Declare which clipboard formats are fully faithful, preview-only, or intentionally degraded. Fix payload fidelity before claiming richer media/search improvements. Make format-specific verification part of phase success criteria, especially before touching compression or migration.

**Warning signs:**
- Storage types distinguish HTML/RTF/File but restore logic normalizes them to text
- UI labels imply support for content types that round-trip incorrectly
- Compression or search plans talk about "all clipboard content" without format carve-outs
- Bug reports mention lost formatting or broken pasted file behavior

**Phase to address:**
Phase 6: Media Fidelity and Compression
Phase 7: Verification and Release Hardening

---

### Pitfall 8: Teams add stateful features without a reproducible verification baseline

**What goes wrong:**
Search, migration, permission recovery, and compression work all land through manual spot checks on the maintainer's machine. Regressions only appear after release because clean-checkout verification, platform setup, and end-to-end stateful flows are not automated.

**Why it happens:**
Desktop clipboard apps are OS-sensitive, so teams postpone automation and rely on "I tested it locally." Brownfield apps especially accumulate assumptions about prebuilt assets, granted permissions, and preexisting data directories.

**How to avoid:**
Create a clean-checkout baseline before or alongside risky feature work. Add automated coverage for storage/search logic, permission-state rendering, and migration/compression decision code. Document the exact bootstrap steps and ensure CI can reproduce them.

**Warning signs:**
- `cargo test`, lint, or type-check require undocumented setup side effects
- No regression tests exist for search recall, migration swap, or permission banners
- Release confidence depends on a single machine with existing permissions/data
- Roadmap phases mention verification late or not at all

**Phase to address:**
Phase 1: Contract Alignment and Baseline
Phase 7: Verification and Release Hardening

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Keep search as an in-memory substring scan over recent decrypted rows | Fastest way to ship "search" | False negatives, slow queries, privacy ambiguity, docs drift | Only as a temporary bridge if the UI and README explicitly describe the limit |
| Treat data-directory migration as copy-and-point-settings | Minimal code change | Stale handles, split state, data loss risk | Never for hot migration; only for offline/manual migration with app restart |
| Store the encryption key beside the database | Simplifies local file management | Security claims become misleading | Only as an explicitly weak legacy mode with clear warnings |
| Add compression directly during capture | Immediate disk savings | Capture latency, fidelity regressions, battery/CPU spikes | Only for derived previews, never as the only stored representation by default |
| Keep desktop permission handling as boolean state | Simple frontend logic | Dead UI states and poor recovery UX | Never once the app ships to end users |
| Let settings semantics live in multiple layers | Faster iteration | Search/retention behavior becomes impossible to reason about | Only during short-lived prototyping before a contract phase |

## Integration Gotchas

Common mistakes when connecting to external services and OS features.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| macOS accessibility / clipboard permissions | Assume granted permissions on dev machines represent production reality | Model denied/requestable/restricted states and test on clean machines |
| SQLite migration and live handles | Copy database files while the process still owns live connections | Quiesce workers, verify copy, reopen storage, then switch active state |
| OS keychain / credential store | Add encryption first and defer keychain integration indefinitely | Decide key storage architecture before expanding security claims |
| Tauri IPC boundary | Return stringly typed status blobs that the UI interprets inconsistently | Use typed command responses for permission, migration, and storage outcomes |
| Tray and monitor integration | Rebuild tray and state synchronously on every clipboard event | Debounce refreshes and keep lifecycle boundaries explicit |
| Updater / remote content display | Trust signed remote markdown enough to inject raw HTML | Treat remote release notes as untrusted content and sanitize/render safely |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Decrypt-and-scan search on every query | Search slows, older items disappear, CPU spikes on typing | Add indexed search strategy with benchmarks and documented scope | Once histories move beyond a few thousand text items |
| Sending full clipboard payloads to the frontend by default | Large renderer heap, slow list rendering, memory churn | Split metadata/previews from full payload retrieval | Image-heavy histories or large pasted blobs |
| Full tray rebuild on every clipboard change | Hot path feels heavier than capture itself | Debounce tray refresh and cache menu models | Fast copy bursts and media-heavy usage |
| Multi-step image transcoding in capture path | Capture lag, extra CPU, larger temporary memory spikes | Preserve canonical bytes early and derive previews/compression off-path | Large screenshots or frequent image copies |
| Retention cleanup only on insert | Visible item count drifts from configured limit | Enforce retention during load, insert, and settings changes | As soon as users lower limits or expect immediate cleanup |

## Security Mistakes

Domain-specific security issues beyond general desktop hardening.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Co-locating the key with the encrypted database | Anyone who can read the folder can usually read both | Use OS credential storage or a clearly separated key location |
| Overselling "encrypted local storage" in docs and UI | Users make trust decisions on a false premise | Document the real threat model and sync/storage caveats |
| Leaving broad desktop capabilities enabled by default | Extra attack surface for a simple local app | Minimize Tauri capabilities/plugins to what current flows need |
| Keeping verbose debug logging in release flows | Clipboard-adjacent privacy leaks grow over time | Make release logging conservative and redact sensitive paths/content |
| Sanitization gaps for remotely sourced UI content | Untrusted update metadata can become an injection surface | Sanitize remote markdown and disable raw HTML rendering |

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Search box implies complete recall when scope is partial | Users stop trusting history and fall back to source apps | Show accurate search scope until full recall exists |
| Permissions failure looks like a broken app | Users churn before finding system settings | Provide platform-specific guidance, retry actions, and degraded-state messaging |
| Storage migration appears instant and safe | Users delete old data too early and lose history | Use staged migration UX with verify/swap/cleanup checkpoints |
| Compression settings hide lossy tradeoffs | Users lose image fidelity without understanding why | Present clear presets with examples and default-safe behavior |
| Rich clipboard formats are silently flattened | Pasted output loses formatting/files unexpectedly | Label fidelity level per content type and verify round-trips |
| Settings exist but do not take effect immediately | Users think the app ignores them | Make each setting's timing and scope explicit in the UI |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Search:** Often missing full-history recall and encrypted-item policy — verify searchable scope, benchmarks, and docs all match
- [ ] **Retention:** Often missing immediate enforcement after settings changes — verify load-time and insert-time behavior both respect limits
- [ ] **Migration:** Often missing live-state teardown/reopen — verify monitor, DB, crypto, and tray all bind to the new path after swap
- [ ] **Permissions:** Often missing denial recovery UX — verify first-run, denied, revoked, and retry flows on clean machines
- [ ] **Encryption:** Often missing truthful threat-model copy — verify key location, custom storage implications, and user-facing claims
- [ ] **Compression:** Often missing restore-fidelity checks — verify canonical/original behavior, preview generation, and large-image latency
- [ ] **Rich formats:** Often missing round-trip tests — verify HTML, RTF, and file payloads paste back with intended fidelity
- [ ] **Verification:** Often missing clean-checkout automation — verify a new machine can run checks without hidden setup knowledge

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Search contract drift | MEDIUM | Freeze marketing copy, document current scope, add missing regression tests, then rebuild the indexed search path |
| Unsafe live migration | HIGH | Stop further migrations, ship restart-required fallback, recover from verified backup/original directory, then refactor lifecycle ownership |
| Broken permission UX | LOW to MEDIUM | Add typed permission status, patch frontend rendering, test denial/retry flows on clean OS installs, and release a UX fix quickly |
| Misleading encryption/storage claims | MEDIUM | Update docs/settings copy immediately, add warnings for custom storage, then migrate key handling to OS credential storage |
| Compression-driven fidelity regressions | MEDIUM to HIGH | Disable lossy defaults, restore canonical/original storage, add feature flags for compression, and compare restored output against fixtures |
| Rich format degradation | MEDIUM | Narrow supported-format claims, add explicit degradation labels, then implement format-specific restore logic and tests |
| Missing verification baseline | MEDIUM | Document bootstrap, fix clean-checkout failures, add CI targets, and block risky roadmap phases on reproducible checks |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Product claims drift from search/retention reality | Phase 1: Contract Alignment and Baseline | README, UI copy, settings semantics, and behavior all match on a test dataset |
| No encrypted indexing strategy for search | Phase 2: Search and Retention Foundation | Benchmarked search recall/latency passes for realistic history sizes |
| Unsafe live storage migration | Phase 3: Storage Migration and Runtime Lifecycle | End-to-end migration test proves the app rebinds to the new path without stale workers |
| Permission UX treated as a boolean | Phase 4: Permissions UX and Recoverable Errors | Clean-machine permission denial and recovery flows are manually and automatically checked |
| Encryption claims exceed real protection | Phase 5: Security Posture and Claim Alignment | Threat-model copy, key storage behavior, and custom-path rules are reviewed together |
| Compression added in the hot path | Phase 6: Media Fidelity and Compression | Large-image latency, storage size, and restore fidelity meet explicit acceptance criteria |
| Rich clipboard formats silently degrade | Phase 6: Media Fidelity and Compression | HTML/RTF/file round-trips are covered by fixtures or platform-specific checks |
| Verification baseline missing | Phase 7: Verification and Release Hardening | Clean checkout can run documented checks in CI and on a contributor machine |

## Sources

- `.planning/PROJECT.md`
- `.planning/codebase/CONCERNS.md`
- `.planning/codebase/ARCHITECTURE.md`
- `README.md`
- Static inspection themes already surfaced in the brownfield codebase map for search, migration, permission UX, encryption, and media handling

---
*Pitfalls research for: local-first desktop clipboard manager evolution*
*Researched: 2026-04-11*
