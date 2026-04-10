# Architecture

**Analysis Date:** 2026-04-11

## Pattern Overview

**Overall:** Hybrid desktop monolith built as a Tauri native shell with a Svelte 5 frontend and an in-process Rust backend.

**Key Characteristics:**
- Single desktop application process with no network API tier
- Event-driven clipboard capture and tray interactions managed in Rust
- Frontend uses Svelte 5 runes and local stores instead of a full client router/state library
- Tauri invoke/event IPC is the only boundary between UI and backend
- SQLite-backed persistence with optional encryption and OS-level integrations

## Layers

**Frontend Shell and Views:**
- Purpose: Render the desktop UI, switch between the home and settings views, and compose page-level interactions.
- Contains: `src/main.ts`, `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`
- Depends on: Frontend stores, UI components, i18n, Tauri invoke APIs
- Used by: The mounted webview inside the Tauri window

**Frontend State and UI Composition:**
- Purpose: Hold client-side state, listen to backend events, and expose reusable UI building blocks.
- Contains: `src/lib/stores/clipboard.svelte.ts`, `src/lib/stores/router.svelte.ts`, `src/lib/stores/theme.svelte.ts`, `src/lib/stores/toast.svelte.ts`, `src/lib/components/**`, `src/lib/i18n/index.svelte.ts`
- Depends on: Tauri `invoke`/`listen`, shared frontend types, browser APIs like `localStorage`
- Used by: Route components and settings subsections

**IPC Command Boundary:**
- Purpose: Translate frontend actions into Rust operations and return serializable results back to the UI.
- Contains: `src-tauri/src/commands.rs`, the `invoke_handler` registration in `src-tauri/src/main.rs`
- Depends on: Shared `AppState`, storage/settings/tray/migration modules, Tauri plugin APIs
- Used by: Frontend pages/components via `invoke("...")`

**Backend Services and System Integration:**
- Purpose: Own clipboard monitoring, tray behavior, shortcut registration, update flow, settings persistence, data migration, and encryption.
- Contains: `src-tauri/src/clipboard.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/settings.rs`, `src-tauri/src/migration.rs`, `src-tauri/src/crypto.rs`
- Depends on: OS clipboard APIs, Tauri plugins, image processing, filesystem access, shared app state
- Used by: Tauri setup lifecycle and command handlers

**Persistence Layer:**
- Purpose: Store and query clipboard history, enforce deduplication/history limits, and map backend items into frontend-safe payloads.
- Contains: `src-tauri/src/storage.rs`
- Depends on: SQLite (`rusqlite`), hashing, optional crypto, serde serialization
- Used by: Clipboard monitor, command handlers, tray menu builder

## Data Flow

**Clipboard Capture Pipeline:**

1. App startup in `src-tauri/src/main.rs` builds `AppState`, loads settings, opens the database, and starts `ClipboardMonitor`.
2. `ClipboardMonitor` in `src-tauri/src/clipboard.rs` receives OS clipboard events through `clipboard-master`, with polling as fallback.
3. New clipboard content is normalized into a `ClipItem` and written through `ClipStorage::insert()` in `src-tauri/src/storage.rs`.
4. Storage deduplicates by `content_hash`, encrypts payloads if crypto is enabled, and trims non-pinned history.
5. Rust emits `clipboard-changed` to the frontend and refreshes the tray menu.
6. `clipboardStore` in `src/lib/stores/clipboard.svelte.ts` listens for the event and updates its in-memory list, which re-renders `src/routes/+page.svelte`.

**User Action Pipeline (search, pin, delete, clear, copy):**

1. A Svelte component or store calls `invoke()` from the frontend.
2. A command in `src-tauri/src/commands.rs` runs the operation, often via `spawn_blocking` to keep SQLite work off the async runtime.
3. The command mutates storage and/or settings, refreshes the tray when needed, and may emit follow-up events such as `history-cleared` or `clipboard-changed`.
4. The frontend store reloads or patches local state and updates the visible list.

**Settings and Localization Pipeline:**

1. `src/routes/settings/+page.svelte` loads settings and current data path on mount.
2. Settings section components mutate a shared `settings` object passed down from the page.
3. Saving calls `update_settings`, which persists `settings.json`, re-registers global shortcuts, updates autostart, and rebuilds the tray menu when locale/tray text changes.
4. Theme stays frontend-local through `themeStore`, while locale is dual-written: local state in `src/lib/i18n/index.svelte.ts` and backend settings for tray translations.

