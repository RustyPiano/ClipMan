# SPEC-2：前端支持 files/富文本 + ⌥Enter 纯文本粘贴

> Wave 2-A · 模型：Opus · 文件域：`src/lib/**`、`src/routes/**`
> 对应任务：#5/#6 前端部分、#10 纯文本粘贴（UI）
> 前置：SPEC-1 已验收合入（后端已提供 `contentType: 'files'`、`hasHtml`、`paste_clip` 的 `plain` 参数）
> ⚠️ 编辑任何 .svelte / .svelte.ts 文件前必须先加载 `svelte-code-writer` 技能，并遵循 `svelte-core-bestpractices`（Svelte 5 runes 语法）

## 0. 后端合同（SPEC-1 产物，以实际代码为准，动手前先读）

- `FrontendClipItem`：`contentType: 'text' | 'image' | 'files'`；新字段 `hasHtml: boolean`
- files 的 `content` = base64(路径按 `\n` 连接的 UTF-8 文本)；预览截断前 4096 字节
- `paste_clip` 命令新增可选参数 `plain?: boolean`
- `get_clip`（完整内容）对 text 与 files 都返回完整 base64 内容

## 1. types.ts

- `ContentType` 加 `'files'`
- `ClipItem` 接口加 `hasHtml: boolean`

## 2. 展示组件

### 2.1 ClipboardItem.svelte（列表项）
- files 渲染：解码 base64 → 按 `\n` 拆路径 → 展示：
  - 单文件：文件名（basename）+ 目录淡色缩略（尾部截断）
  - 多文件：前 2 个文件名 + `+N` 计数徽标
  - 图标用 lucide-svelte（单文件 `File`、多文件 `Files`、路径以 `/` 结尾或无扩展名的目录可统一用 `Folder`——不做磁盘存在性检查，纯字符串判断，保持轻量）
- text 且 `hasHtml`：预览行尾加一个小徽标（如 `Aa` 样式或 `富` 字徽标，样式与现有 kbd-keycap/chip 视觉一致），title 提示"包含富文本格式"
- **注意**：base64 解码沿用项目现有工具/写法（先 grep `atob`/`TextDecoder` 现有用法，保持一致；中文路径必须正确解码 UTF-8——`atob` 直接输出 latin1，需要 `Uint8Array` + `TextDecoder`，如项目已有 util 则复用）

### 2.2 ClipPreview.svelte（预览面板）
- files：完整路径列表逐行展示（等宽字体、可滚动，样式与现有文本预览一致）
- text + hasHtml：预览仍显示纯文本（**不要**渲染 HTML——安全与样式污染风险，明确不做）

## 3. 粘贴交互（+page.svelte）

- 现有 Enter 粘贴路径找到 invoke `paste_clip` 的调用点（大概率在 store 或 +page.svelte 的 keydown 处理）
- 新增：**⌥Enter（Alt+Enter）= 纯文本粘贴**（`plain: true`）
  - 仅对 `contentType === 'text'` 的项有意义；对 image/files 项 ⌥Enter 行为与 Enter 相同（plain 参数后端对非 text 无副作用，直接透传即可，不要在前端加类型分支）
- 底部快捷键提示条（若存在 kbd 提示区）补 ⌥↵ 提示，仅在选中项为 text 时展示（避免噪音）

## 4. i18n（src/lib/i18n/index.svelte.ts）

⚠️ 此文件有未提交 WIP（source_app 相关字符串），**只追加键，不重排不覆盖**。
追加（zh-CN / en 双语）：
- `files`（"文件" / "Files"）
- `fileCount`（"{n} 个文件" / "{n} files"，插值方式跟随现有模式）
- `richTextBadge`（"富文本" / "Rich text"）
- `pastePlain`（"纯文本粘贴" / "Paste as plain text"）

## 5. store（src/lib/stores/clipboard.svelte.ts）

- 检查 ClipItem 映射处是否需要透传 `hasHtml`（大概率 camelCase 自动透传，确认类型即可）
- paste 调用封装追加可选 plain 参数

## 6. 测试与验证

- `bun test tests/` 全绿；如现有前端测试覆盖 store 的 paste 封装，补 plain 参数用例
- `bun run check`（svelte-check）0 error；`bun run lint` 通过；`bun run build` 成功
- 手动验证清单（写进报告，无法运行 app 就注明"待协调者验证"）：
  1. 复制 Finder 文件 → 列表出现文件项（图标+文件名+计数）
  2. 从浏览器复制富文本 → 文本项带富文本徽标
  3. 选中文本项按 ⌥Enter → 粘贴出纯文本

## 7. 明确不做

- 不渲染 HTML 预览、不做语法高亮（Wave 3 类型识别一并做）
- 不改 src-tauri 任何文件
- 不做"总是纯文本粘贴"设置项（后续）
- 不动设置页（除非 lint/check 强制）
