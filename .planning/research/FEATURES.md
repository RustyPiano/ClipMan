# Feature Research

**Domain:** Local-first desktop clipboard manager
**Researched:** 2026-04-11
**Confidence:** MEDIUM

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these makes the app feel broken rather than differentiated.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Automatic clipboard history capture | This is the product's baseline promise | LOW | Already shipped in ClipMan and must stay reliable before new surface area is added. |
| Fast history search and filtering | Users treat clipboard managers as recall tools, not passive logs | MEDIUM | Search scope and retention semantics must match user expectations. |
| Keyboard-first access and tray or menu presence | Clipboard tools are expected to be reachable without hunting through windows | LOW | Already shipped through shortcut and tray integrations. |
| Pin or favorite important entries | Users expect a way to keep frequent snippets available | LOW | Already shipped and should remain separate from retention cleanup. |
| Retention and history-size controls | Users expect local tools to stay manageable and predictable | MEDIUM | Settings must apply immediately and consistently. |
| Text and image support with predictable restore behavior | Plain text only feels incomplete in modern clipboard tools | MEDIUM | Format fidelity matters more than the number of nominally supported types. |
| Clear privacy and permission behavior | Clipboard tools touch sensitive data and OS permissions | MEDIUM | On desktop, trust is part of the feature set. |

### Differentiators (Competitive Advantage)

Features that strengthen ClipMan once the core contract is trustworthy.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Accurate full-history or clearly scoped indexed search | Turns search from "maybe useful" into a dependable retrieval workflow | HIGH | This is the most important brownfield differentiator because current product claims drift here. |
| Content-fidelity-aware restore for HTML, RTF, files, and images | Makes the app reliable for richer workflows than plain-text snippets | HIGH | Must be explicit about what is fully faithful versus intentionally degraded. |
| Configurable image storage and compression | Keeps the app lightweight without punishing screenshot-heavy users | HIGH | Only worth shipping if latency and fidelity tradeoffs are understandable. |
| App exclusions, filtering rules, or safe capture policies | Improves privacy and reduces noise in always-on clipboard capture | MEDIUM | Strong fit for local-first desktop tooling after current contract work lands. |
| Grouping, tags, or smarter organization | Helps heavy users manage larger histories | MEDIUM | Useful after search and retention behavior are already dependable. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Built-in cloud sync as a default assumption | Users like cross-device convenience | It changes the privacy model, requires identity and sync conflict handling, and drags a local utility into backend complexity | Keep local-first as default and treat sync as a future separate product decision |
| Unlimited history with no visible tradeoffs | Sounds powerful | Increases search ambiguity, storage churn, and UI/performance burden | Provide clear retention presets and pinned-item semantics |
| Lossy compression hidden behind "smart" defaults | Sounds efficient | Users lose trust when screenshots or copied media degrade unexpectedly | Make compression explicit, optional, and previewable |
| Plugin or scripting surface before core lifecycle is stable | Appeals to power users | Multiplies unsupported edges while the storage and permission contract is still drifting | Stabilize core APIs and lifecycle guarantees first |

## Feature Dependencies

```text
Search correctness
    └──requires──> Retention semantics
                         └──requires──> Single source of truth for settings

Runtime storage migration
    └──requires──> Explicit lifecycle ownership for monitor, DB, and crypto

Rich-content fidelity
    └──requires──> Canonical content model per clipboard type
                         └──requires──> Verification baseline

Image compression
    └──enhances──> Image support
                         └──requires──> Fidelity contract

Sync / plugins / CLI
    └──conflicts-with current scope until──> Core reliability and trust gaps are closed
```

### Dependency Notes

- **Search correctness requires retention semantics:** users cannot trust search if the app silently drops searchable scope or ignores configured limits.
- **Runtime storage migration requires lifecycle ownership:** file-copy logic alone is not enough in a live clipboard process.
- **Rich-content fidelity requires a canonical content model:** type labels mean little if restore logic still collapses formats to text.
- **Image compression enhances image support:** it is not a standalone feature; it depends on a clear fidelity contract.
- **Sync, plugins, and CLI conflict with current scope:** each adds surface area before the current desktop contract is fully trustworthy.

## MVP Definition

### Launch With (v1)

Minimum next milestone for this brownfield initialization.

- [ ] Search and retention behavior match the product contract — users can trust recall and limits
- [ ] Storage migration is safe and recoverable — users do not risk split state or silent data loss
- [ ] Permission and error states are actionable — users can recover from denial or failure
- [ ] Clipboard content fidelity is truthful per format — users know what round-trips correctly
- [ ] Verification baseline exists — maintainers can reproduce clean checks before release

### Add After Validation (v1.x)

- [ ] Configurable image compression presets — after fidelity and performance baselines are measured
- [ ] App exclusions or filter rules — after current capture semantics are stable
- [ ] Better organization primitives such as groups or tags — after search behavior is trustworthy

### Future Consideration (v2+)

- [ ] Multi-device sync — defer until the product is ready for identity, sync conflict handling, and a new privacy model
- [ ] Plugin system — defer until core APIs and lifecycle guarantees are intentionally exposed
- [ ] CLI tool — defer until the desktop contract is stable enough to support automation cleanly

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Search and retention contract alignment | HIGH | HIGH | P1 |
| Safe storage migration | HIGH | HIGH | P1 |
| Permission and recoverable error UX | HIGH | MEDIUM | P1 |
| Rich-content fidelity | HIGH | HIGH | P1 |
| Verification and release baseline | HIGH | MEDIUM | P1 |
| Configurable image compression | MEDIUM | HIGH | P2 |
| Filter rules / exclusions | MEDIUM | MEDIUM | P2 |
| Groups / tags | MEDIUM | MEDIUM | P3 |
| Sync / plugins / CLI | LOW for current milestone | HIGH | P3 |

**Priority key:**
- P1: Must have for the next planned milestone
- P2: Should have once the contract is reliable
- P3: Future scope, not part of current roadmap

## Competitor Feature Analysis

| Feature | CopyQ | Maccy | Our Approach |
|---------|-------|-------|--------------|
| Search and filtering | Strong search plus scripting and multiple data types | Fast local search-focused UX | Prioritize trustworthy recall and correct retention semantics before adding more surface area |
| Rich content support | Handles many content types and advanced workflows | Optimized around a lighter focused workflow | Be explicit about fidelity level per type instead of over-claiming support |
| Privacy and exclusions | Strong power-user controls | Focused local experience and exclusion settings | Keep local-first trust front and center; add exclusions only after baseline reliability work |
| Organization and automation | Tabs, commands, and scripting | Lightweight favorites/history model | Defer plugins and CLI until core behavior is stable |

## Sources

- [CopyQ](https://copyq.net/) — official product site and documented feature set for a mature desktop clipboard manager
- [CopyQ documentation](https://copyq.readthedocs.io/en/latest/) — official docs for supported formats, limits, and advanced workflows
- [Maccy](https://maccy.app/) — official site describing expectations for a fast local clipboard utility
- [.planning/PROJECT.md](../PROJECT.md) — current ClipMan scope and brownfield priorities
- [.planning/codebase/CONCERNS.md](../codebase/CONCERNS.md) — current product and implementation gaps

---
*Feature research for: local-first desktop clipboard manager*
*Researched: 2026-04-11*
