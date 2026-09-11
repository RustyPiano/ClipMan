# AGENTS.md

Guidance for AI coding agents working on **ClipMan**. For human-facing docs see [`README.md`](README.md).

## Project Overview

ClipMan is a **local-first desktop clipboard manager** (Windows / macOS / Linux). It captures clipboard history (plain text, HTML rich text, images, file lists), lets users search / pin / restore / merge-paste entries from a QuickBar panel or the system tray, and persists everything in a local SQLite database (FTS5 search, keyset pagination, secret-detection and app-ignore privacy filters).

- **Backend:** Rust 2021 + Tauri 2.11 (`src-tauri/`) — clipboard monitoring, SQLite storage, tray, global shortcuts, settings, updater.
- **Frontend:** Svelte 5 (runes) + TypeScript + Tailwind CSS 4, built with Vite 8 (`src/`).
- **IPC:** Tauri `invoke`/`emit` is the _only_ boundary between UI and backend.
- **Architecture:** single desktop process, no network tier. Shared `AppState` (`Arc<Mutex<…>>`) in `src-tauri/src/main.rs`; thin command adapters in `commands.rs` over `storage`/`settings`/`tray`/`clipboard`/`migration` modules.

## 📋 Documentation map & maintenance protocol (read this first)

**Start every session by reading [`docs/dev/STATUS.md`](docs/dev/STATUS.md)** — the living "where the project is right now" doc (a SessionStart hook also injects it automatically). Doc roles:

| Doc | Role | Update policy |
| --- | --- | --- |
| `docs/dev/STATUS.md` | Current state, uncommitted work, TODO queue, known issues | **Update before ending any session that changes `src/` or `src-tauri/`** (a Stop hook reminds you). Keep ≤100 lines; delete stale entries instead of appending forever. |
| `AGENTS.md` (this file) | Stable knowledge: architecture, conventions, commands, gotchas | Update in the same session whenever conventions/architecture/commands change. Never let it describe a past state as present. |
| `docs/dev/PLAN.md` + `SPEC-*.md` | Dated execution records of finished dev waves (specs, acceptance verdicts, deviations) | Historical once a wave is accepted — append status corrections only, don't rewrite. |
| `docs/archive/` | Superseded docs (kept for history, each with a banner) | Read-only. **Never treat archived docs as current guidance.** |
| `README.md` / `README_EN.md` | Human/user-facing feature docs | Update when user-visible features ship (typically at release time). |

Maintenance rules for agents:

1. New docs about in-progress work go under `docs/dev/`; when superseded, move them to `docs/archive/` with a dated "superseded by …" banner at the top — don't leave two docs claiming to be current.
2. If you find a contradiction between a doc and the code, the code is the truth; fix or archive the doc in the same session and note it in `STATUS.md`.

## Setup Commands

Use **Bun** (matches CI); `bun.lock` is the single committed dependency lockfile.

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

## Testing & quality gates

Backend: inline Rust unit tests (`#[cfg(test)]` in most `src-tauri/src/*.rs` modules). Frontend: `bun:test` suites under `tests/frontend/`. CI (`.github/workflows/ci.yml`) enforces all of the below on push/PR — **keep every gate green before finishing any change**:

```bash
cd src-tauri && cargo test                             # Rust unit tests
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo fmt --check
bun run lint                   # ESLint over src/
bun run check                  # svelte-check + TypeScript type check
bun run test:types             # TypeScript-check frontend test files
bun test tests/                # frontend unit tests
bun run test:ui                # Chromium + WebKit interaction tests (install browsers once)
bun run build                  # frontend bundle (cargo test also needs dist/ to exist)
```

Add or update tests for logic you change (Rust and frontend both).
Browser setup: `bunx playwright install chromium webkit` (Linux CI adds `--with-deps`).
`e2e/` exercises compiled Svelte with a synthetic IPC backend; it does not replace native focus/permission tests.
Native macOS smoke: `python3 scripts/native-qa/run.py "/absolute/path/ClipMan QA.app"` (requires an isolated `com.clipman.nativeqa` build and Accessibility grants). `--storage-only` checks thumbnails/tray failures without auto-paste permission; see `docs/dev/REFACTOR-QUICKBAR.md`.
`bun run test:types` checks both Bun and browser suites. Optional synthetic DB benchmark:
`cd src-tauri && cargo test --release search_latency_baseline -- --ignored --nocapture`.

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
- Svelte 5 rune stores: `*.svelte.ts` with singleton named exports (`clipboardStore`, `themeStore`, `toastStore`, `i18n`).
- **Tauri commands are snake_case and must match the string passed to `invoke('…')`.** Register every command in the `invoke_handler![]` macro in `main.rs`.
- **IPC boundary uses camelCase:** Rust stays snake_case internally but serializes with `#[serde(rename_all = "camelCase")]` (see `settings.rs`, `storage.rs`). Keep TS types in `src/lib/types.ts` in sync.
- `update_settings` takes a settings object to save, or `null` to reset from Rust defaults through the same validation/rollback path. The backend preserves migration-owned data paths and tray-owned capture pause state. The frontend has no duplicate defaults; failed loading disables saves and offers retry.
- Settings writes and data migration run through `spawn_blocking`; never wait for `settings_write_lock` from a Tauri main-thread callback. A failed persisted-settings load pauses capture and keeps the original store for recovery.
- Command boundary returns `Result<T, String>`; service code uses `map_err(|e| e.to_string())`. UI wraps calls in try/catch and surfaces failures via toast/inline text.
- Constants: `UPPER_SNAKE_CASE` in both TS and Rust.

