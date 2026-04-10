# Coding Conventions

**Analysis Date:** 2026-04-11

## Naming Patterns

**Files:**
- Frontend route files follow Svelte conventions such as `src/routes/+page.svelte` and `src/routes/settings/+page.svelte`.
- Reusable Svelte components use PascalCase filenames under `src/lib/components/`, `src/lib/components/settings/`, and `src/lib/components/ui/` such as `ClipboardItem.svelte`, `SearchBar.svelte`, and `Button.svelte`.
- Reactive TypeScript modules that use Svelte 5 runes are commonly suffixed with `.svelte.ts`, for example `src/lib/stores/clipboard.svelte.ts` and `src/lib/i18n/index.svelte.ts`.
- Backend Rust modules use snake_case filenames under `src-tauri/src/`, including `clipboard.rs`, `settings.rs`, `storage.rs`, and `commands.rs`.

**Functions:**
- Frontend functions use camelCase and are generally verb-first, such as `loadSettings`, `saveSettings`, `changeDataLocation`, `clearHistory`, and `clearSearch`.
- DOM and keyboard handlers typically use a `handle*` prefix, for example `handleInput`, `handleKeyDown`, and `handleCopy`.
- Async functions are named for the action they perform rather than with a dedicated async prefix.
- Tauri command names are snake_case and match the string literals used with `invoke()`, such as `get_clipboard_history`, `update_settings`, and `migrate_data_location`.

**Variables:**
- Frontend locals and reactive state use camelCase.
- Boolean state usually uses `is*`, `show*`, `has*`, or similar prefixes, such as `isLoading`, `showPinned`, `checkingUpdate`, and `installingUpdate`.
- Constants use UPPER_SNAKE_CASE in both TS and Rust, including `SEARCH_DEBOUNCE_MS`, `TOAST_DURATION_MS`, `TRAY_ICON_SIZE`, and `ICON_CACHE_SIZE`.
- Rust struct fields stay snake_case internally, then cross the IPC boundary through Serde camelCase renaming in `src-tauri/src/settings.rs` and `src-tauri/src/storage.rs`.

**Types:**
- TypeScript interfaces and aliases use PascalCase without an `I` prefix, for example `ClipItem`, `Settings`, `UpdateInfo`, `Theme`, and `SettingsTab`.
- Rust structs and enums also use PascalCase, such as `ClipStorage`, `ClipboardMonitor`, `SettingsManager`, and `ContentType`.
- Rust enum variants are PascalCase as well, for example `Text`, `Image`, `File`, `Html`, and `Rtf`.

## Code Style

**Formatting:**
- Frontend formatting is defined by `.prettierrc`: 2-space indentation, single quotes, semicolons required, trailing commas `es5`, and `printWidth` 100.
- Shared TS and UI primitive files generally follow that style, including `src/main.ts`, `src/lib/stores/clipboard.svelte.ts`, `src/lib/components/SearchBar.svelte`, and `src/lib/components/ui/Button.svelte`.
- Several settings-related Svelte files do not currently match the configured formatter. Files such as `src/routes/settings/+page.svelte`, `src/lib/components/settings/GeneralSettings.svelte`, and `src/lib/components/ui/MarkdownContent.svelte` use 4-space indentation and double quotes. Treat this as repository inconsistency, not a second official style.
- Rust follows standard rustfmt-shaped layout: 4-space indentation, trailing commas where helpful, grouped `use` statements, and explicit early-return error checks.

**Linting:**
- ESLint is configured in `eslint.config.js` for `**/*.ts` and `**/*.svelte`.
- The repo disables base `no-unused-vars` and uses `@typescript-eslint/no-unused-vars` as a warning, with `_`-prefixed args ignored.
- `tsconfig.json` adds strictness with `strict`, `noUnusedLocals`, `noUnusedParameters`, and `noFallthroughCasesInSwitch`.
- Available frontend quality commands are `bun run lint`, `bun run check`, and `bun run format`.
- No Rust lint or formatting command is wired into root package scripts.

## Import Organization

**Order:**
1. External packages and framework imports such as `svelte`, `@tauri-apps/api/*`, and `lucide-svelte`
2. Internal aliased imports from `$lib/*`
3. Relative imports such as `./toast.svelte` or `./settings/+page.svelte`
4. Type-only imports are used selectively, but not enforced as a separate final group

**Grouping:**
- Frontend files usually separate external imports from app-local imports with a blank line.
- Sorting is manual rather than auto-enforced.
- Large Svelte files are grouped by concern more often than alphabetically.
- Rust `use` blocks are grouped by std, crates, and internal modules, but exact ordering is flexible.

**Path Aliases:**
- `$lib/*` maps to `src/lib/*` in both `tsconfig.json` and `vite.config.js`.
- No additional frontend aliases are defined.
- Rust modules use `crate::...` for internal references.

## Error Handling

