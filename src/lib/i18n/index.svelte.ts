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
  delete: string;
  clear: string;
  clearNonPinned: string;
  save: string;
  saving: string;
  reset: string;
  cancel: string;
  confirm: string;
  quit: string;
  
  // Content types
  text: string;
  image: string;
  file: string;
  filePath: string;
  richText: string;
  
  // Time
  justNow: string;
  minutesAgo: string;
  hoursAgo: string;
  
  // Empty states
  noPinnedItems: string;
  noPinnedItemsHint: string;
  noClipboardHistory: string;
  noClipboardHistoryHint: string;
  
  // Search
  searchPlaceholder: string;
  showing: string;
  items: string;
  
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
  
  // Clipboard settings
  maxHistoryItems: string;
  maxHistoryItemsDesc: string;
  storeOriginalImage: string;
  storeOriginalImageDesc: string;
  
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
  
  // Migration dialog
  confirmMigration: string;
  migratingTo: string;
  deleteOldData: string;
  startMigration: string;
  
  // About
  version: string;
  checkUpdate: string;
  checking: string;
  updateAvailable: string;
  noUpdateAvailable: string;
  installUpdate: string;
  installing: string;
  
  // Notifications
  copiedToClipboard: string;
  textCopied: string;
  imageCopied: string;
  filePathCopied: string;
  richTextCopied: string;
  
  // Confirmations
  confirmClearHistory: string;
  confirmResetSettings: string;
  
  // Errors
  loadSettingsFailed: string;
  saveSettingsFailed: string;
  copyFailed: string;
  
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
  binaryFileData: string;
  decodeFailed: string;
  emptyContent: string;
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
  delete: '删除',
  clear: '清除',
  clearNonPinned: '清除非置顶',
  save: '保存',
  saving: '保存中',
  reset: '重置',
  cancel: '取消',
  confirm: '确认',
  quit: '退出',
  
  // Content types
  text: '文本',
  image: '图片',
  file: '文件',
  filePath: '文件路径',
  richText: '富文本',
  
  // Time
  justNow: '刚刚',
  minutesAgo: '{n}分钟前',
  hoursAgo: '{n}小时前',
  
  // Empty states
  noPinnedItems: '暂无置顶项目',
  noPinnedItemsHint: '点击置顶图标收藏常用内容',
  noClipboardHistory: '暂无剪切板历史',
  noClipboardHistoryHint: '复制内容后会自动出现在这里',
  
  // Search
  searchPlaceholder: '搜索剪切板内容...',
  showing: '显示',
  items: '项',
  
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
  
  // Clipboard settings
  maxHistoryItems: '历史记录数量',
  maxHistoryItemsDesc: '保留的最大历史记录数量',
  storeOriginalImage: '存储原图',
  storeOriginalImageDesc: '保存原始图片而非缩略图（占用更多空间）',
  
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
  
  // Migration dialog
  confirmMigration: '确认迁移数据',
  migratingTo: '即将把数据迁移到:',
  deleteOldData: '迁移后删除原位置数据',
  startMigration: '开始迁移',
  
  // About
  version: '版本',
  checkUpdate: '检查更新',
  checking: '检查中',
  updateAvailable: '发现新版本',
  noUpdateAvailable: '当前已是最新版本',
  installUpdate: '安装更新',
  installing: '安装中',
  
  // Notifications
  copiedToClipboard: '已复制到剪贴板',
  textCopied: '文本已复制到剪贴板',
  imageCopied: '图片已复制到剪贴板',
  filePathCopied: '文件路径已复制到剪贴板',
  richTextCopied: '富文本已复制到剪贴板',
  
  // Confirmations
  confirmClearHistory: '确定要清除所有非置顶的历史记录吗？',
  confirmResetSettings: '确定要重置所有设置吗？这将恢复默认配置。',
  
  // Errors
  loadSettingsFailed: '加载设置失败',
  saveSettingsFailed: '保存失败',
  copyFailed: '复制失败',
  
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
  binaryFileData: '[二进制文件数据]',
  decodeFailed: '[解码失败]',
  emptyContent: '[内容为空]',
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
  delete: 'Delete',
  clear: 'Clear',
  clearNonPinned: 'Clear Non-pinned',
  save: 'Save',
  saving: 'Saving',
  reset: 'Reset',
  cancel: 'Cancel',
  confirm: 'Confirm',
  quit: 'Quit',
  
  // Content types
  text: 'Text',
  image: 'Image',
  file: 'File',
  filePath: 'File path',
  richText: 'Rich text',
  
  // Time
  justNow: 'Just now',
  minutesAgo: '{n}m ago',
  hoursAgo: '{n}h ago',
  
  // Empty states
  noPinnedItems: 'No pinned items',
  noPinnedItemsHint: 'Click the pin icon to save frequently used content',
  noClipboardHistory: 'No clipboard history',
  noClipboardHistoryHint: 'Copied content will appear here',
  
  // Search
  searchPlaceholder: 'Search clipboard...',
  showing: 'Showing',
  items: 'items',
  
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
  
  // Clipboard settings
  maxHistoryItems: 'History limit',
  maxHistoryItemsDesc: 'Maximum number of history items to keep',
  storeOriginalImage: 'Store original images',
  storeOriginalImageDesc: 'Save original images instead of thumbnails (uses more space)',
  
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
  
  // Migration dialog
  confirmMigration: 'Confirm data migration',
  migratingTo: 'Data will be migrated to:',
  deleteOldData: 'Delete data from old location after migration',
  startMigration: 'Start migration',
  
  // About
  version: 'Version',
  checkUpdate: 'Check for updates',
  checking: 'Checking',
  updateAvailable: 'Update available',
  noUpdateAvailable: 'You are on the latest version',
  installUpdate: 'Install update',
  installing: 'Installing',
  
  // Notifications
  copiedToClipboard: 'Copied to clipboard',
  textCopied: 'Text copied to clipboard',
  imageCopied: 'Image copied to clipboard',
  filePathCopied: 'File path copied to clipboard',
  richTextCopied: 'Rich text copied to clipboard',
  
  // Confirmations
  confirmClearHistory: 'Are you sure you want to clear all non-pinned history?',
  confirmResetSettings: 'Are you sure you want to reset all settings to defaults?',
  
  // Errors
  loadSettingsFailed: 'Failed to load settings',
  saveSettingsFailed: 'Failed to save',
  copyFailed: 'Copy failed',
  
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
  binaryFileData: '[Binary file data]',
  decodeFailed: '[Decode failed]',
  emptyContent: '[Empty content]',
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
    // Notify backend to update tray menu
    this.syncToBackend(locale);
  }
  
  // Sync locale to backend for tray menu i18n
  private async syncToBackend(locale: Locale) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      // Get current settings
      const settings = await invoke('get_settings') as Record<string, unknown>;
      // Update locale
      settings.locale = locale;
      // Save back
      await invoke('update_settings', { settings });
    } catch (e) {
      console.warn('Failed to sync locale to backend:', e);
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