## Where Things Live

| Area                                                   | Path                                                           |
| ------------------------------------------------------ | -------------------------------------------------------------- |
| App entry, AppState, tray setup, shortcuts, startup recovery | `src-tauri/src/main.rs`                                  |
| Tauri command handlers (IPC)                           | `src-tauri/src/commands.rs`                                    |
| SQLite storage + dedup + history limits + pagination   | `src-tauri/src/storage.rs`                                     |
| Clipboard monitoring (single-representation snapshot: Files > Text+html > Image) | `src-tauri/src/clipboard.rs`         |
| Clipboard write-back + paste simulation (single & merge) | `src-tauri/src/paste.rs`                                     |
| Secret detection (skip PEM/AWS/JWT/token captures)     | `src-tauri/src/secrets.rs`                                     |
| QuickBar window (NSPanel, positioning, native shadow)  | `src-tauri/src/window.rs`                                      |
| macOS Accessibility permission flow                    | `src-tauri/src/accessibility.rs`                               |
| Tray menu (dynamic, rebuilt on change)                 | `src-tauri/src/tray.rs`                                        |
| Settings persistence (tauri-plugin-store)              | `src-tauri/src/settings.rs`                                    |
| Data dir / migration                                   | `src-tauri/src/migration.rs`                                   |
| Frontend unit tests (bun:test)                         | `tests/frontend/*.test.ts`                                     |
| Frontend stores                                        | `src/lib/stores/*.svelte.ts`                                   |
| UI components                                          | `src/lib/components/**`, `src/routes/**`                       |
| Shared TS types                                        | `src/lib/types.ts`                                             |
| Native + frontend config                               | `src-tauri/tauri.conf.json`, `vite.config.js`, `tsconfig.json` |

`$lib/*` maps to `src/lib/*` (in both `tsconfig.json` and `vite.config.js`).

## Platform Notes

- **macOS:** runs as an Accessory (menu-bar) app; clipboard access and auto-paste (simulating Cmd+V) need **Accessibility permission**. Without it `CGEventPost` fails *silently*, so `accessibility.rs` checks `AXIsProcessTrusted()` before pasting and guides the user to re-grant (native dialog + QuickBar banner) rather than failing silently. Releases are signed with a stable self-signed cert so the grant survives updates (see `.github/RELEASE_GUIDE.md`).
- **Windows:** `windows` crate features are already enabled for Win32 windowing/dataexchange.
- **Linux:** auto-paste support is best-effort (Wayland limited); degrade to copy-only, don't error.

## Commits & PRs

- Use **Conventional Commits** (`feat(scope):`, `fix:`, `docs:`, `refactor:`, `chore:`).
- This is a solo repo — **commit directly to `main`** (no feature branch needed). Only commit when asked.
- Stage only the files relevant to your change; don't sweep in unrelated working-tree edits.
- Before committing code: `bun run check`, `bun run lint`, and `cargo build` should pass.

## Gotchas

