# Project Research Summary

**Project:** ClipMan
**Domain:** Local-first desktop clipboard manager
**Researched:** 2026-04-11
**Confidence:** MEDIUM

## Executive Summary

ClipMan is already on the right macro-architecture for its product class: a local-first Tauri desktop shell, Rust runtime services, Svelte UI, and SQLite-backed persistence remain the standard, low-overhead way to build a clipboard manager that feels fast and private. The research does not suggest a platform rewrite. It suggests making the existing contract honest and durable.

The biggest ecosystem lesson is that clipboard tools win or lose on trust, not on feature count. Users expect automatic capture, fast recall, keyboard-first access, retention controls, and clear privacy behavior as table stakes. What differentiates a mature tool is not merely claiming search, encryption, or media support, but making those capabilities behave consistently under real retained histories, richer content types, and OS-permission edge cases.

For ClipMan specifically, the most important path is to stabilize the current brownfield product before expanding into sync, plugins, or CLI surfaces. Search scope, retention semantics, live storage migration, permission recovery, format fidelity, and clean verification are the risks most likely to undermine user trust. Image compression is worth pursuing only after the fidelity and lifecycle contract is explicit.

## Key Findings

### Recommended Stack

The current Tauri 2 + Rust + Svelte 5 + SQLite stack remains the recommended baseline for this product. Incremental improvement is better than replacement.

**Core technologies:**
- **Tauri 2.x**: native shell, updater, tray, global shortcut, autostart, and permissions surface fit a desktop clipboard utility well.
- **Rust 2021/stable**: best place for capture lifecycle, storage, migration, crypto, and media work that must remain predictable.
- **Svelte 5.x**: good fit for a small fast desktop UI where local stores and minimal runtime cost matter.
- **SQLite with FTS5**: the right local search primitive once ClipMan stops relying on decrypt-and-scan behavior.

### Expected Features

Research across current clipboard-manager patterns points to a narrow set of must-haves for the next milestone, plus a smaller set of differentiators worth adding only after the baseline is trustworthy.

**Must have (table stakes):**
- Automatic clipboard capture that remains reliable across restarts
- Fast and truthful history search/filtering
- Shortcut and tray-based access
- Pinning/favorites and predictable retention controls
- Clear permission and privacy behavior
- Reliable text and image restore behavior

**Should have (competitive):**
- Indexed search with documented full-history or explicitly scoped recall
- Explicit fidelity handling for HTML, RTF, files, and images
- Configurable image storage/compression after fidelity baselines exist
- Capture exclusions or filter rules once the baseline contract is stable

**Defer (v2+):**
- Multi-device sync
- Plugin system
- CLI surface

### Architecture Approach

The research points to one dominant architecture direction: keep canonical clipboard storage separate from search projections and derived media outputs, and treat runtime lifecycle as a first-class concern. Search, migration, permissions, and media handling should be modeled as explicit services with typed IPC boundaries rather than spread across ad hoc command logic.

**Major components:**
1. **Capture service** — owns live clipboard monitoring and normalization
2. **Storage plus search projection** — stores canonical content and indexed/queryable search data with explicit searchable-scope policy
3. **Lifecycle and policy services** — own migration, permission states, retention semantics, and media behavior

### Critical Pitfalls

1. **Search and retention claims drift from runtime reality** — fix the user-visible contract before marketing more search capability
2. **Search improvements ship without a clear encrypted indexing strategy** — search is a storage-model decision, not just a UI improvement
3. **Live storage migration lands before runtime ownership is explicit** — pause, verify, reopen, and resume must be part of the design
4. **Permissions UX is modeled as a boolean instead of a state machine** — users need denial and recovery flows, not silent failure
5. **Compression is added on the hot capture path** — separate canonical storage from previews and optional lossy derivatives

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Contract Alignment and Baseline
**Rationale:** Trust problems are already present in current behavior and documentation. Fix the product contract before expanding capability claims.
**Delivers:** Search/retention contract definition, typed status model boundaries, clean verification baseline
**Addresses:** Search truthfulness, settings semantics, release reproducibility
**Avoids:** Product-claim drift and hidden setup assumptions

### Phase 2: Search and Retention Foundation
**Rationale:** Search is the core retrieval promise and the highest-value differentiator once trustworthy.
**Delivers:** Indexed or explicitly scoped search model tied to retention semantics
**Uses:** SQLite FTS5 or another explicit local projection model
**Implements:** Search service and search-policy ownership

