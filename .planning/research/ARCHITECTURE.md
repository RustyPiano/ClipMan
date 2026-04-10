# Architecture Research

**Domain:** Local-first desktop clipboard manager
**Researched:** 2026-04-11
**Confidence:** MEDIUM

## Standard Architecture

### System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                    Desktop Interaction Layer                │
├─────────────────────────────────────────────────────────────┤
│  Main Window  │  Tray/Menu  │  Global Shortcut  │ Settings │
└───────────────┬─────────────┬───────────────────┬──────────┘
                │             │                   │
┌───────────────▼─────────────▼───────────────────▼──────────┐
│                     UI State / IPC Facade                  │
├─────────────────────────────────────────────────────────────┤
│  clipboard store  │  settings store  │  permission state   │
└───────────────┬─────────────┬───────────────────┬──────────┘
                │             │                   │
┌───────────────▼─────────────▼───────────────────▼──────────┐
│                  Runtime Service Boundary                  │
├─────────────────────────────────────────────────────────────┤
│ capture service │ search service │ migration coordinator   │
│ policy engine   │ tray sync      │ permission/check flows  │
└───────────────┬─────────────┬───────────────────┬──────────┘
                │             │                   │
┌───────────────▼─────────────▼───────────────────▼──────────┐
│                  Storage / Projection Layer                │
├─────────────────────────────────────────────────────────────┤
│ canonical blobs │ metadata rows │ searchable projection    │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| Clipboard capture service | Own live OS clipboard monitoring and normalization | Long-lived Rust service with explicit start, stop, and restart lifecycle |
| Storage repository | Persist canonical clipboard entries and retention semantics | SQLite repository with clear pinned/unpinned and content-type rules |
| Search projection | Serve user-facing search with documented scope | FTS-backed index or equivalent shadow representation separate from raw ciphertext storage |
| Migration coordinator | Safely move data directories and rebuild live state | Staged lifecycle flow: pause workers, copy/verify, reopen, swap |
| Policy engine | Apply settings and explain behavior | Single source of truth for retention, compression, searchable scope, and fidelity policy |
| UI state layer | Present accurate status and recovery actions | Typed frontend stores over a narrow Tauri command surface |

## Recommended Project Structure

```text
src/
├── lib/
│   ├── components/        # feature UI and settings sections
│   ├── stores/            # typed state for history, settings, routing, toasts
│   ├── i18n/              # user-facing language strings
│   └── types.ts           # shared frontend DTOs and status enums
├── routes/
│   ├── +page.svelte       # main history/search experience
│   └── settings/+page.svelte
└── app.css

src-tauri/src/
├── main.rs                # runtime wiring and shared AppState
├── commands.rs            # narrow IPC boundary
├── clipboard.rs           # monitor lifecycle and normalization
├── storage.rs             # canonical persistence
├── search.rs              # indexed search policy and queries
├── migration.rs           # staged data-directory migration
├── media.rs               # image preview/compression pipeline
├── permissions.rs         # platform-specific permission state model
└── settings.rs            # policy-backed settings application
```

### Structure Rationale

- **Keep the current brownfield split:** Svelte frontend plus flat Rust backend modules already fits the app; introduce clearer module boundaries instead of a rewrite.
- **Separate `search.rs`, `media.rs`, and `permissions.rs`:** current concerns show that search semantics, image handling, and permission UX are complex enough to deserve explicit ownership.
- **Keep `commands.rs` narrow:** Tauri commands should translate typed requests and responses, not hold business logic.
- **Keep policy logic centralized:** one place must decide retention, searchability, fidelity, and compression semantics.

## Architectural Patterns

### Pattern 1: Canonical Entry Plus Search Projection

**What:** Store the canonical clipboard payload separately from the representation used for search.
**When to use:** When the app must balance encryption, recall, and fast query latency.
**Trade-offs:** Slightly more storage and migration complexity in exchange for explicit behavior and predictable search.

**Example:**
```rust
struct ClipEntry {
    id: String,
    kind: ClipKind,
    canonical_payload: Vec<u8>,
    search_text: Option<String>,
}
```

### Pattern 2: Explicit Runtime Ownership

**What:** Treat monitor, storage, crypto, and tray services as runtime-owned components with start/stop/reopen semantics.
**When to use:** Any time settings can reconfigure live resources such as data path or permissions.
**Trade-offs:** More lifecycle code up front, much lower migration and restart risk later.

**Example:**
```rust
enum RuntimeCommand {
    PauseCapture,
    ResumeCapture,
    ReopenStorage(PathBuf),
}
```

### Pattern 3: Typed Status Across the IPC Boundary

