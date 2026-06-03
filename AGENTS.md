# AGENTS.md

Guidance for AI coding agents working on **ClipMan**. For human-facing docs see [`README.md`](README.md).

## Project Overview

ClipMan is a **local-first desktop clipboard manager** (Windows / macOS / Linux). It captures clipboard history (text + images), lets users search / pin / restore entries from a window or the system tray, and persists everything in a local SQLite database.

- **Backend:** Rust 2021 + Tauri 2.11 (`src-tauri/`) — clipboard monitoring, SQLite storage, tray, global shortcuts, settings, updater.
- **Frontend:** Svelte 5 (runes) + TypeScript + Tailwind CSS 4, built with Vite 8 (`src/`).
- **IPC:** Tauri `invoke`/`emit` is the _only_ boundary between UI and backend.
- **Architecture:** single desktop process, no network tier. Shared `AppState` (`Arc<Mutex<…>>`) in `src-tauri/src/main.rs`; thin command adapters in `commands.rs` over `storage`/`settings`/`tray`/`clipboard`/`migration` modules.

## ⚠️ Active redesign — read before changing things

A v2.0.0 UX redesign is in progress. **Read these first; parts of the current code are being intentionally replaced:**

- [`docs/REDESIGN_SPEC.md`](docs/REDESIGN_SPEC.md) — what we're building (QuickBar popup, entry-based auto-paste, pinned/常用 as first-class snippets, **encryption removal**, FTS5, backend single source of truth).
- [`docs/DEVELOPMENT_PLAN.md`](docs/DEVELOPMENT_PLAN.md) — parallel-friendly work packages with **file ownership**, dependencies, and acceptance criteria. Follow the file-ownership table to avoid collisions when multiple agents work concurrently.

Current redesign status: the AES layer has been removed, `crypto.rs` is gone, FTS5 search is implemented, recent/pinned storage queries are split, images now store original content plus thumbnails, and `last_copied_by_us` uses a `CopyMarker` for text and images. QuickBar/window auto-paste code exists, but the platform runtime matrix is not fully verified yet; see [`docs/WP_0_0_SPIKE_RESULTS.md`](docs/WP_0_0_SPIKE_RESULTS.md) before claiming macOS/Windows focus behavior is proven.

## Setup Commands

Use **Bun** (matches CI); npm also works (both `bun.lock` and `package-lock.json` are committed).

```bash
bun install            # install frontend deps (Cargo deps fetch on first build)
```

Requirements: Bun 1.3+ (or Node 20.19+ for frontend tooling), Rust 1.96.0 via `rust-toolchain.toml`. Linux also needs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.

## Development Workflow

```bash
bun tauri dev          # run the full desktop app (Vite + Rust, hot reload)
bun run dev            # frontend only (Vite dev server, no native shell)
```

Note: `tauri.conf.json` wires `beforeDevCommand: bun run dev`, so Bun is the expected frontend runtime.

## Build

```bash
bun tauri build        # produce platform installers (output under src-tauri/target/release/bundle/)
bun run build          # frontend bundle only -> dist/
```

Release is automated via `.github/workflows/release.yml` on pushing a `vX.Y.Z` tag (Tauri updater artifacts + GitHub draft release).

## Testing

There is **no JS/E2E test framework**. Backend has inline Rust unit tests (`#[cfg(test)]`, e.g. in `storage.rs`, `migration.rs`, `paste.rs`, `tray.rs`).

```bash
cd src-tauri && cargo test     # run Rust unit tests
bun run check                  # svelte-check + TypeScript type check
bun run lint                   # ESLint over src/
```

Add or update Rust tests for backend logic you change. Always keep `cargo build` and `bun run build` green.

## Code Style

Frontend formatter is `.prettierrc`: **2-space indent, single quotes, semicolons, `es5` trailing commas, printWidth 100**.

```bash
bun run format         # prettier --write "src/**/*.{ts,svelte}"
```

