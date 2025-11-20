# ClipMan v1.5.0 Release Notes

## 🚀 Major Performance Improvement
- **Event-Driven Clipboard Monitoring**: Replaced 500ms polling with system event-based monitoring using `clipboard-master` crate
  - **CPU Usage**: Reduced idle CPU usage to ~0% (previously constant polling)
  - **Responsiveness**: Instant clipboard change detection via system events
  - **Automatic Fallback**: Falls back to polling if event-driven mode fails

## ✨ New Features
- **Rich Content Type Support**: Added support for HTML and RTF clipboard formats
  - Copy formatted text from applications like Word, browsers, etc.
  - Automatically converts to plain text when pasting back

## 🛠️ Technical Improvements
- Extended `ContentType` enum with `Html` and `Rtf` variants
- Refactored clipboard monitoring architecture for better maintainability
- Added comprehensive error handling with graceful degradation

## 📊 Performance Comparison
| Metric | v1.4.0 (Polling) | v1.5.0 (Event-Driven) |
|--------|------------------|----------------------|
| Idle CPU | ~0.5-1% | ~0% |
| Detection Latency | Up to 500ms | Instant |
| Battery Impact | Moderate | Minimal |

---

**Full Changelog**: https://github.com/Kiaana/ClipMan/compare/v1.4.0...v1.5.0
