# Codebase Concerns

**Analysis Date:** 2026-04-11
**Scope:** Concern-only audit of the current `ClipMan` repository, based on static analysis plus attempted verification with `npm run check`, `npm run lint`, and `cargo test --manifest-path src-tauri/Cargo.toml`.

## Tech Debt

**Search implementation vs documented behavior:**
- Files: `src-tauri/src/storage.rs:280-333`, `src/lib/stores/clipboard.svelte.ts:134-152`, `README.md:37-45`, `README.md:77-80`
- Issue: Docs and frontend comments describe FTS5/full-text search, but the runtime implementation decrypts the newest 1000 text rows and performs a lowercase substring scan in Rust.
- Why: Likely an MVP simplification after encrypted storage and deduplication were added.
- Impact: Search quality, scalability, and documentation fidelity are all weaker than advertised; users will miss older results and contributors will assume a capability that does not exist.
- Fix approach: Add a real indexed search path (for example an SQLite FTS mirror or searchable shadow fields), decide how encrypted search should work, and update docs/comments to match implementation until then.

**Settings drift and partially implemented options:**
- Files: `src-tauri/src/settings.rs:8-18`, `src-tauri/src/storage.rs:198-208`, `src/lib/stores/clipboard.svelte.ts:104-126`, `src-tauri/src/clipboard.rs:358-386`
- Issue: `auto_cleanup` is persisted but not consulted when pruning history, `loadHistory()` hardcodes `limit: 100`, and `storeOriginalImage` still resizes images larger than 2048px.
- Why: Settings were added faster than the runtime contract was cleaned up.
- Impact: UI promises do not match behavior, making support and future refactors harder.
- Fix approach: Define the intended semantics per setting, enforce them in one place, and add regression tests around settings that affect data retention and storage size.

**Repo/build metadata drift:**
- Files: `src-tauri/Cargo.toml:4-7,15`, `src-tauri/tauri.conf.json:6-10`, `package.json:6-14`
- Issue: Cargo metadata still says "Modern clipboard manager for Windows" with a placeholder repository URL, `devtools` is enabled in the Tauri dependency, and build scripts assume npm while README suggests bun-first onboarding.
- Why: Packaging and runtime hardening were not revisited after cross-platform and release work expanded.
- Impact: Packaging confusion, weaker release posture, and avoidable onboarding friction.
- Fix approach: Align metadata with the actual project, make the supported package manager explicit, and gate `devtools` to debug builds only.

**Operational bootstrap gap:**
- Files: `package.json:6-14`, `src-tauri/tauri.conf.json:6-10`
- Issue: A clean checkout cannot run the advertised verification commands without prior dependency installation and a built frontend.
- Why: Verification assumptions are implicit rather than codified.
- Impact: CI setup and local contributor workflows are brittle; "green" becomes environment-dependent.
- Fix approach: Document the exact bootstrap sequence, add CI, and decouple Rust tests from generated frontend assets where possible.

## Known Bugs

**Changing the data location is unsafe in the running app:**
- Symptoms: After migration, the app can continue using the old open SQLite connection and old in-memory crypto state; deleting old data during migration risks stale handles or inconsistent writes. Restarting the monitor may also leave the previous monitoring thread alive.
- Trigger: Use Settings -> Storage -> change data location in `src/routes/settings/+page.svelte:173-195`.
- Files: `src-tauri/src/commands.rs:558-603`, `src-tauri/src/main.rs:167-177`, `src-tauri/src/clipboard.rs:133-164`, `src-tauri/src/clipboard.rs:167-264`
- Workaround: Avoid in-app migration in production; close the app fully before moving the data directory.
- Root cause: Migration copies files and updates settings, but never rebuilds `ClipStorage` or `Crypto`; `ClipboardMonitor` has no shutdown handle or `Drop` behavior.
- Blocked by: A lifecycle refactor for monitor stop/restart and state reinitialization.

**Permission warning UI is effectively dead code:**
- Symptoms: The warning banner never appears even when the backend returns `"denied: ..."` for clipboard permission.
- Trigger: Missing clipboard/accessibility permission on macOS.
- Files: `src-tauri/src/commands.rs:90-102`, `src/lib/components/PermissionCheck.svelte:7-19`, `src/lib/components/PermissionCheck.svelte:47-91`
- Workaround: None in the UI; users have to infer the problem from missing functionality.
- Root cause: The backend returns `Result<String, String>`, while the frontend stores the result in a boolean state; any non-empty string is truthy in JS, so `{#if !hasPermission}` never fires.

