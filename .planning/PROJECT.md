# ClipMan

## What This Is

ClipMan is a local-first desktop clipboard manager built with Rust, Tauri, and Svelte for people who need fast, persistent access to copied text and images across Windows, macOS, and Linux. It captures clipboard history, lets users search, pin, and restore entries from a lightweight main window or tray menu, and provides settings for storage, theme, language, autostart, shortcuts, and optional encryption.

## Core Value

Clipboard history must stay fast, trustworthy, and instantly retrievable without turning a simple desktop utility into a fragile, heavyweight system.

## Requirements

### Validated

- ✓ User can capture clipboard text and image content automatically and keep history across app restarts — existing
- ✓ User can search, pin, delete, clear, and copy history items from the main window — existing
- ✓ User can open the app with a global shortcut and access recent and pinned entries from the tray menu — existing
- ✓ User can configure theme, locale, startup, storage path, retention-related settings, and tray behavior from the settings UI — existing
- ✓ User can enable local encryption and receive packaged app updates through the desktop client — existing

### Active

- [ ] Search, retention, and settings behavior match what the UI and README promise, especially around full-history search and history limits
- [ ] Changing the data directory is safe at runtime and does not risk stale database handles, duplicate monitors, or silent data loss
- [ ] Permission failures, migration failures, and other backend errors surface clearly in the UI with recoverable next steps
- [ ] Clipboard content fidelity for HTML, RTF, files, and large images is either preserved correctly or clearly degraded by design
- [ ] Verification from a clean checkout is reproducible, and core clipboard and storage flows gain automated coverage
- [ ] Image handling moves toward configurable compression without regressing current local-first performance and privacy expectations

### Out of Scope

- Multi-device sync or account-backed cloud features — requires backend and identity architecture that the current local-first product does not have
- Plugin platform and public extension API — expands the product surface before the core desktop contract is stable
- CLI-first workflows and automation surfaces — valuable later, but not before current desktop reliability and verification gaps are closed

## Context

- Brownfield Tauri 2 + Svelte 5 + Rust + SQLite application currently released as version `1.10.0`
- Existing repo already ships cross-platform desktop builds, bilingual UI, tray access, updater support, local encryption, custom storage, and global shortcuts
- `.planning/codebase/` documents the current architecture as a single-process desktop monolith with Tauri IPC between Svelte stores/components and Rust services
- README marketing currently promises FTS5-style full-text search, but `.planning/codebase/CONCERNS.md` shows search behavior, settings semantics, and migration lifecycle are weaker than the public contract suggests
- Current automated coverage is limited to a few Rust unit tests; frontend state, clipboard monitoring, command flows, and clean-checkout verification are largely untested

## Constraints

- **Tech stack**: Stay within the existing Tauri 2, Rust, SQLite, and Svelte 5 architecture — improve the product without rewriting its foundation
- **Product scope**: Keep the app local-first and lightweight — avoid introducing account systems or always-on cloud dependencies unless the core value changes
- **Compatibility**: Maintain Windows, macOS, and Linux desktop packaging — the product already promises cross-platform availability
- **Performance**: Preserve fast startup and low idle overhead — clipboard utilities lose value quickly when they feel heavy
- **Security**: Improve real-world protection for local clipboard data without overstating the threat model — current key storage and capability scope need hardening
- **Operability**: Planning artifacts should stay committed in git for this repo — the workspace already uses GSD/Codex planning flows

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Local-first desktop app remains the product center | Existing code, README positioning, and user value all center on fast private local use | ✓ Good |
| Brownfield initialization uses current shipped capabilities as Validated requirements | The repo already has real behavior to anchor planning against | ✓ Good |
| Current active scope prioritizes reliability, contract alignment, and testability before major new surfaces | Codebase concerns show product trust is the main risk | — Pending |
| Image compression is treated as an extension of current media handling, not a separate platform pivot | It is already on the public roadmap and fits the current architecture | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-11 after initialization*
