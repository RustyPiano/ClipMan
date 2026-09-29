## ClipMan v2.4.2

### 修复与改进

- 修复 QuickBar 多选：用鼠标点击行右下角的勾选按钮后按回车，会反复勾选、取消，而不是合并粘贴。现在点击行操作按钮后焦点留在搜索框，回车直接合并粘贴。
- 新增键盘多选：`⇧↑` / `⇧↓` 从当前行开始连续选择，往回按会缩小范围；`Esc` 清空多选。底部提示栏显示该快捷键。

---

### Fixes and improvements

- Fixed QuickBar multi-select: after clicking a row's checkbox action, Enter toggled the row again instead of merge-pasting. Row actions now leave focus in the search box, so Enter merge-pastes the selection.
- Added keyboard multi-select: `⇧↑` / `⇧↓` extends a contiguous selection from the current row and shrinks it when reversed; `Esc` clears it. The footer shows the shortcut.
