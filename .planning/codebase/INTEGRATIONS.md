# External Integrations

**Analysis Date:** 2026-04-11

## APIs & External Services

**Release Distribution & Updates:**
- GitHub Releases - Hosts packaged desktop binaries and the updater manifest consumed by the app.
  - SDK/Client: `tauri-plugin-updater` on the Rust side in `src-tauri/src/commands.rs`, configured in `src-tauri/tauri.conf.json`.
  - Auth: No runtime API credential is embedded in the app; update trust is anchored by the `pubkey` in `src-tauri/tauri.conf.json`.
  - Endpoints used: `https://github.com/RustyPiano/ClipMan/releases/latest/download/latest.json`.
- GitHub Actions release publishing - Builds and drafts tagged releases for distribution.
  - SDK/Client: `tauri-apps/tauri-action@dev` and standard GitHub Actions steps in `.github/workflows/release.yml`.
  - Auth: `GITHUB_TOKEN`, `TAURI_SIGNING_PRIVATE_KEY`, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are expected as repository secrets in `.github/workflows/release.yml`.

**Desktop Platform Services:**
- Operating system clipboard - Core product integration for capture and write-back.
  - SDK/Client: `arboard` and `clipboard-master` in `src-tauri/src/clipboard.rs` and `src-tauri/src/commands.rs`; frontend requests flow through `@tauri-apps/api/core` in `src/lib/stores/clipboard.svelte.ts`.
  - Auth: Depends on OS clipboard/accessibility permissions; checked via `check_clipboard_permission` in `src-tauri/src/commands.rs` and surfaced in `src/lib/components/PermissionCheck.svelte`.
- Native folder picker - Lets the user choose a custom storage location.
  - SDK/Client: `@tauri-apps/plugin-dialog` in `src/routes/settings/+page.svelte`, with `tauri-plugin-dialog` registered in `src-tauri/src/main.rs`.
  - Auth: No credential; user-driven native dialog selection.
- Global shortcuts, notifications, autostart, and tray - OS shell integrations used by the desktop app shell.
  - SDK/Client: `tauri-plugin-global-shortcut`, `tauri-plugin-notification`, `tauri-plugin-autostart`, and Tauri tray APIs in `src-tauri/src/main.rs` and `src-tauri/src/commands.rs`.
  - Auth: OS policy and permissions only; no service credentials.
- Local folder opening - Opens the active data directory in the platform file explorer.
  - SDK/Client: Direct `std::process::Command` calls (`open`, `explorer`, `xdg-open`) in `src-tauri/src/commands.rs`.
  - Auth: None.

## Data Storage

**Databases:**
- Local SQLite database - Primary storage for clipboard history, pin state, timestamps, and hashes.
  - Connection: `app.path().app_data_dir()` by default or a user-selected custom path via `migration::get_data_directory(...)` in `src-tauri/src/main.rs` and `src-tauri/src/migration.rs`.
  - Client: `rusqlite` with bundled SQLite in `src-tauri/src/storage.rs`.
  - Migrations: Inline schema checks and `ALTER TABLE` migration logic in `src-tauri/src/storage.rs`.

**File Storage:**
- Local application data directory - Stores `clipman.db`, SQLite WAL/SHM sidecar files, and the encryption key `.clipman.key`.
  - SDK/Client: Rust `std::fs`, Tauri path APIs, and migration helpers in `src-tauri/src/main.rs` and `src-tauri/src/migration.rs`.
  - Auth: Filesystem permissions only.
  - User control: Storage location can be changed from the settings UI in `src/routes/settings/+page.svelte`.

**Caching:**
- No external cache service detected.
- In-memory only: tray icon thumbnails are cached with `lru` in `src-tauri/src/tray.rs`.

## Authentication & Identity

**Auth Provider:**
- None detected.
  - Implementation: No user account system, JWT handling, OAuth SDK, or external identity provider appears in `src/` or `src-tauri/src/`.
  - Token storage: Not applicable.
  - Session management: Not applicable.

**OAuth Integrations:**
- None detected.

## Monitoring & Observability

**Error Tracking:**
- None detected. No Sentry, Bugsnag, Rollbar, or similar SDK imports were found in `src/`, `src-tauri/src/`, or `package.json`.

**Analytics:**
- None detected. No product analytics or telemetry SDK imports were found.

**Logs:**
- Local application logging only.
  - Integration: `env_logger` and `log` are initialized in `src-tauri/src/main.rs`.
  - Remote sink: None detected.

## CI/CD & Deployment

**Hosting:**
- GitHub Releases - Distribution target for installers and updater artifacts.
  - Deployment: Triggered by semver-like git tags in `.github/workflows/release.yml`.
  - Environment vars/secrets: GitHub-hosted secrets plus the default `GITHUB_TOKEN`.

**CI Pipeline:**
- GitHub Actions - Multi-platform release pipeline.
  - Workflows: `.github/workflows/release.yml`.
  - Platforms: macOS ARM64, macOS x64, Ubuntu 22.04, and Windows in `.github/workflows/release.yml`.

## Environment Configuration

**Development:**
- Required env vars: None detected for normal app execution.
- Optional env vars: `TAURI_DEV_HOST` and `TAURI_DEBUG` influence Vite dev/build behavior in `vite.config.js`.
- Secrets location: None detected for local development; release-only signing secrets live in GitHub Actions.
- Mock/stub services: None. Most dependencies are local OS services rather than remote APIs.

**Staging:**
- Not detected. No separate staging service endpoints, config files, or deployment workflow were found.

**Production:**
- Secrets management: GitHub Actions repository secrets for release signing only.
- Redundancy/failover: Not detected. Updater availability depends on GitHub Releases availability.

## Webhooks & Callbacks

**Incoming:**
- None detected. No webhook endpoints, callback routes, or signature verification logic were found.

**Outgoing:**
- None detected. The app polls the updater manifest URL through the Tauri updater plugin rather than sending outbound webhooks.

---

*Integration audit: 2026-04-11*
