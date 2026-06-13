# ClipMan v2.0.2

## Bug Fixes
- Pinned clips now show up in the History list and in search results — previously they were hidden from History and could not be found by searching.
- Copying an item while a search is active now moves it back to the top of the history, just like copying without a search.
- Typing a search query now highlights the top match, so pressing Enter pastes the best result instead of a stale row left selected by the previous query.
- The highlight now follows the clip it is on when the list reorders underneath it (e.g. after a copy bumps a row to the top), instead of jumping to a different item at the same position.

## Improvements
- The Pinned tab now shows a clearer message — with a one-click jump to History — when a search matches items that simply aren't pinned, and its count badge reflects the matching pinned items while searching.
- Removed the brief search-spinner flicker that appeared when copying during an active search.

---

# ClipMan v2.0.2 更新日志

## 问题修复
- 置顶内容现在会显示在「历史」列表和搜索结果中——此前置顶项会从历史中消失，也无法被搜索到。
- 在搜索状态下复制某条内容，现在会像未搜索时一样把它移动到历史记录顶部。
- 输入搜索词后会自动高亮第一条匹配项，按回车粘贴的就是最匹配的结果，而不是上一次搜索残留选中的某一行。
- 当列表在脚下重新排序时（例如复制使某行上浮到顶部），高亮会跟随原本选中的那条内容，而不再跳到同一位置的另一条上。

## 优化
- 在「置顶」标签页下，当搜索命中的内容只是未被置顶时，会显示更清晰的提示并支持一键切换到「历史」；搜索时置顶角标数字也会反映匹配的置顶条数。
- 修复了在搜索状态下复制时搜索图标会短暂闪烁的问题。
