// Internationalization (i18n) module for ClipMan

export type Locale = 'zh-CN' | 'en';

export interface Translations {
  // App
  appName: string;

  // Navigation
  history: string;
  pinned: string;
  settings: string;

  // Actions
  copy: string;
  copied: string;
  pin: string;
  unpin: string;
  editLabel: string;
  labelPlaceholder: string;
  delete: string;
  clear: string;
  clearNonPinned: string;
  save: string;
  saved: string;
  saving: string;
  reset: string;
  cancel: string;
  confirm: string;
  quit: string;

  // Content types
  text: string;
  image: string;

  // Time
  justNow: string;
  minutesAgo: string;
  hoursAgo: string;

  // Source
  source: string;

  // Empty states
  noPinnedItems: string;
  noPinnedItemsHint: string;
  noClipboardHistory: string;
  noClipboardHistoryHint: string;
  noSearchResults: string;
  noSearchResultsHint: string;
  noPinnedMatches: string;
  noPinnedMatchesHint: string;

  // Search
  searchPlaceholder: string;
  showing: string;
  items: string;

  // Preview pane
  selectToPreview: string;
  charCount: string;

  // Settings sections
  settingsGeneral: string;
  settingsClipboard: string;
  settingsAppearance: string;
  settingsTray: string;
  settingsStorage: string;
  settingsAbout: string;

  // General settings
  autostart: string;
  autostartDesc: string;
  globalHotkey: string;
  globalHotkeyDesc: string;
  recording: string;
  recordingHint: string;
  alreadyCurrentHotkey: string;
  commonHotkeys: string;
  advancedManualInput: string;
  pinnedShortcut: string;
  pinnedShortcutDesc: string;

  // Clipboard settings
  maxHistoryItems: string;
  maxHistoryItemsDesc: string;
  autoPaste: string;
  autoPasteDesc: string;
  ignoreConcealed: string;
  ignoreConcealedDesc: string;

  // Appearance settings
  themeMode: string;
  themeLight: string;
  themeDark: string;
  themePink: string;
  themeSystem: string;
  language: string;

  // Tray settings
  trayTextLength: string;
  trayTextLengthDesc: string;
  maxPinnedInTray: string;
  maxPinnedInTrayDesc: string;
  maxRecentInTray: string;
  maxRecentInTrayDesc: string;

  // Storage settings
  dataLocation: string;
  dataLocationDesc: string;
  currentLocation: string;
  changeLocation: string;
  openFolder: string;
  selectDataLocation: string;
  selectDirectoryFailed: string;

  // Migration dialog
  confirmMigration: string;
  migratingTo: string;
  deleteOldData: string;
  startMigration: string;
  migrationSuccess: string;
  migrationFailed: string;

  // About
  version: string;
  checkUpdate: string;
  checking: string;
  updateAvailable: string;
  noUpdateAvailable: string;
  installUpdate: string;
  installing: string;
  checkUpdateFailed: string;
  downloadingUpdate: string;
  updateInstalled: string;
  installUpdateFailed: string;

  // Confirmations
  confirmClearHistory: string;
  confirmResetSettings: string;

  // Errors
  loadSettingsFailed: string;
  saveSettingsFailed: string;
  copyFailed: string;
  clearFailed: string;

  // Statistics
  statistics: string;
  total: string;

  // Tray menu
  trayPinnedHeader: string;
  trayRecentHeader: string;

  // Loading
  loading: string;

  // Misc
  switchTheme: string;
  decodeFailed: string;
  emptyContent: string;
  checkedTask: string;
  uncheckedTask: string;

  // Clipboard permission
  clipboardAccessTitle: string;
  clipboardAccessDesc: string;
  clipboardAccessHint: string;
  recheck: string;
  details: string;
  errorLabel: string;

  // Accessibility permission (macOS)
  accessibilityTitle: string;
  accessibilityDesc: string;
  accessibilityHint: string;
  openSettings: string;

