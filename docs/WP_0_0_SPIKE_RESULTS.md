# WP-0.0 Spike Results

Phase 1 is proceeding under a project-owner gate override. Runtime spike validation is deferred until platform QA after implementation.

## Windows S1 Focus Model

Status: Deferred runtime validation

Implementation may proceed using the documented design:

- Do not use `WS_EX_NOACTIVATE`.
- Record the current foreground window before QuickBar is shown.
- Let QuickBar take focus normally so search can receive keyboard input.
- On paste, hide QuickBar, restore the recorded foreground window, then send `Ctrl+V`.
- If foreground restoration fails during platform testing, document the failure and degrade to copy-only or a visible user-facing error.

Unverified runtime risks:

- `SetForegroundWindow` may be blocked by foreground-lock rules for some target apps.
- `AttachThreadInput` or an equivalent focus-restoration fallback may be required.
- Browser address bars, editors, terminals, and Electron apps may differ in how they accept restored focus plus `Ctrl+V`.

## macOS S2 Non-Activating Panel

Status: Verified on macOS

Validated implementation:

- Use a Tauri v2-compatible non-activating panel approach. The production implementation currently sets the macOS window style through the Tauri window handle and `objc2`; `tauri-nspanel` remains only a fallback candidate if future compatibility issues justify it.
- QuickBar should be able to become the key window for search input without making ClipMan the active foreground app.
- On paste, write the clipboard, hide QuickBar, then send `Cmd+V`.

Verified result:

- QuickBar search focus works on the current macOS runtime.
- The final paste flow works after Accessibility permission is granted.
- ClipMan does not need to become the active foreground app for the verified macOS flow.

## Proceeding Rule

Phase 1 implementation is allowed to proceed from these documented design decisions. macOS runtime behavior has been verified; do not claim Windows runtime behavior is verified until the platform test matrix has actually been run.
