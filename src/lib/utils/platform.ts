// Single source of truth for coarse OS detection in the renderer.
// `navigator.platform` is deprecated; `navigator.userAgent` is not, and inside
// the Tauri webviews we ship (WKWebView / WebView2 / WebKitGTK) it reliably
// carries "Macintosh" on macOS and "Windows"/"Linux" elsewhere.
export const isMac = typeof navigator !== 'undefined' && /mac/i.test(navigator.userAgent);
