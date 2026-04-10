<!-- GSD:project-start source:PROJECT.md -->
## Project

**ClipMan**

ClipMan is a local-first desktop clipboard manager built with Rust, Tauri, and Svelte for people who need fast, persistent access to copied text and images across Windows, macOS, and Linux. It captures clipboard history, lets users search, pin, and restore entries from a lightweight main window or tray menu, and provides settings for storage, theme, language, autostart, shortcuts, and optional encryption.

**Core Value:** Clipboard history must stay fast, trustworthy, and instantly retrievable without turning a simple desktop utility into a fragile, heavyweight system.

### Constraints

- **Tech stack**: Stay within the existing Tauri 2, Rust, SQLite, and Svelte 5 architecture — improve the product without rewriting its foundation
- **Product scope**: Keep the app local-first and lightweight — avoid introducing account systems or always-on cloud dependencies unless the core value changes
- **Compatibility**: Maintain Windows, macOS, and Linux desktop packaging — the product already promises cross-platform availability
- **Performance**: Preserve fast startup and low idle overhead — clipboard utilities lose value quickly when they feel heavy
- **Security**: Improve real-world protection for local clipboard data without overstating the threat model — current key storage and capability scope need hardening
- **Operability**: Planning artifacts should stay committed in git for this repo — the workspace already uses GSD/Codex planning flows
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 2021 edition - Native desktop backend, Tauri setup, clipboard monitoring, storage, tray handling, and command handlers in `src-tauri/src/main.rs`, `src-tauri/src/clipboard.rs`, `src-tauri/src/storage.rs`, `src-tauri/src/commands.rs`, and related modules.
- TypeScript (strict, ES2020 target) - Frontend state, types, i18n, and Tauri bridge usage in `src/main.ts`, `src/lib/stores/*.svelte.ts`, `src/lib/types.ts`, and `src/routes/**/*.svelte`.
- JavaScript ES modules - Build and lint configuration in `vite.config.js`, `svelte.config.js`, and `eslint.config.js`.
- CSS with Tailwind CSS 4 directives - Application theme tokens and global styling in `src/app.css`.
- JSON and TOML - Desktop app configuration and dependency manifests in `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `package.json`, and `src-tauri/Cargo.toml`.
## Runtime
- Tauri 2 desktop runtime - Native shell and WebView host configured in `src-tauri/src/main.rs` and `src-tauri/tauri.conf.json`.
- Rust stable toolchain - README documents Rust `1.82+` for development in `README.md`.
- Node.js or Bun for frontend commands - `src-tauri/tauri.conf.json` runs `npm run dev` / `npm run build`, while `README.md` and `.github/workflows/release.yml` use Bun commands and `bun install`.
- Mixed npm/Bun workflow on the frontend - `package-lock.json` and `bun.lock` are both present at repo root.
- Cargo for the native backend - `src-tauri/Cargo.lock` is present and tracks Rust dependencies.
## Frameworks
- Tauri `2.9` (`tauri`) and Tauri CLI `2.x` - Desktop application framework, window lifecycle, tray integration, and invoke bridge in `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, and `src-tauri/tauri.conf.json`.
- Svelte `5.37.0` - Frontend component system and runes-based state in `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`, and `src/lib/stores/*.svelte.ts`.
- Tailwind CSS `4.x` - Utility styling and theme token wiring in `src/app.css` and `vite.config.js`.
- No dedicated JavaScript or end-to-end test framework detected in `package.json`.
- Rust unit tests only - Inline tests exist in `src-tauri/src/crypto.rs` and `src-tauri/src/migration.rs`.
- Vite `6.x` - Dev server, HMR, aliasing, and production bundling in `vite.config.js`.
- `@sveltejs/vite-plugin-svelte` `5.x` with `vitePreprocess()` - Svelte compilation setup in `vite.config.js` and `svelte.config.js`.
- TypeScript compiler in strict bundler mode - Configured in `tsconfig.json` and `tsconfig.node.json`.
- ESLint `9.x` with TypeScript and Svelte support - Configured in `eslint.config.js`.
- Prettier `3.x` with `prettier-plugin-svelte` - Configured in `.prettierrc`.
## Key Dependencies
- `tauri` `2.9` - Core desktop shell, tray, window, and command registration in `src-tauri/src/main.rs`.
- `svelte` `5.37.0` - UI rendering and stateful frontend components across `src/routes/` and `src/lib/components/`.
- `rusqlite` `0.32` with `bundled` SQLite - Embedded persistence layer in `src-tauri/src/storage.rs`.
- `arboard` `3.4` and `clipboard-master` `4.0.0-beta.6` - Clipboard read/write plus event-driven monitoring in `src-tauri/src/clipboard.rs` and `src-tauri/src/commands.rs`.
- `ring` `0.17` - AES-256-GCM encryption for stored clipboard payloads in `src-tauri/src/crypto.rs` and key bootstrapping in `src-tauri/src/main.rs`.
- `@tauri-apps/api` `2.0.0` - Frontend invoke and event bridge used by `src/lib/stores/clipboard.svelte.ts` and `src/routes/settings/+page.svelte`.
- `tauri-plugin-updater` `2.1` - Update checks and in-app installs in `src-tauri/src/commands.rs`.
- `tauri-plugin-store` `2` - Persistent settings storage in `src-tauri/src/settings.rs`.
- `tauri-plugin-global-shortcut`, `tauri-plugin-notification`, `tauri-plugin-autostart` - OS integrations wired in `src-tauri/src/main.rs` and `src-tauri/src/commands.rs`.
- `lucide-svelte` `0.554.0` - Shared icon set throughout `src/lib/components/` and `src/routes/`.
## Configuration
- No `.env`, `.env.local`, or `.env.example` files were detected at repo root.
- Frontend build/runtime behavior reads `TAURI_DEV_HOST` and `TAURI_DEBUG` in `vite.config.js`.
- Desktop update trust is configured with an embedded updater public key and GitHub endpoint in `src-tauri/tauri.conf.json`.
- Release signing secrets are expected only in GitHub Actions: `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` in `.github/workflows/release.yml`.
- Frontend build configuration: `vite.config.js`, `svelte.config.js`, `tsconfig.json`, `tsconfig.node.json`.
- Native desktop configuration: `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/build.rs`, `src-tauri/Cargo.toml`.
- Code quality configuration: `eslint.config.js`, `.prettierrc`.
## Platform Requirements
- Desktop OS with Tauri/WebView support. `README.md` lists Windows 10+, macOS 10.13+, and Linux.
- Node.js `18+` or Bun plus Rust `1.82+` per `README.md`.
- Linux release builds install `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, and `patchelf` in `.github/workflows/release.yml`.
- Packaged desktop binaries for macOS, Windows, and Linux via Tauri bundling in `src-tauri/tauri.conf.json`.
- Auto-update artifacts are enabled with `"createUpdaterArtifacts": true` in `src-tauri/tauri.conf.json`.
- GitHub Releases is the current distribution target, created by `.github/workflows/release.yml`.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Frontend route files follow Svelte conventions such as `src/routes/+page.svelte` and `src/routes/settings/+page.svelte`.
- Reusable Svelte components use PascalCase filenames under `src/lib/components/`, `src/lib/components/settings/`, and `src/lib/components/ui/` such as `ClipboardItem.svelte`, `SearchBar.svelte`, and `Button.svelte`.
- Reactive TypeScript modules that use Svelte 5 runes are commonly suffixed with `.svelte.ts`, for example `src/lib/stores/clipboard.svelte.ts` and `src/lib/i18n/index.svelte.ts`.
- Backend Rust modules use snake_case filenames under `src-tauri/src/`, including `clipboard.rs`, `settings.rs`, `storage.rs`, and `commands.rs`.
- Frontend functions use camelCase and are generally verb-first, such as `loadSettings`, `saveSettings`, `changeDataLocation`, `clearHistory`, and `clearSearch`.
- DOM and keyboard handlers typically use a `handle*` prefix, for example `handleInput`, `handleKeyDown`, and `handleCopy`.
- Async functions are named for the action they perform rather than with a dedicated async prefix.
- Tauri command names are snake_case and match the string literals used with `invoke()`, such as `get_clipboard_history`, `update_settings`, and `migrate_data_location`.
- Frontend locals and reactive state use camelCase.
- Boolean state usually uses `is*`, `show*`, `has*`, or similar prefixes, such as `isLoading`, `showPinned`, `checkingUpdate`, and `installingUpdate`.
- Constants use UPPER_SNAKE_CASE in both TS and Rust, including `SEARCH_DEBOUNCE_MS`, `TOAST_DURATION_MS`, `TRAY_ICON_SIZE`, and `ICON_CACHE_SIZE`.
- Rust struct fields stay snake_case internally, then cross the IPC boundary through Serde camelCase renaming in `src-tauri/src/settings.rs` and `src-tauri/src/storage.rs`.
- TypeScript interfaces and aliases use PascalCase without an `I` prefix, for example `ClipItem`, `Settings`, `UpdateInfo`, `Theme`, and `SettingsTab`.
- Rust structs and enums also use PascalCase, such as `ClipStorage`, `ClipboardMonitor`, `SettingsManager`, and `ContentType`.
- Rust enum variants are PascalCase as well, for example `Text`, `Image`, `File`, `Html`, and `Rtf`.
## Code Style
- Frontend formatting is defined by `.prettierrc`: 2-space indentation, single quotes, semicolons required, trailing commas `es5`, and `printWidth` 100.
- Shared TS and UI primitive files generally follow that style, including `src/main.ts`, `src/lib/stores/clipboard.svelte.ts`, `src/lib/components/SearchBar.svelte`, and `src/lib/components/ui/Button.svelte`.
- Several settings-related Svelte files do not currently match the configured formatter. Files such as `src/routes/settings/+page.svelte`, `src/lib/components/settings/GeneralSettings.svelte`, and `src/lib/components/ui/MarkdownContent.svelte` use 4-space indentation and double quotes. Treat this as repository inconsistency, not a second official style.
- Rust follows standard rustfmt-shaped layout: 4-space indentation, trailing commas where helpful, grouped `use` statements, and explicit early-return error checks.
- ESLint is configured in `eslint.config.js` for `**/*.ts` and `**/*.svelte`.
- The repo disables base `no-unused-vars` and uses `@typescript-eslint/no-unused-vars` as a warning, with `_`-prefixed args ignored.
- `tsconfig.json` adds strictness with `strict`, `noUnusedLocals`, `noUnusedParameters`, and `noFallthroughCasesInSwitch`.
- Available frontend quality commands are `bun run lint`, `bun run check`, and `bun run format`.
- No Rust lint or formatting command is wired into root package scripts.
## Import Organization
- Frontend files usually separate external imports from app-local imports with a blank line.
- Sorting is manual rather than auto-enforced.
- Large Svelte files are grouped by concern more often than alphabetically.
- Rust `use` blocks are grouped by std, crates, and internal modules, but exact ordering is flexible.
- `$lib/*` maps to `src/lib/*` in both `tsconfig.json` and `vite.config.js`.
- No additional frontend aliases are defined.
- Rust modules use `crate::...` for internal references.
## Error Handling
- Frontend UI actions wrap Tauri and plugin calls in `try/catch`, log failures to `console.error`, and then update local message state or toast state. See `src/routes/settings/+page.svelte`, `src/lib/components/settings/StorageSettings.svelte`, and `src/lib/stores/clipboard.svelte.ts`.
- Expected UI failures are usually surfaced as inline text or toasts rather than rethrown.
- Some UI handlers intentionally swallow an error after a lower layer already handled user feedback, such as `handleCopy()` in `src/lib/components/ClipboardItem.svelte`.
- Browser-native `confirm(...)` is used for destructive actions in `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`, and `src/lib/components/settings/ClipboardSettings.svelte`. One direct `alert(...)` also exists in `src/lib/components/settings/ClipboardSettings.svelte`.
- Frontend code rarely creates custom error objects. It usually converts unknown failures with `err instanceof Error ? err.message : String(err)`.
- Tauri command boundaries consistently return `Result<..., String>` in `src-tauri/src/commands.rs`.
- Rust service code usually converts lower-level errors with `map_err(|e| e.to_string())` or formatted string messages rather than domain-specific error enums.
- Startup and low-level invariants still use `unwrap(...)` and `expect(...)` in several places, including `src-tauri/src/main.rs`, `src-tauri/src/crypto.rs`, `src-tauri/src/settings.rs`, and `src-tauri/src/tray.rs`.
- `src-tauri/src/storage.rs` has one clear resilience pattern: decryption failures are logged and skipped so list queries can continue.
## Logging
- Frontend logging uses raw `console.log`, `console.warn`, and `console.error`.
- Backend logging uses `env_logger` with the `log` crate, initialized in `src-tauri/src/main.rs`.
- Frontend logs are concentrated in `src/lib/stores/clipboard.svelte.ts` and often include pseudo-level tags inside the message string, such as `[INFO]`, `[SUCCESS]`, `[DEBUG]`, and `[ERROR]`.
- Backend logs use `log::debug!`, `log::info!`, `log::warn!`, and `log::error!`, often with emoji markers for operational events.
- Logging happens mainly at IO boundaries, IPC boundaries, clipboard events, storage operations, migrations, and tray updates.
- There is no centralized frontend logger abstraction or visible production suppression of `console.*` calls.
## Comments
- Comments are used to explain intent, platform behavior, or framework-specific reasoning rather than obvious code mechanics.
- The repository mixes English and Chinese comments. Match the language already used in the file you are editing.
- Many presentational Svelte components rely on readable markup and include few comments unless a workaround or unusual behavior needs explanation.
- Short doc comments are used selectively for shared types in `src/lib/types.ts`.
- General function-level JSDoc or TSDoc is not an established frontend convention.
- Rust doc comments are also sparse outside obvious module and function names.
- No active `TODO`, `FIXME`, or `HACK` markers were found in `src/` or `src-tauri/src/`.
- Follow-up work appears to be tracked outside inline source comments.
## Function Design
- Shared store and backend orchestration methods can be medium-to-large when they coordinate framework callbacks or IO, for example `ClipboardStore.initialize()`, `update_settings(...)`, and `ClipboardMonitor::start()`.
- Smaller UI helpers stay compact and single-purpose, such as `clearSearch()`, `formatShortcut()`, and `formatTime()`.
- Frontend functions usually take zero to two positional arguments.
- Component inputs are strongly typed through `$props()` and often use `$bindable()` for two-way settings state.
- Rust and TS both prefer object or struct payloads for richer state, such as `Settings`, `ClipItem`, and Tauri `invoke()` payload objects.
- Early returns are common for guard clauses, such as `if (!query)`, `if (!item) return`, and `if (!currentDataPath) return`.
- Frontend imperative methods typically mutate local state and return `void` or `Promise<void>`.
- Rust command and service APIs return `Result` and let callers decide whether to log, emit events, or rebuild tray state.
## Module Design
- Frontend shared modules prefer named exports, for example `clipboardStore`, `themeStore`, `router`, `toastStore`, and `i18n`.
- Type re-exports are used for convenience in rune store files, such as `src/lib/stores/clipboard.svelte.ts` and `src/lib/stores/theme.svelte.ts`.
- Svelte components are imported directly by file rather than through barrel exports.
- `src/lib/i18n/index.ts` is a small re-export entry point.
- Broad barrel-file usage is not a codebase-wide pattern.
- Rust module boundaries are explicit through `mod ...;` in `src-tauri/src/main.rs` and `use crate::...` imports.
## Established Architectural Patterns
- Frontend state is primarily organized as class-based Svelte 5 rune stores with singleton exports in `src/lib/stores/*.svelte.ts`.
- Page components orchestrate data loading and hand state into focused subcomponents via bindable props.
- Backend Tauri commands in `src-tauri/src/commands.rs` are thin IPC adapters over storage, settings, tray, clipboard, and migration modules.
- JSON crossing the Tauri boundary intentionally uses camelCase names even when Rust internals remain snake_case.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Single desktop application process with no network API tier
- Event-driven clipboard capture and tray interactions managed in Rust
- Frontend uses Svelte 5 runes and local stores instead of a full client router/state library
- Tauri invoke/event IPC is the only boundary between UI and backend
- SQLite-backed persistence with optional encryption and OS-level integrations
## Layers
- Purpose: Render the desktop UI, switch between the home and settings views, and compose page-level interactions.
- Contains: `src/main.ts`, `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`
- Depends on: Frontend stores, UI components, i18n, Tauri invoke APIs
- Used by: The mounted webview inside the Tauri window
- Purpose: Hold client-side state, listen to backend events, and expose reusable UI building blocks.
- Contains: `src/lib/stores/clipboard.svelte.ts`, `src/lib/stores/router.svelte.ts`, `src/lib/stores/theme.svelte.ts`, `src/lib/stores/toast.svelte.ts`, `src/lib/components/**`, `src/lib/i18n/index.svelte.ts`
- Depends on: Tauri `invoke`/`listen`, shared frontend types, browser APIs like `localStorage`
- Used by: Route components and settings subsections
- Purpose: Translate frontend actions into Rust operations and return serializable results back to the UI.
- Contains: `src-tauri/src/commands.rs`, the `invoke_handler` registration in `src-tauri/src/main.rs`
- Depends on: Shared `AppState`, storage/settings/tray/migration modules, Tauri plugin APIs
- Used by: Frontend pages/components via `invoke("...")`
- Purpose: Own clipboard monitoring, tray behavior, shortcut registration, update flow, settings persistence, data migration, and encryption.
- Contains: `src-tauri/src/clipboard.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/settings.rs`, `src-tauri/src/migration.rs`, `src-tauri/src/crypto.rs`
- Depends on: OS clipboard APIs, Tauri plugins, image processing, filesystem access, shared app state
- Used by: Tauri setup lifecycle and command handlers
- Purpose: Store and query clipboard history, enforce deduplication/history limits, and map backend items into frontend-safe payloads.
- Contains: `src-tauri/src/storage.rs`
- Depends on: SQLite (`rusqlite`), hashing, optional crypto, serde serialization
- Used by: Clipboard monitor, command handlers, tray menu builder
## Data Flow
- Frontend: In-memory Svelte rune state inside singleton stores plus `localStorage` for theme/locale.
- Backend: Shared `AppState` with `Arc<Mutex<...>>` around storage and mutable runtime services.
- Persistent: SQLite database file and a Tauri plugin store file (`settings.json`) under the effective app data directory.
## Key Abstractions
- Purpose: Central runtime container shared across Tauri commands and setup callbacks.
- Examples: `storage`, `monitor`, `settings`, `last_copied_by_us`, `icon_cache` in `src-tauri/src/main.rs`
- Pattern: Shared application context with `Arc` and `Mutex`
- Purpose: Frontend facade for clipboard history loading, event subscription, search, pin/delete/copy actions, and optimistic UI updates.
- Examples: `src/lib/stores/clipboard.svelte.ts`
- Pattern: Singleton client store with derived state
- Purpose: Separate storage representation from frontend transport representation.
- Examples: `ClipStorage`, `ClipItem`, `FrontendClipItem` in `src-tauri/src/storage.rs`
- Pattern: Repository-like persistence module with DTO conversion
- Purpose: Organize main screens using SvelteKit-style filenames, even though navigation is handled by a local store instead of the framework router.
- Examples: `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`, `src/lib/stores/router.svelte.ts`
- Pattern: Component-level view switching
- Purpose: Break the settings surface into focused subsections while keeping save/load orchestration in the parent page.
- Examples: `src/lib/components/settings/GeneralSettings.svelte`, `AppearanceSettings.svelte`, `StorageSettings.svelte`, `AboutSection.svelte`
- Pattern: Parent-owned state with bound child editors
## Entry Points
- Location: `src/main.ts`
- Triggers: Webview load inside the Tauri window
- Responsibilities: Load global CSS and mount the root Svelte view
- Location: `src/routes/+page.svelte`
- Triggers: Imported directly by `src/main.ts`
- Responsibilities: Render the home screen, switch to settings, attach permission/toast components, and apply the active theme class to `document.documentElement`
- Location: `src-tauri/src/main.rs`
- Triggers: Desktop app launch
- Responsibilities: Configure plugins, load settings/data paths, initialize crypto/storage, build the tray, register window behavior and global shortcuts, and expose Tauri commands
- Location: `src-tauri/src/clipboard.rs`
- Triggers: Started during Tauri setup
- Responsibilities: Watch the OS clipboard, skip self-copied entries, preprocess images, persist items, and emit UI updates
## Error Handling
- Command handlers wrap blocking storage work in `spawn_blocking` and stringify errors before returning them to the UI.
- `safe_lock()` in `src-tauri/src/main.rs` recovers from poisoned mutexes instead of crashing immediately.
- Clipboard monitoring falls back from event-driven `clipboard-master` to a polling loop when needed.
- Frontend stores/components usually catch invoke failures locally and either log them, set page-level status text, or show a toast.
- There is no centralized frontend error boundary; most errors are handled at the component/store call site.
## Cross-Cutting Concerns
- Most storage mutations call `update_tray_menu()` from `src-tauri/src/tray.rs`, so the menu is treated as a live secondary view of the same clipboard state.
- `last_copied_by_us` in `AppState` marks text copied by the app so the monitor can avoid re-capturing self-generated clipboard writes.
- Frontend text lives in `src/lib/i18n/index.svelte.ts`; tray labels are rebuilt from backend settings locale via `TrayI18n` in `src-tauri/src/tray.rs`.
- Visual tokens are centralized in `src/app.css`; `themeStore` controls root classes and the main page applies them globally.
- Clipboard content is encrypted before hitting SQLite when crypto is available, with the key stored in `.clipman.key` under the app data directory.
- Tauri capabilities in `src-tauri/capabilities/default.json` constrain which window/event/plugin operations are permitted.
- The backend intentionally shares mutable runtime state through mutexes because tray callbacks, clipboard threads, and command handlers all touch common resources.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, or `.github/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