**History limit changes are not respected consistently by the frontend:**
- Symptoms: Raising the limit above 100 does not load more than 100 items; lowering the limit does not immediately prune the list a user sees after reload.
- Trigger: Change `maxHistoryItems` in settings.
- Files: `src/lib/stores/clipboard.svelte.ts:33-43`, `src/lib/stores/clipboard.svelte.ts:104-126`, `src-tauri/src/storage.rs:198-208`
- Workaround: Limit enforcement eventually happens only as new clipboard entries are inserted.
- Root cause: The frontend hardcodes `limit: 100` when loading history, while backend cleanup only runs during insert.

**Rich content/file fidelity is lower than the type system suggests:**
- Symptoms: HTML and RTF are copied back as plain text, and `File` content is copied as a path string rather than a file-list clipboard payload.
- Trigger: Copy rich text or file selections, then restore them from ClipMan.
- Files: `src-tauri/src/storage.rs:8-14`, `src-tauri/src/commands.rs:203-280`
- Workaround: Use the original source application when formatting or file payload fidelity matters.
- Root cause: Storage distinguishes `Html`, `Rtf`, and `File`, but restore logic normalizes them to text writes.

## Security Considerations

**At-rest encryption only weakly protects the database:**
- Risk: The AES key is stored as `.clipman.key` in the same directory as `clipman.db`, including user-selected custom paths. Anyone who can read the data directory can read both ciphertext and key.
- Files: `src-tauri/src/main.rs:57-98`, `src-tauri/src/main.rs:145-170`, `src-tauri/src/migration.rs:27-64`
- Current mitigation: The key file is chmod `0600` on Unix, and content is encrypted before insertion.
- Recommendations: Move key storage to the OS keychain/credential manager, separate key material from user-selected sync folders, and document the real threat model in README.

**Updater release notes are rendered as unsanitized HTML:**
- Risk: Markdown from update metadata is parsed with `marked` and injected via `{@html}`. CSP likely blocks many script vectors, but this is still an avoidable HTML injection surface for network-fetched content.
- Files: `src/lib/components/ui/MarkdownContent.svelte:1-18`, `src/lib/components/settings/AboutSection.svelte:81-98`, `src-tauri/tauri.conf.json:27-29`
- Current mitigation: Tauri CSP is present and updater metadata is signed by the updater flow.
- Recommendations: Sanitize rendered markdown, disable raw HTML in markdown, and treat remote release notes as untrusted input.

**Capability surface is broader than the app currently needs:**
- Risk: The default capability grants shell, filesystem, dialog, clipboard, updater, notification, and global shortcut access to the main window; shell `open` is enabled globally.
- Files: `src-tauri/capabilities/default.json:8-23`, `src-tauri/tauri.conf.json:60-63`, `src-tauri/src/commands.rs:527-554`
- Current mitigation: Only the local UI can invoke commands, and a CSP is configured.
- Recommendations: Reduce permissions to the minimum set, route folder opening through the Tauri plugin API consistently, and disable release `devtools` (`src-tauri/Cargo.toml:15`).

**Debug logging is forced on for the whole app:**
- Risk: Startup, migration, and operational paths emit debug-level logs in all environments, increasing privacy leakage risk over time as more features are added.
- Files: `src-tauri/src/main.rs:101-105`, `src-tauri/src/commands.rs:110-148`, `src-tauri/src/commands.rs:566-603`
- Current mitigation: Logs do not currently print full clipboard text bodies.
- Recommendations: Make log level environment-driven in release, redact file paths where possible, and add a privacy review for future logging changes.

## Performance Bottlenecks

**Search path decrypts and scans rows on every query:**
- Problem: Each search fetches up to 1000 text rows, decrypts them, lowercases them, and scans them in memory.
- Measurement: No benchmark exists in the repo; the code hard-limits search to the newest 1000 text rows.
- Files: `src-tauri/src/storage.rs:280-333`
- Cause: No indexed search path exists for the encrypted storage model.
- Improvement path: Add an indexed search representation and benchmark p50/p95 latency on realistic history sizes.

**Clipboard history payload and renderer decoding are memory-heavy:**
- Problem: Every item is sent to the frontend as base64; images become `data:image/png;base64,...` strings and text is decoded back into UTF-8 inside each card component.
- Measurement: No instrumentation exists, but overhead grows with image-heavy histories and large text items.
- Files: `src-tauri/src/storage.rs:50-84`, `src/lib/stores/clipboard.svelte.ts:104-126`, `src/lib/components/ClipboardItem.svelte:30-60`
- Cause: IPC and rendering are optimized for simplicity rather than payload size or reuse.
- Improvement path: Send previews and metadata separately from full content, predecode text once in the store, and lazy-load full blobs on demand.