- TypeScript is strict (`noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`). Prefix intentionally-unused args with `_`.
- Some `src/routes/settings/**` and a few UI files use 4-space/double-quotes — that's **existing inconsistency, not a second style**. Match `.prettierrc` for new/edited code.
- Rust: standard rustfmt layout (4-space, grouped `use`, early-return error checks). No Rust fmt/clippy is wired into npm scripts; run `cargo fmt`/`cargo clippy` manually if needed.
- **Comments mix English and Chinese — match the language already used in the file you're editing.**

## Naming & Conventions

- Rust modules: `snake_case.rs` under `src-tauri/src/`. Structs/enums PascalCase, fields snake_case.
- Svelte components: `PascalCase.svelte` under `src/lib/components/`. Routes follow Svelte conventions (`+page.svelte`).
- Svelte 5 rune stores: `*.svelte.ts` with singleton named exports (`clipboardStore`, `themeStore`, `router`, `toastStore`, `i18n`).
- **Tauri commands are snake_case and must match the string passed to `invoke('…')`.** Register every command in the `invoke_handler![]` macro in `main.rs`.
- **IPC boundary uses camelCase:** Rust stays snake_case internally but serializes with `#[serde(rename_all = "camelCase")]` (see `settings.rs`, `storage.rs`). Keep TS types in `src/lib/types.ts` in sync.
- Command boundary returns `Result<T, String>`; service code uses `map_err(|e| e.to_string())`. UI wraps calls in try/catch and surfaces failures via toast/inline text.
- Constants: `UPPER_SNAKE_CASE` in both TS and Rust.

## Where Things Live

| Area                                                   | Path                                                           |
| ------------------------------------------------------ | -------------------------------------------------------------- |
| App entry, AppState, tray setup, shortcuts             | `src-tauri/src/main.rs`                                        |
| Tauri command handlers (IPC)                           | `src-tauri/src/commands.rs`                                    |
| SQLite storage + dedup + history limits                | `src-tauri/src/storage.rs`                                     |
| Clipboard monitoring (event-driven + polling fallback) | `src-tauri/src/clipboard.rs`                                   |
| Tray menu (dynamic, rebuilt on change)                 | `src-tauri/src/tray.rs`                                        |
| Settings persistence (tauri-plugin-store)              | `src-tauri/src/settings.rs`                                    |
| Data dir / migration                                   | `src-tauri/src/migration.rs`                                   |
| Frontend stores                                        | `src/lib/stores/*.svelte.ts`                                   |
| UI components                                          | `src/lib/components/**`, `src/routes/**`                       |
| Shared TS types                                        | `src/lib/types.ts`                                             |
| Native + frontend config                               | `src-tauri/tauri.conf.json`, `vite.config.js`, `tsconfig.json` |

`$lib/*` maps to `src/lib/*` (in both `tsconfig.json` and `vite.config.js`).

## Platform Notes

- **macOS:** runs as an Accessory (menu-bar) app; clipboard access (and the planned auto-paste) needs **Accessibility permission**. Detect-and-guide rather than failing silently.
- **Windows:** `windows` crate features are already enabled for Win32 windowing/dataexchange.
- **Linux:** auto-paste support is best-effort (Wayland limited); degrade to copy-only, don't error.

## Commits & PRs

- Use **Conventional Commits** (`feat(scope):`, `fix:`, `docs:`, `refactor:`, `chore:`).
- This is a solo repo — **commit directly to `main`** (no feature branch needed). Only commit when asked.
- Stage only the files relevant to your change; don't sweep in unrelated working-tree edits.
- Before committing code: `bun run check`, `bun run lint`, and `cargo build` should pass.

## Gotchas

- Self-copy guard: when ClipMan writes to the clipboard it marks `last_copied_by_us` with a normalized `CopyMarker` so the monitor skips re-capturing text and image copies.
- Tray menu is rebuilt from the DB on every clipboard change — keep that path cheap.
- Image storage is now "always store original + derived thumbnail"; do not reintroduce `store_original_image`.
- Search uses SQLite FTS5 with a short-query LIKE fallback; keep FTS index maintenance in the storage layer.