**State Management:**
- Frontend: In-memory Svelte rune state inside singleton stores plus `localStorage` for theme/locale.
- Backend: Shared `AppState` with `Arc<Mutex<...>>` around storage and mutable runtime services.
- Persistent: SQLite database file and a Tauri plugin store file (`settings.json`) under the effective app data directory.

## Key Abstractions

**AppState:**
- Purpose: Central runtime container shared across Tauri commands and setup callbacks.
- Examples: `storage`, `monitor`, `settings`, `last_copied_by_us`, `icon_cache` in `src-tauri/src/main.rs`
- Pattern: Shared application context with `Arc` and `Mutex`

**ClipboardStore:**
- Purpose: Frontend facade for clipboard history loading, event subscription, search, pin/delete/copy actions, and optimistic UI updates.
- Examples: `src/lib/stores/clipboard.svelte.ts`
- Pattern: Singleton client store with derived state

**ClipStorage / ClipItem / FrontendClipItem:**
- Purpose: Separate storage representation from frontend transport representation.
- Examples: `ClipStorage`, `ClipItem`, `FrontendClipItem` in `src-tauri/src/storage.rs`
- Pattern: Repository-like persistence module with DTO conversion

**Route-as-Component Pages:**
- Purpose: Organize main screens using SvelteKit-style filenames, even though navigation is handled by a local store instead of the framework router.
- Examples: `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`, `src/lib/stores/router.svelte.ts`
- Pattern: Component-level view switching

**Settings Section Components:**
- Purpose: Break the settings surface into focused subsections while keeping save/load orchestration in the parent page.
- Examples: `src/lib/components/settings/GeneralSettings.svelte`, `AppearanceSettings.svelte`, `StorageSettings.svelte`, `AboutSection.svelte`
- Pattern: Parent-owned state with bound child editors

## Entry Points

**Frontend Bootstrap:**
- Location: `src/main.ts`
- Triggers: Webview load inside the Tauri window
- Responsibilities: Load global CSS and mount the root Svelte view

**Main UI Shell:**
- Location: `src/routes/+page.svelte`
- Triggers: Imported directly by `src/main.ts`
- Responsibilities: Render the home screen, switch to settings, attach permission/toast components, and apply the active theme class to `document.documentElement`

**Tauri Runtime Startup:**
- Location: `src-tauri/src/main.rs`
- Triggers: Desktop app launch
- Responsibilities: Configure plugins, load settings/data paths, initialize crypto/storage, build the tray, register window behavior and global shortcuts, and expose Tauri commands

**Clipboard Monitoring Thread:**
- Location: `src-tauri/src/clipboard.rs`
- Triggers: Started during Tauri setup
- Responsibilities: Watch the OS clipboard, skip self-copied entries, preprocess images, persist items, and emit UI updates

## Error Handling

**Strategy:** Rust modules return `Result<_, String>` across the Tauri boundary, log heavily on failure, and let the frontend show inline messages or toasts where appropriate.

**Patterns:**
- Command handlers wrap blocking storage work in `spawn_blocking` and stringify errors before returning them to the UI.
- `safe_lock()` in `src-tauri/src/main.rs` recovers from poisoned mutexes instead of crashing immediately.
- Clipboard monitoring falls back from event-driven `clipboard-master` to a polling loop when needed.
- Frontend stores/components usually catch invoke failures locally and either log them, set page-level status text, or show a toast.
- There is no centralized frontend error boundary; most errors are handled at the component/store call site.

## Cross-Cutting Concerns

**Tray Synchronization:**
- Most storage mutations call `update_tray_menu()` from `src-tauri/src/tray.rs`, so the menu is treated as a live secondary view of the same clipboard state.

**Clipboard Loop Prevention:**
- `last_copied_by_us` in `AppState` marks text copied by the app so the monitor can avoid re-capturing self-generated clipboard writes.

**Localization:**
- Frontend text lives in `src/lib/i18n/index.svelte.ts`; tray labels are rebuilt from backend settings locale via `TrayI18n` in `src-tauri/src/tray.rs`.

**Theming:**
- Visual tokens are centralized in `src/app.css`; `themeStore` controls root classes and the main page applies them globally.

**Security and Persistence:**
- Clipboard content is encrypted before hitting SQLite when crypto is available, with the key stored in `.clipman.key` under the app data directory.
- Tauri capabilities in `src-tauri/capabilities/default.json` constrain which window/event/plugin operations are permitted.

**Concurrency:**
- The backend intentionally shares mutable runtime state through mutexes because tray callbacks, clipboard threads, and command handlers all touch common resources.

---

*Architecture analysis: 2026-04-11*
*Update when major patterns change*