**Tray updates rebuild too much work on every clipboard event:**
- Problem: Every saved clipboard item emits an event and rebuilds the tray, which re-queries storage and may decode/resize images synchronously.
- Measurement: No timing exists, but the hot path includes DB access, base64/image work, and menu reconstruction.
- Files: `src-tauri/src/clipboard.rs:267-299`, `src-tauri/src/tray.rs:67-115`, `src-tauri/src/tray.rs:206-279`
- Cause: Tray refresh is modeled as a full rebuild after each event.
- Improvement path: Debounce tray refreshes, cache the menu model separately from storage, and move icon work off the hot path.

**Image processing does multiple conversions per clipboard image:**
- Problem: Clipboard images are PNG-encoded from raw bytes, then decoded/resized/re-encoded again for storage or preview; "store original" still recompresses and may resize large images.
- Measurement: No benchmark exists, but the pipeline clearly performs repeated transformations.
- Files: `src-tauri/src/clipboard.rs:74-112`, `src-tauri/src/clipboard.rs:223-259`, `src-tauri/src/clipboard.rs:306-386`
- Cause: The pipeline normalizes everything to PNG bytes early.
- Improvement path: Keep original bytes when safe, background-process previews, and record telemetry for large image events.

## Fragile Areas

**Clipboard monitor lifecycle and concurrency:**
- Why fragile: The monitor thread has no stop signal, fallback polling is an infinite loop, and several paths rely on direct mutex locking with `unwrap()`.
- Common failures: Duplicate captures after migration, hard-to-reason self-copy suppression, and stuck background threads during future lifecycle changes.
- Files: `src-tauri/src/clipboard.rs:11-21`, `src-tauri/src/clipboard.rs:133-164`, `src-tauri/src/clipboard.rs:167-264`
- Safe modification: Introduce a cancellation channel or join handle, own the worker lifecycle explicitly in `AppState`, and test restart/teardown behavior before changing migration or autostart flows.
- Test coverage: None for monitor lifecycle.

**Startup and app setup path:**
- Why fragile: Setup chains many `expect` and `unwrap` calls across app-data discovery, key init, DB init, tray icon creation, and generated Tauri context.
- Common failures: Startup aborts instead of entering a degraded mode when environment or assets are missing.
- Files: `src-tauri/src/main.rs:145-170`, `src-tauri/src/main.rs:189-190`, `src-tauri/src/main.rs:315-316`, `src-tauri/src/crypto.rs:12-18`
- Safe modification: Convert fallible startup steps to typed error handling and staged initialization so user-facing failures are recoverable.
- Test coverage: No startup integration tests; current `cargo test` is blocked by missing `../dist`.

**Shared state and mutex usage:**
- Why fragile: Multiple modules lock shared state directly with `unwrap()` while others use `safe_lock`, so the locking model is inconsistent and easy to extend badly.
- Common failures: Poisoned locks become unpredictable, lock-scope regressions are hard to spot, and expensive work can accidentally happen while holding a mutex.
- Files: `src-tauri/src/settings.rs:56-146`, `src-tauri/src/clipboard.rs:35-39`, `src-tauri/src/tray.rs:63-70`, `src-tauri/src/commands.rs:570`, `src-tauri/src/main.rs:38-44`
- Safe modification: Standardize on one lock helper, shrink critical sections, and add concurrency-focused tests around storage/tray/monitor interactions.
- Test coverage: No concurrency or stress tests.

**Search/data contract between backend and frontend:**
- Why fragile: Comments, docs, settings, and UI labels describe behavior that the backend no longer provides exactly.
- Common failures: Contributors will "fix" the wrong layer because the product contract is unclear.
- Files: `README.md:35-49`, `src/lib/stores/clipboard.svelte.ts:143-147`, `src-tauri/src/storage.rs:280-333`
- Safe modification: Document the real contract first, then refactor implementation to match it.
- Test coverage: None.

## Scaling Limits

**Clipboard history size and payload model:**
- Current capacity: Practical usage appears tuned around roughly 100 recent items and thumbnail-sized images.
- Limit: Memory and IPC overhead rise sharply with large images or when `storeOriginalImage` is enabled.
- Symptoms at limit: Slower history loads, larger renderer heap, and heavier tray rebuilds.
- Files: `src/lib/stores/clipboard.svelte.ts:104-126`, `src-tauri/src/storage.rs:62-84`, `src-tauri/src/clipboard.rs:330-386`
- Scaling path: Split metadata from blob content, page history queries, and cache derived image previews.

