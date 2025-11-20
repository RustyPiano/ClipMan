# ClipMan v1.6.0 Release Notes

## 🚀 Performance Improvements
- **Async Image Processing**: Image resize and thumbnail generation now happen in background tasks
  - Eliminates blocking of clipboard monitor (200-500ms → <10ms)
  - Enables rapid consecutive image copies without loss
  - Smoother overall system performance

## 🛠️ UX Improvements  
- **Simplified List Rendering**: Removed virtual list for smoother scrolling
  - Eliminated scroll jitter and lag
  - More natural browser-native scrolling
  - Simpler, more maintainable code

## 📊 Performance Comparison
| Metric | v1.5.0 | v1.6.0 |
|--------|--------|--------|
| Image Processing Block | 200-500ms | <10ms |
| Scroll Performance | Jitter/Lag | Smooth |
| Code Complexity | Higher | Lower |

---

**Full Changelog**: https://github.com/Kiaana/ClipMan/compare/v1.5.0...v1.6.0
