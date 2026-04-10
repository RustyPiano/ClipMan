# Codebase Structure

**Analysis Date:** 2026-04-11

## Directory Layout

```text
ClipMan/
├── .codex/                 # Local Codex/GSD workflow assets and agent definitions
│   ├── agents/             # Specialized agent prompts/configs
│   ├── get-shit-done/      # Workflow engine, templates, references, scripts
│   └── skills/             # Skill entrypoints used by local workflows
├── .github/                # Release docs and CI workflow
│   └── workflows/          # GitHub Actions definitions
├── .planning/              # Generated planning and codebase map output
│   └── codebase/           # Architecture/stack/structure analysis docs
├── src/                    # Svelte frontend application
│   ├── lib/                # Shared frontend modules
│   │   ├── components/     # Reusable UI and feature components
│   │   ├── i18n/           # Frontend localization runtime
│   │   ├── stores/         # Client-side singleton state stores
│   │   └── types.ts        # Shared frontend types
│   ├── routes/             # View components named with SvelteKit conventions
│   ├── app.css             # Tailwind v4 theme tokens and global styles
│   └── main.ts             # Frontend bootstrap
├── src-tauri/              # Rust/Tauri desktop backend and bundle resources
│   ├── capabilities/       # Tauri capability permissions
│   ├── gen/                # Generated Tauri schemas
│   ├── icons/              # App and bundle icon assets
│   ├── src/                # Rust application modules
│   ├── Cargo.toml          # Rust dependency/config manifest
│   └── tauri.conf.json     # Tauri app/window/build config
├── README.md               # Chinese project overview and setup
├── README_EN.md            # English project overview and setup
├── package.json            # Frontend scripts and JS dependencies
├── vite.config.js          # Vite build/dev config and aliases
└── svelte.config.js        # Svelte preprocessing config
```

## Directory Purposes