- **One clipboard change = one record.** The monitor reads a single representative snapshot per change with priority `Files > Text(+html companion) > Image` (`clipboard.rs`). Never re-introduce independent text/image reads — that's the old double-record bug.
- Self-copy guard: when ClipMan writes to the clipboard it marks `last_copied_by_us` with a normalized `CopyMarker` (Text hashes the plain text only — never the html; Files hash the JSON-encoded "effective" path list — stored paths on macOS, canonicalized elsewhere). A self-copy or dedup skip must still advance `last_marker` — there's a test locking this.
- globalPlain paste mode rewrites the live clipboard to plain text. The strip must (a) also run on marker-unchanged re-copies of the same rich text (behind the dedup early-return, excluding our own rich writes via the self-copy marker), (b) key off the *pre-clamp* html, (c) only run after `save_to_storage` succeeded (the live clipboard may hold the only rich copy), (d) hold `clipboard_use_lock` (skip if contended), and (e) re-flatten ⌥Enter's rich residue after the marker TTL only when the clipboard still holds that exact write. Skipped captures (ignored apps, secrets, oversized, paused) intentionally keep a rich clipboard.
- **macOS file paste must NOT go through arboard, and "write succeeded" must be verified.** Two layered traps, both empirically proven on macOS 26: (1) arboard's `file_list` canonicalizes (stats) every path and TCC denies that to a Finder-launched GUI app — `paste.rs::write_file_list` therefore writes `NSURL`s to `NSPasteboard` directly. (2) The Tahoe pasteboard server **validates the writer's access to each file URL and silently drops unauthorized items while `writeObjects` still returns `true`** (Desktop file → `items=0`; `/Users/Shared` file → pastes fine). So `write_file_list` pre-opens each file (surfaces the one-time Files-and-Folders TCC prompt; terminal-launched processes inherit the terminal's grants, which masks all of this) and then confirms `pasteboardItems.count > 0` before claiming success. Full Disk Access covers everything including other apps' containers.
- QuickBar surface/focus tokens (`--qb-*`) live in `src/app.css`, scoped to `.quickbar-panel`. Row action backgrounds use the row surface, avoiding unrelated white/dark toolbar patches.
- QuickBar shadow is the **native macOS window shadow** derived from the window's alpha shape. Do not add CSS drop shadows or translucent pixels around `.quickbar-panel` — they distort the alpha shape and the shadow renders as a gray halo. Windows keeps `shadow: false` (DWM shadows follow the rectangular frame).
- Tray menu is rebuilt from the DB on every clipboard change — keep that path cheap. Propagate query/build errors and preserve the previous menu on refresh failure.
- Runtime data-path reporting and migration source/cleanup come from `ClipStorage::data_directory()`, the canonical parent of the database actually opened. `custom_data_path` remains only the next-start preference.
- Images always store original + thumbnail. If both dimensions are ≤256px, reuse the original PNG bytes for the thumbnail; never upscale small images. Do not reintroduce `store_original_image`.
- Clip timestamps are fractional Unix seconds (`f64` / JS number); preserve fractions through capture, dedup, paste history touch and pagination cursors. SQLite reads legacy integer seconds alongside new real values; do not truncate to whole seconds.
- Database format v3 stores file lists as JSON string arrays while reading legacy newline-separated rows. Upgrades create a SQLite backup before changing rows and reject newer schema versions; do not lower `CURRENT_DB_USER_VERSION` or write newline-joined file records.
- Search uses SQLite FTS5 with a full-text short-query LIKE fallback. Results contain match excerpts and at most 1001 rows (1000 visible + sentinel); keep FTS maintenance in storage. Limit FTS candidates before generating excerpts.
- QuickBar rows are 5.5rem tall (three sections: label/title line, content preview — two text lines / two file paths + dirname / inline image thumbnail — and a metadata line) and virtualized. Keep the CSS height and the measured rem size in sync; do not reintroduce per-arrow `flushSync`/row geometry reads. Selection/multi-selection belong to `selectionStore`, keyed by ID. Common mouse actions sit in a bottom-right overlay above the metadata line's reserved right padding (`pr-[8.75rem]`) — the text itself is never covered and action buttons must stay OUTSIDE the row's main `<button>` (nested buttons make action clicks bubble into a paste); label editing replaces the row body without changing height. Results use grid/row/gridcell semantics to expose action buttons accessibly. Alt+Left/Right switches panels; Tab follows native focus navigation.
- `get_clip` returns decoded `ClipDetail` (`text` / optional original `imageUrl`), distinct from base64 list summaries. Small text details have one bounded cache (16 × 256K UTF-16 units). SQL caps text/file details at 1 MiB and rejects images above 16 MiB before returning their BLOB; `truncated` drives the preview notice. Copy/paste still read complete content.
- Copy, single paste and merge paste share `paste::use_clips`, guarded across all callers. Write succeeds before history touch; `clips-used` invalidates the entire affected view after the action, while `clipboard-changed` remains the capture upsert event. Results distinguish `copied`, `pasteRequested`, and `copiedOnly`; never turn post-write maintenance failure into a retryable paste failure.
- Merge paste has a 50 MB aggregate text budget. History limits are enforced on new/duplicate capture, settings reduction, and unpinning; keep pruning and FTS updates in the same storage transaction.
- History initialization and append pagination have independent request sequences. Background invalidations share one running refresh with a pending flag. Incoming capture events are retained only during a history reload; do not restore a permanent duplicate event cache.
- App exclusions on macOS accept bundle identifiers as well as legacy display names; Windows/Linux display their unsupported status. New settings choose the system language; saved language preferences remain authoritative.