**What:** Commands return structured status objects instead of booleans or strings.
**When to use:** Permissions, migration, search scope, and recoverable failures.
**Trade-offs:** More DTO maintenance, but the UI can actually render meaningful states.

**Example:**
```typescript
type PermissionState =
  | { status: 'granted' }
  | { status: 'needs_system_permission'; platformSteps: string[] }
  | { status: 'error'; message: string };
```

## Data Flow

### Capture Flow

```text
[OS Clipboard Event]
    ↓
[clipboard.rs monitor]
    ↓
[normalization + content classification]
    ↓
[storage.rs canonical write]
    ├──> [search.rs projection update]
    └──> [tray/UI refresh event]
```

### Search Flow

```text
[User query]
    ↓
[frontend store]
    ↓
[commands.rs search request]
    ↓
[search.rs indexed lookup]
    ↓
[storage.rs hydrate visible result payloads]
    ↓
[frontend list render]
```

### Migration Flow

```text
[User selects new data path]
    ↓
[preflight validation]
    ↓
[pause capture + freeze writes]
    ↓
[copy and verify files]
    ↓
[reopen storage/crypto at new path]
    ↓
[resume capture + refresh tray/UI]
```

### Key Data Flows

1. **Capture and persist:** fast path must stay lightweight and deterministic.
2. **Search and hydrate:** search should hit an index first and only hydrate the result set.
3. **Migration and reopen:** lifecycle control matters more than raw file IO speed.
4. **Settings and policy application:** every user-facing setting should change one authoritative policy layer.

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0-10k retained items | Single-process monolith is fine; prioritize search indexing and payload separation |
| 10k-100k retained items | Page history queries, keep previews lightweight, benchmark FTS and hydration costs |
| Heavy image usage | Move previews/compression off the hot path and avoid sending full blobs to the UI by default |

### Scaling Priorities

1. **First bottleneck:** search recall and latency once retained text histories get large.
2. **Second bottleneck:** renderer and tray payload churn when image-heavy histories are hydrated eagerly.

## Anti-Patterns

### Anti-Pattern 1: Search as a UI concern only

**What people do:** Patch search in the frontend or command layer while storage stays vague.
**Why it's wrong:** Search scope, privacy, and performance are storage-model decisions.
**Do this instead:** Define search behavior alongside storage and retention policy.

### Anti-Pattern 2: Live migration as file copy only

**What people do:** Copy DB files and update settings while background workers keep running.
**Why it's wrong:** The process still owns open state that now points at the wrong place.
**Do this instead:** Build an explicit pause, verify, reopen, and resume lifecycle.

### Anti-Pattern 3: Type labels without fidelity guarantees

**What people do:** Add `Html`, `Rtf`, or `File` variants but normalize restore to plain text.
**Why it's wrong:** The architecture appears richer than the real product contract.
**Do this instead:** Define full-fidelity, preview-only, and degraded modes per content type.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| OS clipboard | Native Rust service plus typed IPC events | Core hot path; avoid expensive work before canonical persistence |
| Tauri plugin ecosystem | Plugin registration in runtime bootstrap | Keep only the plugins/capabilities needed for current user flows |
| SQLite FTS5 | Local indexed projection | Best fit for local searchable history with clear behavior |
| OS credential store or stronghold | Secret material storage | Needed if security claims move beyond casual local obscurity |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `clipboard.rs` ↔ `storage.rs` | direct service API | Canonical payloads and retention policy |
| `storage.rs` ↔ `search.rs` | projection update/query | Avoid search-specific logic leaking into every storage path |
| `commands.rs` ↔ frontend stores | typed IPC DTOs | Never rely on truthy strings or implicit booleans for stateful flows |
| `migration.rs` ↔ runtime services | orchestration commands | Migration must own pause/reopen/resume sequencing |

## Sources

- [Tauri Plugins](https://v2.tauri.app/plugin/) — official Tauri 2 integration surface relevant to desktop utilities
- [SQLite FTS5](https://sqlite.org/fts5.html) — official search-index behavior and query capabilities
- [CopyQ documentation](https://copyq.readthedocs.io/en/latest/) — reference for mature clipboard-manager data-type and workflow breadth
- [.planning/PROJECT.md](../PROJECT.md) — current project scope and brownfield priorities
- [.planning/codebase/ARCHITECTURE.md](../codebase/ARCHITECTURE.md) — actual current ClipMan architecture baseline
- [.planning/codebase/CONCERNS.md](../codebase/CONCERNS.md) — architecture pain points shaping this milestone

---
*Architecture research for: local-first desktop clipboard manager*
*Researched: 2026-04-11*
