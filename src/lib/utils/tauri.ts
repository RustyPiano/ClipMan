export function hasTauriRuntime() {
  if (typeof window === 'undefined') return false;
  return Boolean((window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}