  // QuickBar footer hints
  paste: string;
  slot: string;
  switchPanel: string;
  close: string;
  reorder: string;
  pastePlain: string;

  // Files / rich text
  files: string;
  fileCount: string;
  richTextBadge: string;

  // Ignored apps (SPEC-4 §3)
  ignoredApps: string;
  ignoredAppsDesc: string;
  ignoredAppsPlaceholder: string;
  addIgnoredApp: string;
  removeIgnoredApp: string;
  noIgnoredApps: string;

  // Secret detection setting (SPEC-4 §2 UI)
  skipSecrets: string;
  skipSecretsDesc: string;

  // Capture size limits (SPEC-3 §5 UI)
  maxTextBytes: string;
  maxTextBytesDesc: string;
  maxImageDimension: string;
  maxImageDimensionDesc: string;

  // Multi-select merge paste
  selectedCount: string;
  mergePasteHint: string;
  clearSelection: string;
}

const zh: Translations = {
  // App
  appName: 'ClipMan',

  // Navigation
  history: '历史记录',
  pinned: '置顶',
  settings: '设置',

  // Actions
  copy: '复制',
  copied: '已复制',
  pin: '置顶',
  unpin: '取消置顶',
  editLabel: '编辑标签',
  labelPlaceholder: '输入常用项标签',
  delete: '删除',
  clear: '清除',
  clearNonPinned: '清除非置顶',
  save: '保存',
  saved: '已保存',
  saving: '保存中',
  reset: '重置',
  cancel: '取消',
  confirm: '确认',
  quit: '退出',

  // Content types
  text: '文本',
  image: '图片',

  // Time
  justNow: '刚刚',
  minutesAgo: '{n}分钟前',
  hoursAgo: '{n}小时前',

  source: '来源',

  // Empty states
  noPinnedItems: '暂无置顶项目',
  noPinnedItemsHint: '点击置顶图标收藏常用内容',
  noClipboardHistory: '暂无剪切板历史',
  noClipboardHistoryHint: '复制内容后会自动出现在这里',
  noSearchResults: '没有匹配的结果',
  noSearchResultsHint: '换个关键词试试',
  noPinnedMatches: '没有匹配的置顶项',
  noPinnedMatchesHint: '在「历史」中查看 {n} 条匹配',

  // Search
  searchPlaceholder: '搜索剪切板内容...',
  showing: '显示',
  items: '项',

  // Preview pane
  selectToPreview: '选择一项查看完整内容',
  charCount: '{n} 字',

  // Settings sections
  settingsGeneral: '常规',
  settingsClipboard: '剪切板',
  settingsAppearance: '外观',
  settingsTray: '托盘',
  settingsStorage: '存储',
  settingsAbout: '关于',

  // General settings
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

  // Clipboard settings
  maxHistoryItems: '历史记录数量',
  maxHistoryItemsDesc: '保留的最大历史记录数量',
  autoPaste: '自动粘贴',
  autoPasteDesc: '从 QuickBar 取用时自动粘回当前应用；关闭后只复制。',
  ignoreConcealed: '忽略密码类剪贴板',
  ignoreConcealedDesc: '跳过系统标记为密码、临时或不可记录的剪贴板内容。',

  // Appearance settings
  themeMode: '主题模式',
  themeLight: '浅色',
  themeDark: '深色',
  themePink: '粉色',
  themeSystem: '跟随系统',
  language: '语言',

  // Tray settings
  trayTextLength: '托盘文本长度',
  trayTextLengthDesc: '托盘菜单中显示的文本最大长度',
  maxPinnedInTray: '托盘置顶数量',
  maxPinnedInTrayDesc: '托盘菜单中显示的置顶项数量',
  maxRecentInTray: '托盘最近数量',
  maxRecentInTrayDesc: '托盘菜单中显示的最近项数量',

  // Storage settings
  dataLocation: '数据存储位置',
  dataLocationDesc: '选择 ClipMan 数据的存储位置',
  currentLocation: '当前位置',
  changeLocation: '更改位置',
  openFolder: '打开文件夹',
  selectDataLocation: '选择新的数据存储位置',
  selectDirectoryFailed: '选择目录失败',

  // Migration dialog
  confirmMigration: '确认迁移数据',
  migratingTo: '即将把数据迁移到:',
  deleteOldData: '迁移后删除原位置数据',
  startMigration: '开始迁移',
  migrationSuccess: '数据迁移成功！',
  migrationFailed: '迁移失败',

  // About
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

  // Confirmations
  confirmClearHistory: '确定要清除所有非置顶的历史记录吗？',
  confirmResetSettings: '确定要重置所有设置吗？这将恢复默认配置。',

  // Errors
  loadSettingsFailed: '加载设置失败',
  saveSettingsFailed: '保存失败',
  copyFailed: '复制失败',
  clearFailed: '清除失败',

  // Statistics
  statistics: '统计信息',
  total: '总计',

  // Tray menu
  trayPinnedHeader: '置顶项',
  trayRecentHeader: '最近复制',

  // Loading
  loading: '加载中...',

  // Misc
  switchTheme: '切换主题',
  decodeFailed: '[解码失败]',
  emptyContent: '[内容为空]',
  checkedTask: '已完成任务',
  uncheckedTask: '未完成任务',

  // Clipboard permission
  clipboardAccessTitle: '无法访问剪贴板',
  clipboardAccessDesc: 'ClipMan 暂时无法读取系统剪贴板，新复制的内容可能不会被记录。',
  clipboardAccessHint: '请稍后重试；若持续出现，检查是否有其他剪贴板工具正在占用剪贴板。',
  recheck: '重新检查',
  details: '查看详情',
  errorLabel: '错误',

  // Accessibility permission (macOS)
  accessibilityTitle: '需要无障碍权限',
  accessibilityDesc: 'ClipMan 无法自动粘贴（模拟 Cmd+V）：“辅助功能”权限已失效，常见于应用更新后。',
  accessibilityHint:
    '在系统设置中重新勾选 ClipMan 即可恢复自动粘贴；期间内容仍会复制到剪贴板，可手动粘贴。',
  openSettings: '打开系统设置',

  // QuickBar footer hints
  paste: '粘贴',
  slot: '槽位',
  switchPanel: '切换',
  close: '关闭',
  reorder: '排序',
  pastePlain: '纯文本粘贴',

  // Files / rich text
  files: '文件',
  fileCount: '{n} 个文件',
  richTextBadge: '富文本',

  // Ignored apps
  ignoredApps: '忽略的应用',
  ignoredAppsDesc: '来自以下应用的复制内容不会被记录（按应用名匹配，不区分大小写）。',
  ignoredAppsPlaceholder: '输入应用名称，如 1Password',
  addIgnoredApp: '添加',
  removeIgnoredApp: '移除',
  noIgnoredApps: '暂无忽略的应用',

  // Secret detection
  skipSecrets: '自动跳过疑似密钥',
  skipSecretsDesc: '检测到私钥、API 密钥、JWT 等高置信度敏感内容时跳过采集。',

  // Capture size limits
  maxTextBytes: '文本大小上限 (MB)',
  maxTextBytesDesc: '超过此大小的文本或文件路径列表将不会被记录。',
  maxImageDimension: '图片尺寸上限 (px)',
  maxImageDimensionDesc: '超过此边长的图片会被等比缩小后再保存；设为 0 可关闭缩放。',

  // Multi-select merge paste
  selectedCount: '已选 {n} 项',
  mergePasteHint: '合并粘贴（换行分隔）',
  clearSelection: '清除选择',
};

