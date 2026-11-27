# AGENTS.md

Project guidance for AI coding agents working on **ClipLeaf**. User-facing docs live in [`README.md`](README.md) / [`README_EN.md`](README_EN.md).

## Overview

ClipLeaf is a local-first desktop clipboard manager (macOS / Windows / Linux) in a single desktop process with no network tier:

- **Backend (`src-tauri/`):** Rust 2021 + Tauri 2.11, SQLite (FTS5 + keyset pagination), clipboard monitor, tray, global shortcuts, settings, updater.
- **Frontend (`src/`):** Svelte 5 (runes) + TypeScript + Tailwind CSS 4 + Vite 8 (`$lib/*` -> `src/lib/*`).
- **Boundary:** Tauri `invoke`/`emit` is the only interface between UI and Rust.

## Documentation Map

| Doc | Role | Update Rule |
| --- | --- | --- |
| [`docs/dev/STATUS.md`](docs/dev/STATUS.md) | Current state, TODOs, known issues | **Read first every session.** Update whenever code or release/CI state changes; keep ≤ 100 lines and delete stale items. |
| `AGENTS.md` | Commands, enforced conventions, non-inferable gotchas | Update when tooling or core conventions change. Do not duplicate ADR rationales here. |
| `docs/adr/` | Architectural & platform design decisions (0001–0023) | Read the relevant ADR before modifying capture, paste/focus, SQLite schema, macOS windowing, settings, release flows, or app identity/naming; update or replace ADRs when decisions change. |
| `README.md` / `README_EN.md` | User-facing feature & install docs | Update at release time when user-visible features change. |

If a doc contradicts the code, the code is truth — fix or delete the stale doc in the same session.

## Commands

Use **Bun** (`bun.lock` is the single frontend lockfile). Toolchain: Bun 1.3+, Rust 1.96.0 (`rust-toolchain.toml`). Linux also needs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.

```bash
bun install            # install frontend deps
bun tauri dev          # full desktop app (do not open `bun run dev` in a plain browser)
bun run build          # frontend bundle -> dist/ (required before `cargo test` / `cargo clippy`)
bun tauri build        # platform installers under src-tauri/target/release/bundle/
bun run format         # prettier --write "src/**/*.{ts,svelte}"
```

### CI Quality Gates (keep all green before finishing a change)

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test
bun run lint
bun run check
bun run test:types
bun test tests/
bun run build
bun run test:ui
python3 -B -m unittest discover -s scripts -p 'test_verify_release.py' -v
```

- `bun run test:ui` runs Chromium + WebKit against compiled Svelte with a synthetic IPC mock (`e2e/`). First-time browser setup: `bunx playwright install chromium webkit`.
- `test_verify_release.py` needs `minisign` (pass `MINISIGN=/path/to/minisign` if not in `PATH`; verification artifacts live in ignored `src-tauri/target/release-verification/`).
- Optional native macOS QA smoke: build an isolated bundle with `bun tauri build --bundles app --config '{"productName":"ClipLeaf QA","identifier":"app.clipleaf.nativeqa","bundle":{"createUpdaterArtifacts":false}}'`, grant Accessibility to terminal + `ClipLeaf QA.app`, then run `python3 scripts/native-qa/run.py "$PWD/src-tauri/target/release/bundle/macos/ClipLeaf QA.app"` (`--storage-only` skips auto-paste).
- Optional search benchmark: `cd src-tauri && cargo test --release search_latency_baseline -- --ignored --nocapture`.
- Release steps and signing rules: [`.github/RELEASE_GUIDE.md`](.github/RELEASE_GUIDE.md).

## Enforced Conventions & Gotchas

- **No code comments** (`docs/adr/0001-no-code-comments.md`): ESLint (`bun run lint`) and `src-tauri/tests/no_comments.rs` (`cargo test`) enforce zero new comments in code, tests, scripts, and configs; legacy suppression counts can only decrease. Put non-obvious rationale in `docs/adr/`.
- **IPC contract:** Tauri command names are `snake_case` and must be registered in `invoke_handler![]` in `src-tauri/src/main.rs`. Payloads serialize as `camelCase` (`#[serde(rename_all = "camelCase")]`) and must stay in sync with `src/lib/types.ts`. Commands return `Result<T, String>`.
- **Fail fast:** Return errors or panic at the fault site instead of silently falling back to alternate paths, formats, or modes, and never treat failed commands as success in UI. `std::sync::Mutex::lock().unwrap()` is intentional because release builds use `panic = "abort"`.
- **TypeScript & Prettier:** Strict TS (`noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`; prefix unused params with `_`). Prettier: 2 spaces, single quotes, semicolons, `es5` trailing commas, width 100.
- **Settings & threading:** Settings persistence and data migration run via `spawn_blocking`; never wait on `settings_write_lock` from a Tauri main-thread callback. Backend owns defaults (`update_settings(null)` resets) and input validation; frontend never duplicates defaults. Locale lives in backend `settings.locale`; theme lives in frontend `localStorage`.
- **Single-capture & privacy:** Clipboard monitor starts once and reads one snapshot per change (`Files > Text(+html) > Image`). All clipboard writes hold `clipboard_use_lock` and update `last_copied_by_us` (`docs/adr/0002`–`0004`, `0012`).
- **Database schema v3:** File lists store JSON string arrays; never lower `CURRENT_DB_USER_VERSION` or write newline-joined file rows (`docs/adr/0008`). Timestamps are fractional Unix seconds (`f64`); never truncate to whole seconds (`docs/adr/0006`).
- **QuickBar UI:** Keep fixed `ROW_HEIGHT_REM` virtualization, non-nested `grid`/`row`/`gridcell` buttons, IME `isComposing || keyCode === 229` guards, and native macOS window shadow without CSS shadows/translucent halos on `.quickbar-panel` (`docs/adr/0014`, `0015`).
- **External links & folders:** Always use `tauri-plugin-opener` (never shell out to `cmd /C start` or raw `spawn()`).
- **Commits:** Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`), directly on `main`, only when asked; stage only relevant files.