**Search scale:**
- Current capacity: The newest 1000 text rows only.
- Limit: Older items and non-text items are invisible to search; latency grows with item size because rows are decrypted and scanned in process.
- Symptoms at limit: False negatives and slower interactive search.
- Files: `src-tauri/src/storage.rs:285-333`
- Scaling path: FTS index or searchable plaintext shadow fields with explicit privacy decisions.

**Verification and release scale:**
- Current capacity: Single-developer, manual workflow.
- Limit: The current repo state does not support clean reproducible checks without extra install/build side effects.
- Symptoms at limit: CI brittleness, onboarding friction, and regressions landing unnoticed.
- Files: `package.json:6-14`, `src-tauri/tauri.conf.json:6-10`
- Scaling path: Add documented bootstrap, CI jobs, and test targets that do not depend on prebuilt frontend artifacts.

## Dependencies at Risk

**`clipboard-master` beta dependency:**
- Files: `src-tauri/Cargo.toml:55-57`, `src-tauri/src/clipboard.rs:2`
- Risk: Clipboard monitoring depends on `clipboard-master = "4.0.0-beta.6"`, which sits on the app's core runtime path and may change behavior or remain unstable.
- Impact: Monitor reliability is central to product correctness; failures fall back to a polling loop with no shutdown path.
- Migration plan: Evaluate stable alternatives or wrap the dependency behind a smaller internal adapter with explicit lifecycle control.

**Tauri surface area and plugin count:**
- Files: `src-tauri/Cargo.toml:15-24`, `src-tauri/capabilities/default.json:8-23`
- Risk: Many plugins are initialized globally even though most user flows are simple; more plugins mean more update and hardening burden.
- Impact: Release hardening and cross-platform testing complexity grow faster than the product surface suggests.
- Migration plan: Audit plugin usage and drop unused capabilities/plugins before adding more features.

## Missing Critical Features

**True searchable history model:**
- Problem: There is no indexed, encrypted-aware search strategy despite the app positioning search as a core feature.
- Current workaround: Substring scan over the newest text rows.
- Blocks: Predictable search quality, scaling to larger histories, advanced filters, and future sync/search features.
- Implementation complexity: Medium to High, depending on privacy requirements.

**Safe runtime reconfiguration for storage/migration:**
- Problem: Changing data location does not safely rotate the live DB, key, or monitor state.
- Current workaround: Users should close the app and migrate offline.
- Blocks: Reliable storage relocation, backup/export features, and future multi-profile support.
- Implementation complexity: High because it touches lifecycle, state ownership, and IO consistency.

**Structured error states in the UI:**
- Problem: Several backend failures only hit logs or console output; the UI often assumes success and has limited degraded-mode handling.
- Current workaround: Manual retry or checking logs.
- Blocks: Trustworthy permissions onboarding, migration recovery, and supportability.
- Implementation complexity: Medium.

## Test Coverage Gaps

**Clipboard capture, dedupe, restore, and migration paths:**
- What's not tested: `ClipboardMonitor`, `ClipStorage::insert/search/get_recent`, copy-back logic, migration lifecycle, and hotkey re-registration.
- Files: `src-tauri/src/clipboard.rs`, `src-tauri/src/storage.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`
- Risk: The most stateful flows can regress silently.
- Priority: High
- Difficulty to test: Medium to High; requires integration harnesses and some platform-conditioned abstractions.

**Frontend state management and permission UX:**
- What's not tested: Clipboard store initialization, history-limit behavior, permission banner rendering, settings save/reset flows, and update UI behavior.
- Files: `src/lib/stores/clipboard.svelte.ts`, `src/lib/components/PermissionCheck.svelte`, `src/routes/settings/+page.svelte`
- Risk: User-facing regressions are currently caught only manually.
- Priority: High
- Difficulty to test: Medium; can be covered with component tests and mocked Tauri invokes.

**Verification commands from a clean checkout:**
- What's not tested: Reproducible bootstrap itself.
- Observed on 2026-04-11:
- `npm run check` failed because `svelte-check` was not installed locally.
- `npm run lint` failed because `eslint` was not installed locally.
- `cargo test --manifest-path src-tauri/Cargo.toml` failed because `tauri::generate_context!()` requires `src-tauri/../dist`, which was absent.
- Risk: Contributors cannot trust local green checks unless they already know the missing setup steps.
- Priority: Medium
- Difficulty to test: Low; this is mostly CI and documentation work.

*Concerns audit: 2026-04-11*
*Update this file whenever the search model, migration flow, permissions UX, or verification baseline changes.*