const en: Translations = {
  // App
  appName: 'ClipMan',

  // Navigation
  history: 'History',
  pinned: 'Pinned',
  settings: 'Settings',

  // Actions
  copy: 'Copy',
  copied: 'Copied',
  pin: 'Pin',
  unpin: 'Unpin',
  editLabel: 'Edit label',
  labelPlaceholder: 'Enter pinned label',
  delete: 'Delete',
  clear: 'Clear',
  clearNonPinned: 'Clear Non-pinned',
  save: 'Save',
  saved: 'Saved',
  saving: 'Saving',
  reset: 'Reset',
  cancel: 'Cancel',
  confirm: 'Confirm',
  quit: 'Quit',

  // Content types
  text: 'Text',
  image: 'Image',

  // Time
  justNow: 'Just now',
  minutesAgo: '{n}m ago',
  hoursAgo: '{n}h ago',

  source: 'From',

  // Empty states
  noPinnedItems: 'No pinned items',
  noPinnedItemsHint: 'Click the pin icon to save frequently used content',
  noClipboardHistory: 'No clipboard history',
  noClipboardHistoryHint: 'Copied content will appear here',
  noSearchResults: 'No matching results',
  noSearchResultsHint: 'Try a different keyword',
  noPinnedMatches: 'No matching pinned items',
  noPinnedMatchesHint: 'View {n} matches in History',

  // Search
  searchPlaceholder: 'Search clipboard...',
  showing: 'Showing',
  items: 'items',

  // Preview pane
  selectToPreview: 'Select an item to see the full content',
  charCount: '{n} chars',

  // Settings sections
  settingsGeneral: 'General',
  settingsClipboard: 'Clipboard',
  settingsAppearance: 'Appearance',
  settingsTray: 'Tray',
  settingsStorage: 'Storage',
  settingsAbout: 'About',

  // General settings
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

  // Clipboard settings
  maxHistoryItems: 'History limit',
  maxHistoryItemsDesc: 'Maximum number of history items to keep',
  autoPaste: 'Auto-paste',
  autoPasteDesc: 'Paste selected QuickBar items back into the current app; off means copy only.',
  ignoreConcealed: 'Ignore concealed clipboard content',
  ignoreConcealedDesc:
    'Skip clipboard payloads marked as passwords, transient, or excluded from history.',

  // Appearance settings
  themeMode: 'Theme',
  themeLight: 'Light',
  themeDark: 'Dark',
  themePink: 'Pink',
  themeSystem: 'System',
  language: 'Language',

  // Tray settings
  trayTextLength: 'Tray text length',
  trayTextLengthDesc: 'Maximum text length in tray menu',
  maxPinnedInTray: 'Pinned in tray',
  maxPinnedInTrayDesc: 'Number of pinned items shown in tray menu',
  maxRecentInTray: 'Recent in tray',
  maxRecentInTrayDesc: 'Number of recent items shown in tray menu',

  // Storage settings
  dataLocation: 'Data location',
  dataLocationDesc: 'Choose where ClipMan stores its data',
  currentLocation: 'Current location',
  changeLocation: 'Change location',
  openFolder: 'Open folder',
  selectDataLocation: 'Select new data location',
  selectDirectoryFailed: 'Failed to select directory',

  // Migration dialog
  confirmMigration: 'Confirm data migration',
  migratingTo: 'Data will be migrated to:',
  deleteOldData: 'Delete data from old location after migration',
  startMigration: 'Start migration',
  migrationSuccess: 'Data migrated successfully!',
  migrationFailed: 'Migration failed',

  // About
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

  // Confirmations
  confirmClearHistory: 'Are you sure you want to clear all non-pinned history?',
  confirmResetSettings: 'Are you sure you want to reset all settings to defaults?',

  // Errors
  loadSettingsFailed: 'Failed to load settings',
  saveSettingsFailed: 'Failed to save',
  copyFailed: 'Copy failed',
  clearFailed: 'Failed to clear',

  // Statistics
  statistics: 'Statistics',
  total: 'Total',

  // Tray menu
  trayPinnedHeader: 'Pinned',
  trayRecentHeader: 'Recent',

  // Loading
  loading: 'Loading...',

  // Misc
  switchTheme: 'Switch theme',
  decodeFailed: '[Decode failed]',
  emptyContent: '[Empty content]',
  checkedTask: 'Checked task',
  uncheckedTask: 'Unchecked task',

  // Clipboard permission
  clipboardAccessTitle: 'Cannot access the clipboard',
  clipboardAccessDesc:
    'ClipMan cannot read the system clipboard right now, so new copies may not be recorded.',
  clipboardAccessHint:
    'Try again shortly. If it keeps happening, check whether another clipboard tool is holding the clipboard.',
  recheck: 'Re-check',
  details: 'Details',
  errorLabel: 'Error',

  // Accessibility permission (macOS)
  accessibilityTitle: 'Accessibility permission needed',
  accessibilityDesc:
    "ClipMan can't auto-paste (simulate Cmd+V): its Accessibility permission is no longer valid, which often happens after an update.",
  accessibilityHint:
    'Re-enable ClipMan under System Settings to restore auto-paste. Until then, content is still copied to the clipboard so you can paste manually.',
  openSettings: 'Open System Settings',

  // QuickBar footer hints
  paste: 'Paste',
  slot: 'Slot',
  switchPanel: 'Switch',
  close: 'Close',
  reorder: 'Reorder',
  pastePlain: 'Paste as plain text',

  // Files / rich text
  files: 'Files',
  fileCount: '{n} files',
  richTextBadge: 'Rich text',

  // Ignored apps
  ignoredApps: 'Ignored apps',
  ignoredAppsDesc:
    "Copies from these apps won't be recorded (matched by app name, case-insensitive).",
  ignoredAppsPlaceholder: 'App name, e.g. 1Password',
  addIgnoredApp: 'Add',
  removeIgnoredApp: 'Remove',
  noIgnoredApps: 'No ignored apps',

  // Secret detection
  skipSecrets: 'Skip likely secrets',
  skipSecretsDesc:
    'Skip capturing high-confidence secrets such as private keys, API tokens, or JWTs.',

  // Capture size limits
  maxTextBytes: 'Max text size (MB)',
  maxTextBytesDesc: 'Text or file path lists larger than this are not recorded.',
  maxImageDimension: 'Max image dimension (px)',
  maxImageDimensionDesc:
    'Images larger than this on their longest side are downscaled before saving; set to 0 to disable.',

  // Multi-select merge paste
  selectedCount: '{n} selected',
  mergePasteHint: 'Merge paste (newlines)',
  clearSelection: 'Clear selection',
};