**Patterns:**
- Frontend UI actions wrap Tauri and plugin calls in `try/catch`, log failures to `console.error`, and then update local message state or toast state. See `src/routes/settings/+page.svelte`, `src/lib/components/settings/StorageSettings.svelte`, and `src/lib/stores/clipboard.svelte.ts`.
- Expected UI failures are usually surfaced as inline text or toasts rather than rethrown.
- Some UI handlers intentionally swallow an error after a lower layer already handled user feedback, such as `handleCopy()` in `src/lib/components/ClipboardItem.svelte`.
- Browser-native `confirm(...)` is used for destructive actions in `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`, and `src/lib/components/settings/ClipboardSettings.svelte`. One direct `alert(...)` also exists in `src/lib/components/settings/ClipboardSettings.svelte`.

**Error Types:**
- Frontend code rarely creates custom error objects. It usually converts unknown failures with `err instanceof Error ? err.message : String(err)`.
- Tauri command boundaries consistently return `Result<..., String>` in `src-tauri/src/commands.rs`.
- Rust service code usually converts lower-level errors with `map_err(|e| e.to_string())` or formatted string messages rather than domain-specific error enums.
- Startup and low-level invariants still use `unwrap(...)` and `expect(...)` in several places, including `src-tauri/src/main.rs`, `src-tauri/src/crypto.rs`, `src-tauri/src/settings.rs`, and `src-tauri/src/tray.rs`.
- `src-tauri/src/storage.rs` has one clear resilience pattern: decryption failures are logged and skipped so list queries can continue.

## Logging

**Framework:**
- Frontend logging uses raw `console.log`, `console.warn`, and `console.error`.
- Backend logging uses `env_logger` with the `log` crate, initialized in `src-tauri/src/main.rs`.

**Patterns:**
- Frontend logs are concentrated in `src/lib/stores/clipboard.svelte.ts` and often include pseudo-level tags inside the message string, such as `[INFO]`, `[SUCCESS]`, `[DEBUG]`, and `[ERROR]`.
- Backend logs use `log::debug!`, `log::info!`, `log::warn!`, and `log::error!`, often with emoji markers for operational events.
- Logging happens mainly at IO boundaries, IPC boundaries, clipboard events, storage operations, migrations, and tray updates.
- There is no centralized frontend logger abstraction or visible production suppression of `console.*` calls.

## Comments

**When to Comment:**
- Comments are used to explain intent, platform behavior, or framework-specific reasoning rather than obvious code mechanics.
- The repository mixes English and Chinese comments. Match the language already used in the file you are editing.
- Many presentational Svelte components rely on readable markup and include few comments unless a workaround or unusual behavior needs explanation.

**JSDoc/TSDoc:**
- Short doc comments are used selectively for shared types in `src/lib/types.ts`.
- General function-level JSDoc or TSDoc is not an established frontend convention.
- Rust doc comments are also sparse outside obvious module and function names.

**TODO Comments:**
- No active `TODO`, `FIXME`, or `HACK` markers were found in `src/` or `src-tauri/src/`.
- Follow-up work appears to be tracked outside inline source comments.

## Function Design

**Size:**
- Shared store and backend orchestration methods can be medium-to-large when they coordinate framework callbacks or IO, for example `ClipboardStore.initialize()`, `update_settings(...)`, and `ClipboardMonitor::start()`.
- Smaller UI helpers stay compact and single-purpose, such as `clearSearch()`, `formatShortcut()`, and `formatTime()`.

**Parameters:**
- Frontend functions usually take zero to two positional arguments.
- Component inputs are strongly typed through `$props()` and often use `$bindable()` for two-way settings state.
- Rust and TS both prefer object or struct payloads for richer state, such as `Settings`, `ClipItem`, and Tauri `invoke()` payload objects.

**Return Values:**
- Early returns are common for guard clauses, such as `if (!query)`, `if (!item) return`, and `if (!currentDataPath) return`.
- Frontend imperative methods typically mutate local state and return `void` or `Promise<void>`.
- Rust command and service APIs return `Result` and let callers decide whether to log, emit events, or rebuild tray state.

## Module Design

**Exports:**
- Frontend shared modules prefer named exports, for example `clipboardStore`, `themeStore`, `router`, `toastStore`, and `i18n`.
- Type re-exports are used for convenience in rune store files, such as `src/lib/stores/clipboard.svelte.ts` and `src/lib/stores/theme.svelte.ts`.
- Svelte components are imported directly by file rather than through barrel exports.

**Barrel Files:**
- `src/lib/i18n/index.ts` is a small re-export entry point.
- Broad barrel-file usage is not a codebase-wide pattern.
- Rust module boundaries are explicit through `mod ...;` in `src-tauri/src/main.rs` and `use crate::...` imports.

## Established Architectural Patterns

- Frontend state is primarily organized as class-based Svelte 5 rune stores with singleton exports in `src/lib/stores/*.svelte.ts`.
- Page components orchestrate data loading and hand state into focused subcomponents via bindable props.
- Backend Tauri commands in `src-tauri/src/commands.rs` are thin IPC adapters over storage, settings, tray, clipboard, and migration modules.
- JSON crossing the Tauri boundary intentionally uses camelCase names even when Rust internals remain snake_case.

---
*Convention analysis: 2026-04-11*
*Update when formatter use, logging policy, or project-wide linting practices change*
