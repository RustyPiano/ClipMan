# Testing Patterns

**Analysis Date:** 2026-04-11

## Test Framework

**Runner:**
- Rust's built-in test harness via `cargo test` is the only test runner currently present.
- All discovered automated tests live under `src-tauri`; there is no JS or TS test runner dependency in `package.json`.
- No root `test` script is defined in `package.json`.
- `src-tauri/tauri.conf.json` ties the Tauri app to `frontendDist: "../dist"`, which affects how backend test builds resolve frontend assets.

**Assertion Library:**
- Rust standard library assertions only: `assert_eq!`, `assert!`, and `unwrap_err()`-style checks.
- No custom matcher library is used.

**Run Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml tray::tests::test_truncate_content_text
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
bun run check
bun run lint
```

- `bun run check` and `bun run lint` are static validation commands, not test runners.
- During this analysis, `cargo test --manifest-path src-tauri/Cargo.toml -- --list` failed before running tests because `tauri::generate_context!()` expected `../dist` to exist and that directory was missing.

## Test File Organization

**Location:**
- Rust unit tests are colocated inline with source code under `#[cfg(test)]` modules:
- `src-tauri/src/crypto.rs`
- `src-tauri/src/migration.rs`
- `src-tauri/src/tray.rs`
- No top-level `tests/`, `e2e/`, `spec/`, `__tests__/`, or frontend `*.test.*` files were found in the repository.

**Naming:**
- Rust test functions use `test_*` snake_case names such as `test_encrypt_decrypt`, `test_same_path_error`, `test_truncate_content_text`, and `test_truncate_content_newlines`.
- There is no naming distinction between unit, integration, and e2e tests because only inline unit tests exist today.

**Structure:**
```text
src-tauri/src/
  crypto.rs      # #[cfg(test)] mod tests
  migration.rs   # #[cfg(test)] mod tests
  tray.rs        # #[cfg(test)] mod tests

src/
  ...            # no colocated frontend test files
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_path_error() {
        let result = migrate_data(&test_dir, &test_dir, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same"));
    }
}
```

**Patterns:**
- Tests are small, colocated, and focused on helper behavior rather than user flows.
- Arrange, act, and assert are kept inline in a single function rather than split into helpers.
- No `before_each` or `after_each` equivalent setup helpers are used.
- No async tests (`tokio::test`) are present.

## Mocking

**Framework:**
- No mocking framework is in use. `mockall`, `rstest`, `vitest`, `jest`, `playwright`, and similar tools are absent.

**Patterns:**
```rust
let test_dir = std::env::temp_dir().join("clipman_test");
std::fs::create_dir_all(&test_dir).unwrap();

let result = migrate_data(&test_dir, &test_dir, false);
assert!(result.is_err());
```

- Tests call real functions directly and use literal inputs.
- When filesystem behavior is needed, the test creates a real temporary directory with std APIs and cleans it up manually.
- No established stubbing pattern exists for clipboard IO, updater behavior, tray interactions, notifications, or frontend `invoke()` calls.

**What to Mock:**
- There is no active convention yet for mocking external dependencies because the current suite does not cover those layers.

**What NOT to Mock:**
- Existing tests do not mock pure helper logic such as encryption round-trips, locale string selection, or text truncation.

## Fixtures and Factories

**Test Data:**
- Test data is written inline inside each test function.
- Inputs are simple literals such as locale strings, byte slices, enum variants, and temporary directory paths.
- No shared factories or reusable fixture modules exist.

**Location:**
- No `tests/fixtures/`, `tests/factories/`, or shared test helper layer is present.

## Coverage

**Requirements:**
- No coverage threshold is configured.
- No CI job enforces test, lint, or typecheck coverage before release packaging.

**Configuration:**
- `package.json` contains `check`, `lint`, and `format`, but no `test` or coverage scripts.
- `.github/workflows/release.yml` installs dependencies and packages release artifacts, but does not run `bun run check`, `bun run lint`, or `cargo test`.

**View Coverage:**
- No coverage command is configured today.

## Test Types

**Unit Tests:**
- Present only in Rust backend files.
- Current unit coverage focuses on:
- crypto round-trip behavior in `src-tauri/src/crypto.rs`
- migration validation in `src-tauri/src/migration.rs`
- tray i18n and text truncation helpers in `src-tauri/src/tray.rs`

**Integration Tests:**
- None present.
- No automated test exercises Tauri commands, SQLite behavior through IPC, clipboard monitoring, updater flows, or settings persistence end to end.

**E2E Tests:**
- None present.
- No browser, WebView, or desktop automation framework is configured.

## Common Patterns

**Async Testing:**
- Not used. All discovered tests are synchronous.

**Error Testing:**
```rust
let result = migrate_data(&test_dir, &test_dir, false);
assert!(result.is_err());
assert!(result.unwrap_err().contains("same"));
```

- Error cases typically assert both that failure occurred and that the returned message contains a meaningful substring.

**Snapshot Testing:**
- Not used.

## Current Constraints And Gaps

- Frontend quality currently depends on static validation commands rather than automated unit or component tests.
- Backend tests do not cover `src-tauri/src/commands.rs`, `src-tauri/src/clipboard.rs`, `src-tauri/src/storage.rs`, or startup behavior in `src-tauri/src/main.rs`.
- Running `cargo test --manifest-path src-tauri/Cargo.toml` in a clean workspace is currently blocked by `src-tauri/tauri.conf.json` requiring `../dist` to exist for `tauri::generate_context!()`.
- Because tests are inline only, there is no reusable harness for mocking clipboard events, tray state, or filesystem migrations beyond direct std calls.

## Recommended Match-Existing Additions

- If you add Rust tests now, match the current style first: colocated `#[cfg(test)]` modules, literal fixtures, and direct `assert!` or `assert_eq!` assertions.
- For frontend changes, the closest existing quality gates are `bun run check` and `bun run lint`.
- If a frontend runner or integration harness is added later, document the scripts and conventions here so testing stops depending on repository memory.

---
*Testing analysis: 2026-04-11*
*Update when a frontend runner, CI test gate, or coverage tool is added*
