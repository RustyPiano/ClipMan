# Stack Research

**Domain:** Local-first desktop clipboard manager
**Researched:** 2026-04-11
**Confidence:** MEDIUM

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Tauri | 2.x | Desktop shell, tray, shortcuts, updater, native integrations | ClipMan already ships on Tauri 2 and the official plugin surface covers the OS integration this product needs without paying Electron-level runtime cost. |
| Rust | 2021 / stable | Clipboard monitoring, storage, migration, crypto, media processing | The current backend is already Rust, and this class of long-lived, IO-heavy desktop process benefits from explicit ownership and mature systems crates. |
| Svelte | 5.x | Desktop UI rendering and local state orchestration | The current UI already uses Svelte 5. Keeping the existing frontend avoids a rewrite while preserving the low-overhead webview profile expected from a clipboard utility. |
| SQLite | 3.x with FTS5 enabled | Local persistence and indexed search | SQLite remains the right local-first database. FTS5 is the correct indexed search primitive for retained text history and is preferable to decrypt-and-scan behavior. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `rusqlite` | 0.32.x | Local database access from Rust | Keep for the existing storage layer unless async or connection-pooling needs become dominant. |
| `tauri-plugin-store` | 2.x | App settings persistence | Keep for user settings and avoid splitting settings semantics across multiple ad hoc files. |
| `tauri-plugin-updater` | 2.x | Signed desktop update flow | Keep because the app already ships updater artifacts and users expect desktop release continuity. |
| `tauri-plugin-global-shortcut` / `autostart` / `notification` | 2.x | Core OS affordances for a desktop utility | These remain part of the core product surface and should be hardened rather than replaced. |
| `image` | 0.25.x | Image normalization, preview generation, format inspection | Keep as the baseline image pipeline. Add format-specific encoders only when configurable compression ships. |
| Secret storage layer | current Tauri stronghold line or OS credential storage | Protection for encryption keys | Use when ClipMan wants stronger local-security claims than a colocated key file provides. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `svelte-check` | Svelte and TypeScript validation | Already present; make it part of the clean-checkout verification baseline. |
| ESLint 9.x | Frontend linting | Already present; should run in CI before release packaging. |
| Prettier 3.x | Frontend formatting | Keep aligned with current Svelte formatting conventions. |
| Cargo test | Rust unit and integration checks | Expand beyond helper tests into storage, migration, and lifecycle behavior. |
| Frontend test runner | Component and store regression coverage | Add a current Svelte-compatible runner during implementation planning rather than continuing with manual-only validation. |

## Installation

```bash
# Existing core stack
bun install
cargo fetch --manifest-path src-tauri/Cargo.toml

# Verification baseline
bun run check
bun run lint
cargo test --manifest-path src-tauri/Cargo.toml
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Tauri 2 | Electron | Use Electron only if the app eventually needs mature browser-engine-specific APIs or a much heavier plugin ecosystem than Tauri can support. |
| SQLite + FTS5 projection | In-memory substring scan over decrypted rows | Use substring scan only as a temporary fallback while the UI explicitly documents search limits. |
| Local-first desktop architecture | Cloud-synced account architecture | Use cloud architecture only if multi-device sync becomes the core product value rather than a future extension. |
| Keychain/stronghold-backed key storage | Key file beside database | Keep colocated keys only as a clearly documented weak mode, not the long-term security story. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Decrypt-and-scan search as the long-term model | Search quality, latency, and privacy semantics stay ambiguous | Use an explicit indexed search projection with documented encrypted-item behavior. |
| Runtime migration that only copies files and edits config | Leaves stale DB, crypto, and monitor state alive | Use staged lifecycle ownership: quiesce, verify, reopen, then swap. |
| Lossy image compression on the clipboard capture hot path | Turns storage optimization into latency and fidelity regressions | Keep canonical capture separate from previews and optional background compression. |
| Expanding desktop capabilities without audit | A simple local app accumulates unnecessary attack surface quickly | Keep Tauri permissions and plugins minimal and intentional. |

## Stack Patterns by Variant

**If search must remain full-history and privacy-sensitive:**
- Keep SQLite as the source of truth.
- Add an explicit search projection or shadow index for searchable content.
- Document how encrypted items participate before shipping UI claims.

**If image compression becomes user-facing:**
- Preserve a canonical representation first.
- Generate previews and optional compressed derivatives separately.
- Make the default mode safe and reversible where possible.

**If stronger security claims are added:**
- Move key storage out of the user-selected data directory.
- Treat storage migration and key migration as the same design problem.
- Align README and settings copy with the actual threat model.

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| Tauri 2.x | Svelte 5.x + Vite 6.x | This is the current repo baseline and should remain the incremental path. |
| `rusqlite` 0.32.x | bundled SQLite with FTS5 | Fits the current storage layer and supports a proper local search index. |
| `image` 0.25.x | current Rust backend pipeline | Good baseline for image preview and format work before specialized encoders are introduced. |

## Sources

- [Tauri Plugins](https://v2.tauri.app/plugin/) — official Tauri 2 plugin surface for updater, store, global shortcuts, autostart, and stronghold
- [SQLite FTS5](https://sqlite.org/fts5.html) — official documentation for full-text indexing behavior and query model
- [Svelte](https://svelte.dev/) — official Svelte documentation and current framework line
- [Image crate docs](https://docs.rs/image/latest/image/) — current Rust image-processing capabilities relevant to preview and format work
- [.planning/PROJECT.md](../PROJECT.md) — project-specific brownfield context and current scope
- [.planning/codebase/STACK.md](../codebase/STACK.md) — current implementation baseline

---
*Stack research for: local-first desktop clipboard manager*
*Researched: 2026-04-11*