**src/**
- Purpose: All webview-side application code.
- Contains: `*.svelte`, `*.ts`, `*.svelte.ts`, global CSS
- Key files: `src/main.ts`, `src/app.css`
- Subdirectories: `lib/` for shared modules, `routes/` for top-level views

**src/lib/components/**
- Purpose: Reusable presentation and feature widgets used by the home and settings views.
- Contains: Clipboard UI pieces plus nested `settings/` and `ui/` subfolders
- Key files: `src/lib/components/ClipboardItem.svelte`, `src/lib/components/SearchBar.svelte`, `src/lib/components/PermissionCheck.svelte`, `src/lib/components/Toast.svelte`
- Subdirectories: `settings/` for settings sections, `ui/` for low-level primitives

**src/lib/components/settings/**
- Purpose: Modular subsections of the settings page.
- Contains: One Svelte component per settings area
- Key files: `src/lib/components/settings/GeneralSettings.svelte`, `src/lib/components/settings/AppearanceSettings.svelte`, `src/lib/components/settings/TraySettings.svelte`, `src/lib/components/settings/StorageSettings.svelte`, `src/lib/components/settings/AboutSection.svelte`
- Subdirectories: None; flat feature-section layout

**src/lib/components/ui/**
- Purpose: Reusable styling primitives shared across feature components.
- Contains: `Button.svelte`, `Card.svelte`, `Input.svelte`, `MarkdownContent.svelte`
- Key files: `src/lib/components/ui/Button.svelte`, `src/lib/components/ui/Input.svelte`
- Subdirectories: None

**src/lib/stores/**
- Purpose: Long-lived frontend state containers and cross-view coordination.
- Contains: `clipboard.svelte.ts`, `router.svelte.ts`, `theme.svelte.ts`, `toast.svelte.ts`
- Key files: `src/lib/stores/clipboard.svelte.ts`, `src/lib/stores/router.svelte.ts`
- Subdirectories: None

**src/lib/i18n/**
- Purpose: Frontend localization definitions and runtime locale syncing.
- Contains: `index.svelte.ts` translations/runtime, `index.ts` re-export
- Key files: `src/lib/i18n/index.svelte.ts`
- Subdirectories: None

**src/routes/**
- Purpose: Page-level view components.
- Contains: `+page.svelte` files
- Key files: `src/routes/+page.svelte`, `src/routes/settings/+page.svelte`
- Subdirectories: `settings/` for the settings screen

**src-tauri/src/**
- Purpose: Native backend logic for persistence, OS integrations, and command handlers.
- Contains: Rust modules in a flat layout
- Key files: `src-tauri/src/main.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/storage.rs`, `src-tauri/src/clipboard.rs`, `src-tauri/src/tray.rs`
- Subdirectories: None; modules are split by concern rather than nested folders

**src-tauri/capabilities/**
- Purpose: Permission declarations for what the Tauri app is allowed to access.
- Contains: JSON capability manifests
- Key files: `src-tauri/capabilities/default.json`
- Subdirectories: None

**src-tauri/gen/**
- Purpose: Generated Tauri schemas and ACL metadata.
- Contains: JSON schema files under `schemas/`
- Key files: `src-tauri/gen/schemas/desktop-schema.json`, `src-tauri/gen/schemas/capabilities.json`
- Subdirectories: `schemas/`

**src-tauri/icons/**
- Purpose: Bundle and platform icon assets.
- Contains: PNG/ICO/ICNS assets plus Android and iOS icon folders
- Key files: `src-tauri/icons/icon.icns`, `src-tauri/icons/icon.ico`, `src-tauri/icons/icon.png`
- Subdirectories: `android/`, `ios/`

**.github/**
- Purpose: Release process documentation and automation.
- Contains: Markdown guides and GitHub Actions YAML
- Key files: `.github/workflows/release.yml`, `.github/RELEASE_GUIDE.md`
- Subdirectories: `workflows/`

**.codex/**
- Purpose: Local agent/workflow support files for the GSD/Codex setup in this workspace.
- Contains: Agent definitions, workflow templates, skill directories
- Key files: `.codex/agents/gsd-codebase-mapper.toml`, `.codex/get-shit-done/workflows/map-codebase.md`
- Subdirectories: `agents/`, `get-shit-done/`, `skills/`

**.planning/**
- Purpose: Workspace planning output rather than runtime app code.
- Contains: Generated planning artifacts such as this codebase map
- Key files: `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md`
- Subdirectories: `codebase/`

## Key File Locations

**Entry Points:**
- `src/main.ts` - Frontend bootstrap that mounts the root Svelte view
- `src/routes/+page.svelte` - Main application shell and home screen
- `src/routes/settings/+page.svelte` - Settings screen composed from subsection components
- `src-tauri/src/main.rs` - Desktop runtime startup, plugin registration, tray/window setup, and Tauri command registration

**Configuration:**
- `package.json` - JS scripts and frontend dependency manifest
- `vite.config.js` - Vite plugins, `$lib` alias, dev server/HMR settings
- `svelte.config.js` - Svelte preprocessing
- `tsconfig.json` / `tsconfig.node.json` - TypeScript compiler settings
- `eslint.config.js` - ESLint rules for frontend source
- `src-tauri/Cargo.toml` - Rust crate dependencies and release profile
- `src-tauri/tauri.conf.json` - Tauri window/build/updater settings
- `src-tauri/capabilities/default.json` - Allowed Tauri operations/plugins

**Core Logic:**
- `src/lib/stores/clipboard.svelte.ts` - Frontend clipboard state, event listeners, and invoke wrappers
- `src/lib/stores/router.svelte.ts` - Simple in-memory route switching
- `src/lib/i18n/index.svelte.ts` - Locale runtime and backend sync
- `src-tauri/src/commands.rs` - Tauri command surface used by the frontend
- `src-tauri/src/storage.rs` - SQLite persistence, dedupe, DTO conversion
- `src-tauri/src/clipboard.rs` - Clipboard monitoring and image preprocessing
- `src-tauri/src/tray.rs` - Tray menu rendering and icon caching
- `src-tauri/src/settings.rs` - Persistent app settings manager
- `src-tauri/src/migration.rs` - Data directory migration support
- `src-tauri/src/crypto.rs` - AES-GCM encryption/decryption helper

**Testing:**
- No dedicated `tests/` directory exists in the frontend
- `src-tauri/src/tray.rs` - Inline unit tests for tray i18n and truncation helpers
- `src-tauri/src/crypto.rs` - Inline unit test for encrypt/decrypt roundtrip
- `src-tauri/src/migration.rs` - Inline unit test for migration path validation

**Documentation:**
- `README.md` - Primary Chinese documentation
- `README_EN.md` - English documentation
- `.github/RELEASE_GUIDE.md` - Release process notes
- `release_notes_1.10.0.md` - Version-specific release notes
- `.planning/codebase/*.md` - Generated codebase reference docs

## Naming Conventions

**Files:**
- `PascalCase.svelte` for reusable UI components such as `ClipboardItem.svelte` and `StorageSettings.svelte`
- `+page.svelte` for page-level view files under `src/routes/`
- `*.svelte.ts` for stateful TypeScript modules using Svelte runes, such as `clipboard.svelte.ts`
- `snake_case.rs` for Rust modules such as `commands.rs`, `storage.rs`, `migration.rs`
- `UPPERCASE.md` for top-level/reference docs such as `README.md`, `LICENSE`, and `.planning/codebase/ARCHITECTURE.md`

**Directories:**
- Lowercase directory names throughout the app: `components/`, `stores/`, `routes/`, `capabilities/`
- Feature subdirectories under `components/` rather than many files at the top level: `settings/`, `ui/`
- Flat Rust module layout in `src-tauri/src/` instead of nested domain folders

**Special Patterns:**
- `src/routes/settings/+page.svelte` is imported like a component by `src/routes/+page.svelte`; navigation is local-store-driven, not framework-router-driven
- Settings section components are named by concern, usually `*Settings.svelte`, with `AboutSection.svelte` as the only non-`Settings` variant
- DTO and state names use camelCase at the serialization boundary because Rust structs are annotated with `#[serde(rename_all = "camelCase")]`

## Where to Add New Code

**New Clipboard/UI Feature:**
- Primary code: `src/lib/components/` and, if page-level, `src/routes/+page.svelte`
- State and backend calls: `src/lib/stores/clipboard.svelte.ts`
- Native/backend support: add or update a module in `src-tauri/src/` and expose it through `src-tauri/src/commands.rs`

**New Settings Section:**
- Implementation: `src/lib/components/settings/{Feature}Settings.svelte`
- Tab typing: update `src/lib/types.ts`
- Navigation wiring: update `src/lib/components/settings/Sidebar.svelte`
- Page composition/save flow: update `src/routes/settings/+page.svelte`

**New View/Screen:**
- View file: `src/routes/{name}/+page.svelte` or another page component under `src/routes/`
- Navigation state: update `src/lib/stores/router.svelte.ts`
- Shell integration: import/render from `src/routes/+page.svelte`

**New Tauri Command or Backend Capability:**
- Command definition: `src-tauri/src/commands.rs`
- Shared/runtime wiring: `src-tauri/src/main.rs`
- Domain logic: existing module in `src-tauri/src/` or a new `snake_case.rs` module
- Frontend call sites: `invoke()` usage in `src/lib/stores/` or `src/lib/components/`

**New Shared Frontend Primitive or Utility:**
- UI primitive: `src/lib/components/ui/`
- Shared types: `src/lib/types.ts`
- Theme tokens/global styling: `src/app.css`
- Localization text: `src/lib/i18n/index.svelte.ts`

## Special Directories

**src-tauri/gen/**
- Purpose: Generated Tauri schemas and ACL artifacts
- Source: Produced by Tauri tooling
- Committed: Yes, currently present in the repository tree

**src-tauri/icons/**
- Purpose: Packaging assets for desktop/mobile targets
- Source: App branding resources, partly generated for target platforms
- Committed: Yes

**.planning/**
- Purpose: Local/generated planning artifacts and project analysis output
- Source: GSD/Codex workflows such as codebase mapping
- Committed: Not inherently; generated inside the workspace as needed

**.codex/**
- Purpose: Local agent/workflow support files used by this workspace
- Source: Codex/GSD setup assets
- Committed: In this workspace it currently appears as local metadata rather than shipped application code

---

*Structure analysis: 2026-04-11*
*Update when directory structure changes*
