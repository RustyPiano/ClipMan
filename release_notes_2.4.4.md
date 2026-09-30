## ClipMan v2.4.4

### 修复与改进

- **多选的操作方式与访达一致。** `⇧↑` / `⇧↓` 选择连续范围，往回缩到只剩起始行时自动取消多选；不按 Shift 的 `↑` / `↓` 清空多选，只移动光标。
- **`⇧`+点击选择范围，不再直接粘贴。** 还没有多选时先勾选点到的行，再次 `⇧`+点击选中这两行之间的范围。
- **只勾选一行时，回车使用勾选的那一行，** 而不是鼠标悬停所在的行；底部提示栏显示“已选 1 项”。
- **删除或取消置顶当前行后，光标留在原位置，** 落到下一行，不再跳回列表顶部。
- 在置顶面板用 `⌘⇧↑` / `⌘⇧↓` 调整顺序时，列表跟随被移动的行滚动。

---

### Fixes and improvements

- **Multi-select now works like Finder.** `⇧↑` / `⇧↓` select a contiguous range and clear the selection once it shrinks back to the starting row; plain `↑` / `↓` clear the selection and only move the cursor.
- **`⇧`-click selects a range instead of pasting.** With nothing selected it checks the clicked row; the next `⇧`-click selects everything in between.
- **With a single checked row, Enter uses that row** rather than the row under the mouse, and the footer shows "1 selected".
- **Deleting or unpinning the cursor row keeps the cursor in place** on the next row instead of jumping back to the top.
- Reordering pinned items with `⌘⇧↑` / `⌘⇧↓` scrolls the list along with the moved row.
