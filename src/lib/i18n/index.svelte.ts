export type Locale = 'zh-CN' | 'en';

const zh = {
  copiedOnly: '已复制，自动粘贴未完成，请在目标应用手动粘贴。',
  searchLimit: '仅显示前 1000 条结果，请缩小搜索范围。',
  previewToggle: '切换预览',
  actions: '条目操作',
  mergeImagesSkipped: '合并时跳过 {n} 张图片',
  capturePausedNotice: '剪贴板采集已暂停，可从托盘恢复。',
  ignoredAppsPlatform: '应用排除目前仅支持 macOS。',
  fullPreview: '展开预览',
  previewTruncated: '正在显示内容摘要',
  previewLimited: '仅预览前 1 MiB，复制和粘贴仍使用完整内容',
  toggleSelection: '选择 / 取消选择',
  searchPendingHint: '搜索尚未完成，请稍后再按回车。',
  appName: 'ClipMan',
  appTagline: '高效的剪贴板管理工具',

  history: '历史记录',
  pinned: '置顶',
  settings: '设置',

  copy: '复制',
  copied: '已复制',
  pin: '置顶',
  unpin: '取消置顶',
  editLabel: '编辑标签',
  delete: '删除',
  clear: '清除',
  clearNonPinned: '清除非置顶',
  save: '保存',
  saved: '已保存',
  saving: '保存中',
  reset: '重置',
  cancel: '取消',
  confirm: '确认',

  text: '文本',
  image: '图片',

  justNow: '刚刚',
  minutesAgo: '{n}分钟前',
  yesterday: '昨天',
  characterCount: '{n} 字符',

  noPinnedItems: '暂无置顶项目',
  noClipboardHistory: '暂无剪贴板历史',
  noSearchResults: '没有匹配的结果',

  searchPlaceholder: '搜索剪贴板内容...',

  settingsGeneral: '常规',
  settingsClipboard: '剪贴板',
  settingsAppearance: '外观',
  settingsTray: '托盘',
  settingsStorage: '存储',
  settingsAbout: '关于',

  autostart: '开机自启动',
  autostartDesc: '系统启动时自动运行 ClipMan',
  globalHotkey: '全局热键',
  globalHotkeyDesc: '设置打开 ClipMan 窗口的快捷键',
  recording: '录入',
  recordingHint: '按下快捷键组合...',
  alreadyCurrentHotkey: '这已经是当前快捷键了',
  commonHotkeys: '常用快捷键:',
  advancedManualInput: '高级：手动输入...',
  pinnedShortcut: '常用快捷键',
  pinnedShortcutDesc: '可选。设置后直接打开常用面板；留空则不绑定。',

  maxHistoryItems: '历史记录数量',
  maxHistoryItemsDesc: '保留的最大历史记录数量',
  autoPaste: '自动粘贴',
  autoPasteDesc: '从 QuickBar 取用时自动粘回当前应用；关闭后只复制。',
  pasteFormat: '粘贴格式',
  pasteFormatOriginal: '保留原始格式',
  pasteFormatOriginalDesc: '按复制时的原样粘贴，包含富文本格式。',
  pasteFormatTakePlain: '取用时纯文本',
  pasteFormatTakePlainDesc: '经 ClipMan 取用或复制时去除格式；直接 Cmd+V 仍粘贴原格式。',
  pasteFormatGlobalPlain: '始终纯文本',
  pasteFormatGlobalPlainDesc:
    '复制被捕获后即去除富文本，之后的直接粘贴即为纯文本（少数跳过捕获的内容除外）。',
  ignoreConcealed: '忽略密码类剪贴板',
  ignoreConcealedDesc: '跳过系统标记为密码、临时或不可记录的剪贴板内容。',

  themeMode: '主题模式',
  themeLight: '浅色',
  themeDark: '深色',
  themePink: '粉色',
  themeSystem: '跟随系统',
  language: '语言',

  trayTextLength: '托盘文本长度',
  trayTextLengthDesc: '托盘菜单中显示的文本最大长度',
  maxPinnedInTray: '托盘置顶数量',
  maxPinnedInTrayDesc: '托盘菜单中显示的置顶项数量',
  maxRecentInTray: '托盘最近数量',
  maxRecentInTrayDesc: '托盘菜单中显示的最近项数量',

  dataLocationDesc: '选择 ClipMan 数据的存储位置',
  currentLocation: '当前位置',
  changeLocation: '更改位置',
  openFolder: '打开文件夹',
  selectDataLocation: '选择新的数据存储位置',
  selectDirectoryFailed: '选择目录失败',

  confirmMigration: '确认迁移数据',
  migratingTo: '即将把数据迁移到:',
  deleteOldData: '迁移后删除原位置数据',
  startMigration: '开始迁移',
  migrationSuccess: '数据迁移成功！',
  migrationFailed: '迁移失败',

  version: '版本',
  checkUpdate: '检查更新',
  checking: '检查中',
  updateAvailable: '发现新版本',
  noUpdateAvailable: '当前已是最新版本',
  installUpdate: '安装更新',
  installing: '安装中',
  checkUpdateFailed: '检查更新失败',
  downloadingUpdate: '正在下载并安装更新...',
  updateInstalled: '更新安装成功，请重启应用',
  installUpdateFailed: '安装更新失败',
  releaseNotes: '更新内容',

  confirmClearHistory: '确定要清除所有非置顶的历史记录吗？',
  confirmResetSettings: '确定要重置所有设置吗？这将恢复默认配置。',

  loadSettingsFailed: '加载设置失败',
  saveSettingsFailed: '保存失败',
  copyFailed: '复制失败',
  pasteFailed: '粘贴失败',
  clearFailed: '清除失败',

  loading: '加载中...',

  emptyContent: '[内容为空]',

  clipboardAccessTitle: '无法访问剪贴板',
  clipboardAccessDesc: 'ClipMan 暂时无法读取系统剪贴板，新复制的内容可能不会被记录。',
  clipboardAccessHint: '请稍后重试；若持续出现，检查是否有其他剪贴板工具正在占用剪贴板。',
  recheck: '重新检查',
  details: '查看详情',
  errorLabel: '错误',

  accessibilityTitle: '需要无障碍权限',
  accessibilityDesc: 'ClipMan 无法自动粘贴（模拟 Cmd+V）：“辅助功能”权限已失效，常见于应用更新后。',
  accessibilityHint:
    '在系统设置中重新勾选 ClipMan 即可恢复自动粘贴；期间内容仍会复制到剪贴板，可手动粘贴。',
  openSettings: '打开系统设置',

  paste: '粘贴',
  slot: '槽位',
  switchPanel: '切换',
  close: '关闭',
  reorder: '排序',
  pastePlain: '纯文本粘贴',
  pasteRich: '富文本粘贴',

  files: '文件',
  fileCount: '{n} 个文件',
  richTextBadge: '富文本',

  ignoredApps: '忽略的应用',
  ignoredAppsDesc: '来自以下应用的复制内容不会被记录（按应用名匹配，不区分大小写）。',
  ignoredAppsPlaceholder: '输入应用名称，如 1Password',
  addIgnoredApp: '添加',
  removeIgnoredApp: '移除',
  noIgnoredApps: '暂无忽略的应用',

  skipSecrets: '自动跳过疑似密钥',
  skipSecretsDesc: '检测到私钥、API 密钥、JWT 等高置信度敏感内容时跳过采集。',

  maxTextBytes: '文本大小上限 (MB)',
  maxTextBytesDesc: '超过此大小的文本或文件路径列表将不会被记录。',
  maxImageDimension: '图片尺寸上限 (px)',
  maxImageDimensionDesc: '超过此边长的图片会被等比缩小后再保存；设为 0 可关闭缩放。',

  selectedCount: '已选 {n} 项',
  mergePasteHint: '合并粘贴（换行分隔）',
  multiSelectHint: '多选',
};

