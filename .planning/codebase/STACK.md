# Technology Stack

**Analysis Date:** 2026-04-11

## Languages

**Primary:**
- Rust 2021 edition - Native desktop backend, Tauri setup, clipboard monitoring, storage, tray handling, and command handlers in `src-tauri/src/main.rs`, `src-tauri/src/clipboard.rs`, `src-tauri/src/storage.rs`, `src-tauri/src/commands.rs`, and related modules.
- TypeScript (strict, ES2020 target) - Frontend state, types, i18n, and Tauri bridge usage in `src/main.ts`, `src/lib/stores/*.svelte.ts`, `src/lib/types.ts`, and `src/routes/**/*.svelte`.

**Secondary:**
- JavaScript ES modules - Build and lint configuration in `vite.config.js`, `svelte.config.js`, and `eslint.config.js`.
- CSS with Tailwind CSS 4 directives - Application theme tokens and global styling in `src/app.css`.
- JSON and TOML - Desktop app configuration and dependency manifests in `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `package.json`, and `src-tauri/Cargo.toml`.

## Runtime

**Environment:**
- Tauri 2 desktop runtime - Native shell and WebView host configured in `src-tauri/src/main.rs` and `src-tauri/tauri.conf.json`.
- Rust stable toolchain - README documents Rust `1.82+` for development in `README.md`.
- Node.js or Bun for frontend commands - `src-tauri/tauri.conf.json` runs `npm run dev` / `npm run build`, while `README.md` and `.github/workflows/release.yml` use Bun commands and `bun install`.

**Package Manager:**
- Mixed npm/Bun workflow on the frontend - `package-lock.json` and `bun.lock` are both present at repo root.
- Cargo for the native backend - `src-tauri/Cargo.lock` is present and tracks Rust dependencies.

## Frameworks

**Core:**
- Tauri `2.9` (`tauri`) and Tauri CLI `2.x` - Desktop application framework, window lifecycle, tray integration, and invoke bridge in `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, and `src-tauri/tauri.conf.json`.
- Svelte `5.37.0` - Frontend component system and runes-based state in `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`, and `src/lib/stores/*.svelte.ts`.
- Tailwind CSS `4.x` - Utility styling and theme token wiring in `src/app.css` and `vite.config.js`.

**Testing:**
- No dedicated JavaScript or end-to-end test framework detected in `package.json`.
- Rust unit tests only - Inline tests exist in `src-tauri/src/crypto.rs` and `src-tauri/src/migration.rs`.

**Build/Dev:**
- Vite `6.x` - Dev server, HMR, aliasing, and production bundling in `vite.config.js`.
- `@sveltejs/vite-plugin-svelte` `5.x` with `vitePreprocess()` - Svelte compilation setup in `vite.config.js` and `svelte.config.js`.
- TypeScript compiler in strict bundler mode - Configured in `tsconfig.json` and `tsconfig.node.json`.
- ESLint `9.x` with TypeScript and Svelte support - Configured in `eslint.config.js`.
- Prettier `3.x` with `prettier-plugin-svelte` - Configured in `.prettierrc`.

## Key Dependencies

**Critical:**
- `tauri` `2.9` - Core desktop shell, tray, window, and command registration in `src-tauri/src/main.rs`.
- `svelte` `5.37.0` - UI rendering and stateful frontend components across `src/routes/` and `src/lib/components/`.
- `rusqlite` `0.32` with `bundled` SQLite - Embedded persistence layer in `src-tauri/src/storage.rs`.
- `arboard` `3.4` and `clipboard-master` `4.0.0-beta.6` - Clipboard read/write plus event-driven monitoring in `src-tauri/src/clipboard.rs` and `src-tauri/src/commands.rs`.
- `ring` `0.17` - AES-256-GCM encryption for stored clipboard payloads in `src-tauri/src/crypto.rs` and key bootstrapping in `src-tauri/src/main.rs`.

**Infrastructure:**
- `@tauri-apps/api` `2.0.0` - Frontend invoke and event bridge used by `src/lib/stores/clipboard.svelte.ts` and `src/routes/settings/+page.svelte`.
- `tauri-plugin-updater` `2.1` - Update checks and in-app installs in `src-tauri/src/commands.rs`.
- `tauri-plugin-store` `2` - Persistent settings storage in `src-tauri/src/settings.rs`.
- `tauri-plugin-global-shortcut`, `tauri-plugin-notification`, `tauri-plugin-autostart` - OS integrations wired in `src-tauri/src/main.rs` and `src-tauri/src/commands.rs`.
- `lucide-svelte` `0.554.0` - Shared icon set throughout `src/lib/components/` and `src/routes/`.

## Configuration

**Environment:**
- No `.env`, `.env.local`, or `.env.example` files were detected at repo root.
- Frontend build/runtime behavior reads `TAURI_DEV_HOST` and `TAURI_DEBUG` in `vite.config.js`.
- Desktop update trust is configured with an embedded updater public key and GitHub endpoint in `src-tauri/tauri.conf.json`.
- Release signing secrets are expected only in GitHub Actions: `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` in `.github/workflows/release.yml`.

**Build:**
- Frontend build configuration: `vite.config.js`, `svelte.config.js`, `tsconfig.json`, `tsconfig.node.json`.
- Native desktop configuration: `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/build.rs`, `src-tauri/Cargo.toml`.
- Code quality configuration: `eslint.config.js`, `.prettierrc`.

## Platform Requirements

**Development:**
- Desktop OS with Tauri/WebView support. `README.md` lists Windows 10+, macOS 10.13+, and Linux.
- Node.js `18+` or Bun plus Rust `1.82+` per `README.md`.
- Linux release builds install `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, and `patchelf` in `.github/workflows/release.yml`.

**Production:**
- Packaged desktop binaries for macOS, Windows, and Linux via Tauri bundling in `src-tauri/tauri.conf.json`.
- Auto-update artifacts are enabled with `"createUpdaterArtifacts": true` in `src-tauri/tauri.conf.json`.
- GitHub Releases is the current distribution target, created by `.github/workflows/release.yml`.

---

*Stack analysis: 2026-04-11*