### Phase 3: Storage Migration and Runtime Lifecycle
**Rationale:** Data-path changes are too risky until the app owns monitor, storage, and crypto lifecycle explicitly.
**Delivers:** Safe migration flow with pause, verify, reopen, and resume behavior
**Uses:** Existing Rust runtime plus explicit lifecycle control
**Implements:** Migration coordinator and runtime ownership model

### Phase 4: Permission UX and Recoverable Errors
**Rationale:** Once lifecycle and search behavior are clearer, the app can expose accurate state and recovery guidance.
**Delivers:** Typed permission states and actionable failure UX
**Implements:** Typed IPC results and frontend status handling

### Phase 5: Content Fidelity Matrix
**Rationale:** Before adding compression or broader media features, ClipMan must define what each content type can round-trip reliably.
**Delivers:** Format-specific fidelity policy for text, HTML, RTF, files, and images
**Avoids:** Silent rich-content degradation

### Phase 6: Media Compression and Storage Controls
**Rationale:** Compression belongs after fidelity rules and performance baselines exist.
**Delivers:** Safe image compression/storage settings and background media pipeline decisions
**Uses:** Existing image stack plus explicit preview/canonical separation

### Phase 7: Security and Release Hardening
**Rationale:** The app should align encryption claims, capability surface, CI verification, and release gating before broader expansion.
**Delivers:** Honest security posture, reduced attack surface, and release verification gates
**Avoids:** Oversold encryption and unverified shipping builds

### Phase Ordering Rationale

- Search comes before organization or sync because retrieval trust is the core product promise.
- Migration comes before more settings power because runtime lifecycle must be safe before more reconfiguration options land.
- Fidelity comes before compression because optimization without a format contract creates silent regressions.
- Verification and security hardening remain visible throughout, but the strongest release gate work lands after the most stateful changes are implemented.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2:** search model and encrypted-index tradeoffs need explicit design choices
- **Phase 6:** image compression format choices and performance tradeoffs need implementation-level evaluation
- **Phase 7:** key-storage strategy and capability minimization need security-focused validation

Phases with standard patterns (skip heavy research-phase unless scope changes):
- **Phase 1:** contract alignment and baseline verification are mostly local-repo work
- **Phase 4:** permission-state modeling and recoverable error UX follow standard desktop patterns
- **Phase 5:** fidelity matrix work is mostly current-code truth mapping rather than ecosystem novelty

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | Official docs support the macro-stack, but some future library choices should be finalized during phase planning |
| Features | MEDIUM | Competitor and product-site evidence is strong enough for expectations, but differentiation scope still depends on ClipMan priorities |
| Architecture | MEDIUM | The architecture direction is well-supported by the current codebase and standard local-first patterns |
| Pitfalls | HIGH | The strongest findings map directly onto observed ClipMan brownfield issues plus common desktop-tool failure modes |

**Overall confidence:** MEDIUM

### Gaps to Address

- Exact search-index strategy for optionally encrypted items still requires a concrete design decision.
- Exact image compression codecs and quality presets should be chosen during phase planning, not assumed here.
- If sync or plugin work is reprioritized later, PROJECT.md and REQUIREMENTS.md should be revisited because that would change the product boundary materially.

## Sources

### Primary (HIGH confidence)
- [Tauri Plugins](https://v2.tauri.app/plugin/) — official desktop integration surface
- [SQLite FTS5](https://sqlite.org/fts5.html) — official local full-text indexing behavior
- [Svelte](https://svelte.dev/) — official framework reference
- [Image crate docs](https://docs.rs/image/latest/image/) — current Rust media-processing reference

### Secondary (MEDIUM confidence)
- [CopyQ](https://copyq.net/) — mature clipboard-manager product reference
- [CopyQ documentation](https://copyq.readthedocs.io/en/latest/) — workflow and data-type breadth reference
- [Maccy](https://maccy.app/) — lightweight clipboard-manager expectation reference

### Project-Specific
- [.planning/PROJECT.md](../PROJECT.md)
- [.planning/codebase/STACK.md](../codebase/STACK.md)
- [.planning/codebase/ARCHITECTURE.md](../codebase/ARCHITECTURE.md)
- [.planning/codebase/CONCERNS.md](../codebase/CONCERNS.md)
- [.planning/research/STACK.md](./STACK.md)
- [.planning/research/FEATURES.md](./FEATURES.md)
- [.planning/research/ARCHITECTURE.md](./ARCHITECTURE.md)
- [.planning/research/PITFALLS.md](./PITFALLS.md)

---
*Research completed: 2026-04-11*
*Ready for roadmap: yes*