type Translations = typeof zh;

const en: Translations = {
  copiedOnly: 'Copied. Automatic paste was unavailable; paste manually in the target app.',
  searchLimit: 'Showing the first 1000 results. Refine your search.',
  previewToggle: 'Toggle preview',
  actions: 'Item actions',
  mergeImagesSkipped: 'Merge skips {n} image(s)',
  capturePausedNotice: 'Clipboard capture is paused. Resume it from the tray.',
  ignoredAppsPlatform: 'App exclusions are currently supported only on macOS.',
  fullPreview: 'Expand preview',
  previewTruncated: 'Showing a content excerpt',
  previewLimited: 'Preview limited to 1 MiB. Copy and paste use the full content.',
  toggleSelection: 'Select / deselect',
  searchPendingHint: 'Search is still running. Press Enter again when it finishes.',
  appName: 'ClipMan',
  appTagline: 'Efficient clipboard manager',

  history: 'History',
  pinned: 'Pinned',
  settings: 'Settings',

  copy: 'Copy',
  copied: 'Copied',
  pin: 'Pin',
  unpin: 'Unpin',
  editLabel: 'Edit label',
  delete: 'Delete',
  clear: 'Clear',
  clearNonPinned: 'Clear Non-pinned',
  save: 'Save',
  saved: 'Saved',
  saving: 'Saving',
  reset: 'Reset',
  cancel: 'Cancel',
  confirm: 'Confirm',

  text: 'Text',
  image: 'Image',

  justNow: 'Just now',
  minutesAgo: '{n}m ago',
  yesterday: 'Yesterday',
  characterCount: '{n} chars',

  noPinnedItems: 'No pinned items',
  noClipboardHistory: 'No clipboard history',
  noSearchResults: 'No matching results',

  searchPlaceholder: 'Search clipboard...',

  settingsGeneral: 'General',
  settingsClipboard: 'Clipboard',
  settingsAppearance: 'Appearance',
  settingsTray: 'Tray',
  settingsStorage: 'Storage',
  settingsAbout: 'About',

  autostart: 'Launch at startup',
  autostartDesc: 'Automatically start ClipMan when system boots',
  globalHotkey: 'Global hotkey',
  globalHotkeyDesc: 'Set the shortcut to open ClipMan window',
  recording: 'Record',
  recordingHint: 'Press key combination...',
  alreadyCurrentHotkey: 'This is already the current hotkey',
  commonHotkeys: 'Common hotkeys:',
  advancedManualInput: 'Advanced: Manual input...',
  pinnedShortcut: 'Pinned shortcut',
  pinnedShortcutDesc:
    'Optional. Opens QuickBar directly on the pinned panel; leave empty to disable.',

  maxHistoryItems: 'History limit',
  maxHistoryItemsDesc: 'Maximum number of history items to keep',
  autoPaste: 'Auto-paste',
  autoPasteDesc: 'Paste selected QuickBar items back into the current app; off means copy only.',
  pasteFormat: 'Paste format',
  pasteFormatOriginal: 'Keep original formatting',
  pasteFormatOriginalDesc: 'Paste exactly as copied, rich formatting included.',
  pasteFormatTakePlain: 'Plain text via ClipMan',
  pasteFormatTakePlainDesc:
    'Taking or copying through ClipMan strips formatting; a direct Cmd+V keeps the copied format.',
  pasteFormatGlobalPlain: 'Always plain text',
  pasteFormatGlobalPlainDesc:
    'Rich text is stripped once a copy is captured, so later direct pastes are plain text.',
  ignoreConcealed: 'Ignore concealed clipboard content',
  ignoreConcealedDesc:
    'Skip clipboard payloads marked as passwords, transient, or excluded from history.',

  themeMode: 'Theme',
  themeLight: 'Light',
  themeDark: 'Dark',
  themePink: 'Pink',
  themeSystem: 'System',
  language: 'Language',

  trayTextLength: 'Tray text length',
  trayTextLengthDesc: 'Maximum text length in tray menu',
  maxPinnedInTray: 'Pinned in tray',
  maxPinnedInTrayDesc: 'Number of pinned items shown in tray menu',
  maxRecentInTray: 'Recent in tray',
  maxRecentInTrayDesc: 'Number of recent items shown in tray menu',

  dataLocationDesc: 'Choose where ClipMan stores its data',
  currentLocation: 'Current location',
  changeLocation: 'Change location',
  openFolder: 'Open folder',
  selectDataLocation: 'Select new data location',
  selectDirectoryFailed: 'Failed to select directory',

  confirmMigration: 'Confirm data migration',
  migratingTo: 'Data will be migrated to:',
  deleteOldData: 'Delete data from old location after migration',
  startMigration: 'Start migration',
  migrationSuccess: 'Data migrated successfully!',
  migrationFailed: 'Migration failed',

  version: 'Version',
  checkUpdate: 'Check for updates',
  checking: 'Checking',
  updateAvailable: 'Update available',
  noUpdateAvailable: 'You are on the latest version',
  installUpdate: 'Install update',
  installing: 'Installing',
  checkUpdateFailed: 'Failed to check for updates',
  downloadingUpdate: 'Downloading and installing update...',
  updateInstalled: 'Update installed, please restart the app',
  installUpdateFailed: 'Failed to install update',
  releaseNotes: 'Release Notes',

  confirmClearHistory: 'Are you sure you want to clear all non-pinned history?',
  confirmResetSettings: 'Are you sure you want to reset all settings to defaults?',

  loadSettingsFailed: 'Failed to load settings',
  saveSettingsFailed: 'Failed to save',
  copyFailed: 'Copy failed',
  pasteFailed: 'Paste failed',
  clearFailed: 'Failed to clear',

  loading: 'Loading...',

  emptyContent: '[Empty content]',

  clipboardAccessTitle: 'Cannot access the clipboard',
  clipboardAccessDesc:
    'ClipMan cannot read the system clipboard right now, so new copies may not be recorded.',
  clipboardAccessHint:
    'Try again shortly. If it keeps happening, check whether another clipboard tool is holding the clipboard.',
  recheck: 'Re-check',
  details: 'Details',
  errorLabel: 'Error',

  accessibilityTitle: 'Accessibility permission needed',
  accessibilityDesc:
    "ClipMan can't auto-paste (simulate Cmd+V): its Accessibility permission is no longer valid, which often happens after an update.",
  accessibilityHint:
    'Re-enable ClipMan under System Settings to restore auto-paste. Until then, content is still copied to the clipboard so you can paste manually.',
  openSettings: 'Open System Settings',

  paste: 'Paste',
  slot: 'Slot',
  switchPanel: 'Switch',
  close: 'Close',
  reorder: 'Reorder',
  pastePlain: 'Paste as plain text',
  pasteRich: 'Paste rich text',

  files: 'Files',
  fileCount: '{n} files',
  richTextBadge: 'Rich text',

  ignoredApps: 'Ignored apps',
  ignoredAppsDesc:
    "Copies from these apps won't be recorded (matched by app name, case-insensitive).",
  ignoredAppsPlaceholder: 'App name, e.g. 1Password',
  addIgnoredApp: 'Add',
  removeIgnoredApp: 'Remove',
  noIgnoredApps: 'No ignored apps',

  skipSecrets: 'Skip likely secrets',
  skipSecretsDesc:
    'Skip capturing high-confidence secrets such as private keys, API tokens, or JWTs.',

  maxTextBytes: 'Max text size (MB)',
  maxTextBytesDesc: 'Text or file path lists larger than this are not recorded.',
  maxImageDimension: 'Max image dimension (px)',
  maxImageDimensionDesc:
    'Images larger than this on their longest side are downscaled before saving; set to 0 to disable.',

  selectedCount: '{n} selected',
  mergePasteHint: 'Merge paste (newlines)',
  multiSelectHint: 'Multi-select',
};

const translations: Record<Locale, Translations> = { 'zh-CN': zh, en };

class I18n {
  private _locale = $state<Locale>('zh-CN');

  get locale() {
    return this._locale;
  }

  get t(): Translations {
    return translations[this._locale];
  }

  setLocale(locale: Locale) {
    this._locale = locale;
  }

  format(template: string, params: Record<string, string | number>): string {
    let result = template;
    for (const [key, value] of Object.entries(params)) {
      result = result.replace(`{${key}}`, String(value));
    }
    return result;
  }
}

export const i18n = new I18n();