const translations: Record<Locale, Translations> = { 'zh-CN': zh, en };

class I18n {
  private _locale = $state<Locale>('zh-CN');

  constructor() {
    // Detect system locale
    if (typeof navigator !== 'undefined') {
      const systemLang = navigator.language;
      if (systemLang.startsWith('zh')) {
        this._locale = 'zh-CN';
      } else {
        this._locale = 'en';
      }

      // Load saved preference
      const saved = localStorage.getItem('locale') as Locale | null;
      if (saved && (saved === 'zh-CN' || saved === 'en')) {
        this._locale = saved;
      }

      // Sync locale across windows via storage events
      window.addEventListener('storage', (event) => {
        if (event.key === 'locale' && event.newValue) {
          if (event.newValue === 'zh-CN' || event.newValue === 'en') {
            this._locale = event.newValue as Locale;
          }
        }
      });
    }
  }

  get locale() {
    return this._locale;
  }

  get t(): Translations {
    return translations[this._locale];
  }

  setLocale(locale: Locale) {
    this._locale = locale;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('locale', locale);
    }
  }

  // Helper for interpolation: t.minutesAgo with {n} -> "5分钟前"
  format(template: string, params: Record<string, string | number>): string {
    let result = template;
    for (const [key, value] of Object.entries(params)) {
      result = result.replace(`{${key}}`, String(value));
    }
    return result;
  }
}

export const i18n = new I18n();
