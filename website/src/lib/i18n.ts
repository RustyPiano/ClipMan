export type Locale = 'zh' | 'en';
export type OsKind = 'mac-arm' | 'mac-x64' | 'win' | 'linux';
export type ClipCategory = 'all' | 'text' | 'image' | 'file' | 'link';
export type SandboxClipKind = 'color' | 'text' | 'image' | 'file' | 'link';
export type PlainModeId = 'global' | 'quickbar' | 'alt';

export interface SandboxClip {
  id: string;
  kind: SandboxClipKind;
  title: string;
  preview: string;
  content: string;
  sourceApp: string;
  timestamp: string;
  favorite: boolean;
  richHtml?: boolean;
  colorHex?: string;
  colorRgb?: string;
  colorHsl?: string;
  imageMeta?: { width: number; height: number; size: string };
  files?: string[];
  url?: string;
}

export const RELEASE_VERSION = __RELEASE_VERSION__;
export const RELEASE_TAG = `v${RELEASE_VERSION}`;
export const REPO_URL = 'https://github.com/RustyPiano/ClipLeaf';
export const RELEASES_URL = 'https://github.com/RustyPiano/ClipLeaf/releases/latest';
const RELEASE_DOWNLOAD_BASE = `${REPO_URL}/releases/download/${RELEASE_TAG}`;

export const DOWNLOADS: Record<
  OsKind,
  { file: string; url: string; labelZh: string; labelEn: string; metaZh: string; metaEn: string }
> = {
  'mac-arm': {
    file: `ClipLeaf_${RELEASE_VERSION}_aarch64.dmg`,
    url: `${RELEASE_DOWNLOAD_BASE}/ClipLeaf_${RELEASE_VERSION}_aarch64.dmg`,
    labelZh: 'macOS · Apple Silicon',
    labelEn: 'macOS · Apple Silicon',
    metaZh: 'M 系列芯片 · macOS 13.3+',
    metaEn: 'Apple M-series · macOS 13.3+',
  },
  'mac-x64': {
    file: `ClipLeaf_${RELEASE_VERSION}_x64.dmg`,
    url: `${RELEASE_DOWNLOAD_BASE}/ClipLeaf_${RELEASE_VERSION}_x64.dmg`,
    labelZh: 'macOS · Intel',
    labelEn: 'macOS · Intel',
    metaZh: 'x86_64 · macOS 13.3+',
    metaEn: 'Intel Mac · macOS 13.3+',
  },
  win: {
    file: `ClipLeaf_${RELEASE_VERSION}_x64_en-US.msi`,
    url: `${RELEASE_DOWNLOAD_BASE}/ClipLeaf_${RELEASE_VERSION}_x64_en-US.msi`,
    labelZh: 'Windows · 64-bit',
    labelEn: 'Windows · 64-bit',
    metaZh: 'Windows 10 / 11 · MSI',
    metaEn: 'Windows 10 / 11 · MSI',
  },
  linux: {
    file: `ClipLeaf_${RELEASE_VERSION}_amd64.AppImage`,
    url: `${RELEASE_DOWNLOAD_BASE}/ClipLeaf_${RELEASE_VERSION}_amd64.AppImage`,
    labelZh: 'Linux · AppImage',
    labelEn: 'Linux · AppImage',
    metaZh: 'x86_64 · X11 / XWayland',
    metaEn: 'x86_64 · X11 / XWayland',
  },
};

export function detectOs(): OsKind | null {
  if (typeof navigator === 'undefined') return null;
  const ua = navigator.userAgent.toLowerCase();
  if (/iphone|ipad|ipod|android|mobile/.test(ua)) return null;
  if (ua.includes('windows nt')) return 'win';
  if (ua.includes('linux') && (ua.includes('x86_64') || ua.includes('amd64'))) return 'linux';
  return null;
}

export function detectLocale(): Locale {
  if (typeof navigator === 'undefined') return 'zh';
  return navigator.language.toLowerCase().startsWith('zh') ? 'zh' : 'en';
}

export function createInitialClips(locale: Locale): SandboxClip[] {
  if (locale === 'en') {
    return [
      {
        id: 'c1',
        kind: 'color',
        title: '#9F3C5C',
        preview: 'Petal Rose · Design token',
        content: '#9F3C5C',
        sourceApp: 'Figma',
        timestamp: 'Now',
        favorite: true,
        colorHex: '#9F3C5C',
        colorRgb: 'rgb(159, 60, 92)',
        colorHsl: 'hsl(341, 45%, 43%)',
      },
      {
        id: 'c2',
        kind: 'text',
        title: 'SQLite FTS5 trigram query',
        preview: 'SELECT id, content FROM clips_fts WHERE clips_fts MATCH ? ORDER BY created_at DESC;',
        content:
          'SELECT id, content, created_at\nFROM clips_fts\nWHERE clips_fts MATCH ?\nORDER BY created_at DESC\nLIMIT 100;',
        sourceApp: 'Zed',
        timestamp: '2m',
        favorite: true,
        richHtml: true,
      },
      {
        id: 'c3',
        kind: 'image',
        title: 'clipleaf-mark.png',
        preview: '1024 × 1024 · Transparent PNG · 184 KB',
        content: 'clipleaf-mark.png',
        sourceApp: 'Preview',
        timestamp: '9m',
        favorite: false,
        imageMeta: { width: 1024, height: 1024, size: '184 KB' },
      },
      {
        id: 'c4',
        kind: 'text',
        title: 'Release note prompt',
        preview: 'Summarize the git diff into crisp user-facing highlights.',
        content:
          'Summarize the git diff into 3–5 crisp user-facing highlights.\nStay factual, avoid hype, and list breaking changes first.',
        sourceApp: 'Notes',
        timestamp: '18m',
        favorite: true,
      },
      {
        id: 'c5',
        kind: 'link',
        title: 'https://clipleaf.app',
        preview: 'Local-first clipboard manager · Custom local storage directory',
        content: 'https://clipleaf.app',
        sourceApp: 'Arc',
        timestamp: '42m',
        favorite: false,
        url: 'https://clipleaf.app',
      },
      {
        id: 'c6',
        kind: 'file',
        title: '3 files · brand-kit/',
        preview: 'clipleaf-mark.svg, type-scale.pdf, deck.key',
        content: '/Users/alex/Design/clipleaf-mark.svg',
        sourceApp: 'Finder',
        timestamp: '1h',
        favorite: false,
        files: [
          '/Users/alex/Design/clipleaf-mark.svg',
          '/Users/alex/Design/type-scale.pdf',
          '/Users/alex/Design/deck.key',
        ],
      },
    ];
  }

  return [
    {
      id: 'c1',
      kind: 'color',
      title: '#9F3C5C',
      preview: '落樱梅红 · 主题主色值',
      content: '#9F3C5C',
      sourceApp: 'Figma',
      timestamp: '刚刚',
      favorite: true,
      colorHex: '#9F3C5C',
      colorRgb: 'rgb(159, 60, 92)',
      colorHsl: 'hsl(341, 45%, 43%)',
    },
    {
      id: 'c2',
      kind: 'text',
      title: 'SQLite FTS5 全文检索',
      preview: 'SELECT id, content FROM clips_fts WHERE clips_fts MATCH ? ORDER BY created_at DESC;',
      content:
        'SELECT id, content, created_at\nFROM clips_fts\nWHERE clips_fts MATCH ?\nORDER BY created_at DESC\nLIMIT 100;',
      sourceApp: 'Zed',
      timestamp: '2 分钟',
      favorite: true,
      richHtml: true,
    },
    {
      id: 'c3',
      kind: 'image',
      title: 'clipleaf-mark.png',
      preview: '1024 × 1024 · 透明通道 PNG · 184 KB',
      content: 'clipleaf-mark.png',
      sourceApp: '预览',
      timestamp: '9 分钟',
      favorite: false,
      imageMeta: { width: 1024, height: 1024, size: '184 KB' },
    },
    {
      id: 'c4',
      kind: 'text',
      title: '发版日志 Prompt',
      preview: '把下面的 git diff 整理成克制、准确的更新说明。',
      content:
        '请把下面的 git diff 整理成 3–5 条面向用户的更新说明：\n1. 先写可感知的体验变化\n2. 保持克制简洁\n3. 破坏性变更置顶',
      sourceApp: '备忘录',
      timestamp: '18 分钟',
      favorite: true,
    },
    {
      id: 'c5',
      kind: 'link',
      title: 'https://clipleaf.app',
      preview: '本地优先跨平台剪贴板 · 支持自定义本机存储目录',
      content: 'https://clipleaf.app',
      sourceApp: 'Safari',
      timestamp: '42 分钟',
      favorite: false,
      url: 'https://clipleaf.app',
    },
    {
      id: 'c6',
      kind: 'file',
      title: '3 个文件 · 品牌交付包',
      preview: 'clipleaf-mark.svg, 字体规范.pdf, 演示稿.key',
      content: '/Users/rustypiano/Design/clipleaf-mark.svg',
      sourceApp: '访达',
      timestamp: '1 小时',
      favorite: false,
      files: [
        '/Users/rustypiano/Design/clipleaf-mark.svg',
        '/Users/rustypiano/Design/字体规范.pdf',
        '/Users/rustypiano/Design/演示稿.key',
      ],
    },
  ];
}

export const COPY = {
  zh: {
    navDownload: '下载',
    themePaper: '素纸',
    themeInk: '墨夜',
    themePetal: '落樱',
    heroTitle: '复制过的，都在手边。',
    heroSub: '贴进 Word / PPT 不毁排版 · 本地 SQLite 毫秒检索 · 免费开源',
    downloadFor: '下载',
    chooseInstaller: '选择平台安装包',
    allPlatforms: '全部平台',
    macGuideTrigger: 'macOS 首次打开指引',
    sandboxShortcutTip: '点击聚焦搜索（或按 ⌘K）',
    sandboxSearchPlaceholder: '搜索历史，或按 ↑ ↓ 试玩…',
    sandboxTabHistory: '历史',
    sandboxTabFavorites: '收藏',
    sandboxCatAll: '全部',
    sandboxCatText: '文本',
    sandboxCatImage: '图片',
    sandboxCatFile: '文件',
    sandboxCatLink: '链接',
    sandboxEmpty: '无匹配记录',
    sandboxPastePlain: '纯文本',
    sandboxPasteDefault: '粘贴',
    sandboxEditInline: '编辑',
    sandboxSaveEdit: '保存',
    sandboxCopyLink: '复制',
    sandboxCopiedToast: (label: string) => `已复制 ${label}`,
    sandboxCopyFailedToast: (value: string) => `浏览器未允许写入剪贴板，请手动复制：${value}`,
    sandboxRevealToast: (name: string) => `已在访达中定位：${name}`,
    sandboxPastedToast: (title: string, plain: boolean) =>
      plain ? `已纯文本粘贴：${title}` : `已粘贴：${title}`,
    sandboxSavedToast: '已保存修改',
    plainPaste: {
      heading: '粘进 PPT，不该像一场车祸。',
      dragHint: '拖动中间滑块，实时洗净网页格式',
      tabPpt: 'PPT 幻灯片',
      tabWord: 'Word 文档',
      labelMessy: '普通粘贴',
      labelPlain: 'ClipLeaf 纯文本',
      pptHeader: 'Q4 产品复盘',
      pptTitle: '本地架构让搜索延迟降至毫秒级',
      pptMetric: '重复排版耗时减少',
      wordDocName: '项目方案_v3.docx',
      wordHeading: '2.1 本地存储架构',
      wordBeforeText: '为兼顾启动速度与数据隐私，客户端采用嵌入式本地索引方案：',
      wordAfterText: '整份文档的字号、行距与字色保持原样，无需再逐行清除格式。',
      sampleParagraph:
        '通过 SQLite FTS5 全文索引与 Rust 原生剪贴板监听，ClipLeaf 在离线环境下即可实现毫秒级历史检索，在 macOS、Windows 与 Linux 上保持一致体验。',
      messyLead: '通过 SQLite FTS5 全文索引',
      messyMid: ' 与 Rust 原生剪贴板监听，',
      messyLink: 'ClipLeaf（查看原文）',
      messyTail: ' 在离线环境下即可实现毫秒级检索…',
      modes: [
        {
          id: 'global' as PlainModeId,
          title: '全局纯文本',
          badge: '⌘V / Ctrl+V',
          desc: '复制后直接洗净样式，连系统普通粘贴都不带灰底与乱字号。',
        },
        {
          id: 'quickbar' as PlainModeId,
          title: 'QuickBar 去格式',
          badge: 'Enter',
          desc: '历史保留原样，从面板粘贴进文档时自动剥离格式。',
        },
        {
          id: 'alt' as PlainModeId,
          title: '单次临时切换',
          badge: '⌥Enter',
          desc: '平时保留富文本，需要干净文本时按住 ⌥Enter 切换一次。',
        },
      ],
    },
    craftHeading: '为每一次 ⌘⇧V 而做。',
    craftItems: [
      {
        title: '中文输入法不打架',
        body: 'SQLite FTS5 + trigram 毫秒级匹配高亮，专门适配 macOS 原生输入法候选窗，选词不误触。',
        meta: '1.4 ms · FTS5 · IME Safe',
      },
      {
        title: '色值、原图、文件，一眼看清',
        body: '点任意色块试试直接复制 HEX / RGB / HSL；透明 PNG 棋盘格预览，链接直开，文件一键定位。',
        meta: 'HEX · RGB · HSL · PNG',
      },
      {
        title: '历史之外，也是你的片段库',
        body: '高频 Prompt、命令与模板顺手收藏，支持预览栏在线改字与多选合并粘贴。',
        meta: '⇧↑↓ Merge · Inline Edit',
      },
    ],
    privacyHeading: '无云端。无账号。只留本机。',
    privacyLead:
      '剪贴板里藏着你最私密的瞬间。ClipLeaf 没有网络同步层，所有记录只写入你本机的 SQLite 数据库。',
    privacyFacts: [
      {
        title: '自定义本机目录',
        body: '所有记录保存在本地 SQLite 数据库中，可按习惯指定本机存储目录，零后台上传与遥测。',
      },
      {
        title: '隐私标记跳过（macOS / Windows）',
        body: '支持跳过密码管理器写入剪贴板隐私标记的内容（需密码管理器提供相应标记；Linux 暂不支持该标记检测，无标记的普通随机密码仍可能被记录），macOS 另支持按应用排除。',
      },
    ],
    themesHeading: '三种纸感，点一下就换。',
    themeCards: [
      { id: 'paper' as const, name: '素纸', sub: 'Paper' },
      { id: 'ink' as const, name: '墨夜', sub: 'Ink' },
      { id: 'petal' as const, name: '落樱', sub: 'Petal' },
    ],
    downloadHeading: '免费，轻量，开箱即用。',
    downloadNote: `当前版本 v${RELEASE_VERSION}，安装后可在应用内自动更新。`,
    downloadGuideBtn: 'macOS 首次打开指引',
    dialogTitle: 'macOS 首次打开与自动粘贴指引',
    dialogIntro:
      '当前开源版本使用自签名证书发布、尚未做 Apple 公证，首次打开按下面两步即可永久放行：',
    dialogStep1Title: '1. 解除 Gatekeeper 拦截（仅首次）',
    dialogStep1Body:
      '在访达中右键点击应用 → 选择「打开」；或前往「系统设置 → 隐私与安全性」，在底部点击「仍要打开」。',
    dialogStep2Title: '2. 开启辅助功能权限（用于自动粘贴）',
    dialogStep2Body:
      '前往「系统设置 → 隐私与安全性 → 辅助功能」，勾选应用即可回车自动粘回原窗口（未授权时仍可正常复制，手动 ⌘V 粘贴）。',
    dialogClose: '知道了',
    footerLicense: 'MIT Open Source',
    footerReleases: 'Releases',
    footerIssues: 'Issues',
  },
  en: {
    navDownload: 'Download',
    themePaper: 'Paper',
    themeInk: 'Ink',
    themePetal: 'Petal',
    heroTitle: 'Everything you copy. Kept light.',
    heroSub: 'Layout-safe plain paste for Word & PowerPoint · Local SQLite · Free & open source',
    downloadFor: 'Download for',
    chooseInstaller: 'Choose your installer',
    allPlatforms: 'All platforms',
    macGuideTrigger: 'macOS first-launch guide',
    sandboxShortcutTip: 'Focus search (or press ⌘K)',
    sandboxSearchPlaceholder: 'Search clips, or use ↑ ↓…',
    sandboxTabHistory: 'History',
    sandboxTabFavorites: 'Favorites',
    sandboxCatAll: 'All',
    sandboxCatText: 'Text',
    sandboxCatImage: 'Images',
    sandboxCatFile: 'Files',
    sandboxCatLink: 'Links',
    sandboxEmpty: 'No matching clips',
    sandboxPastePlain: 'Plain',
    sandboxPasteDefault: 'Paste',
    sandboxEditInline: 'Edit',
    sandboxSaveEdit: 'Save',
    sandboxCopyLink: 'Copy',
    sandboxCopiedToast: (label: string) => `Copied ${label}`,
    sandboxCopyFailedToast: (value: string) =>
      `Clipboard access blocked — please copy manually: ${value}`,
    sandboxRevealToast: (name: string) => `Revealed in Finder: ${name}`,
    sandboxPastedToast: (title: string, plain: boolean) =>
      plain ? `Pasted plain: ${title}` : `Pasted: ${title}`,
    sandboxSavedToast: 'Saved changes',
    plainPaste: {
      heading: "Pasting into slides shouldn't wreck them.",
      dragHint: 'Drag the handle to scrub web formatting live',
      tabPpt: 'PowerPoint',
      tabWord: 'Word',
      labelMessy: 'Standard Paste',
      labelPlain: 'ClipLeaf Plain',
      pptHeader: 'Q4 Product Review',
      pptTitle: 'Local indexing brings recall down to milliseconds',
      pptMetric: 'Less time fixing broken formatting',
      wordDocName: 'Proposal_v3.docx',
      wordHeading: '2.1 Local Storage',
      wordBeforeText:
        'To guarantee instant launch and strict privacy across every OS, the app embeds a local index:',
      wordAfterText:
        'Your document font, color, and spacing stay untouched — no manual reformatting required.',
      sampleParagraph:
        'Backed by SQLite FTS5 full-text search and a native Rust clipboard monitor, ClipLeaf searches thousands of clips offline in milliseconds on macOS, Windows, and Linux.',
      messyLead: 'Backed by SQLite FTS5 full-text search',
      messyMid: ' and a native Rust monitor, ',
      messyLink: 'ClipLeaf (Read Docs)',
      messyTail: ' searches thousands of clips offline in ms…',
      modes: [
        {
          id: 'global' as PlainModeId,
          title: 'Global Plain Text',
          badge: '⌘V / Ctrl+V',
          desc: 'Cleans rich HTML immediately on copy so even regular paste stays spotless.',
        },
        {
          id: 'quickbar' as PlainModeId,
          title: 'QuickBar Plain Paste',
          badge: 'Enter',
          desc: 'Keep formatting in history and strip styles automatically when pasting from QuickBar.',
        },
        {
          id: 'alt' as PlainModeId,
          title: 'One-Time Toggle',
          badge: '⌥Enter',
          desc: 'Keep formatting normally, hold ⌥Enter whenever you want a single plain paste.',
        },
      ],
    },
    craftHeading: 'Built for every ⌘⇧V.',
    craftItems: [
      {
        title: 'Instant search that respects CJK IME',
        body: 'SQLite FTS5 + trigram indexing highlights matches in milliseconds without interfering with macOS input method candidate windows.',
        meta: '1.4 ms · FTS5 · IME Safe',
      },
      {
        title: 'Colors, PNGs, files — clear at a glance',
        body: 'Click any swatch to copy HEX / RGB / HSL. Transparent PNGs render on a crisp checkerboard; links open directly and files reveal in Finder.',
        meta: 'HEX · RGB · HSL · PNG',
      },
      {
        title: 'History plus your reusable snippets',
        body: 'Star prompts, commands, and replies into Favorites, edit inline in the preview pane, or multi-select with ⇧↑↓ to merge-paste.',
        meta: '⇧↑↓ Merge · Inline Edit',
      },
    ],
    privacyHeading: 'No cloud. No accounts. Only your disk.',
    privacyLead:
      'Your clipboard carries your most private moments. ClipLeaf has zero network sync tier — every entry stays in your local SQLite database.',
    privacyFacts: [
      {
        title: 'Custom local storage directory',
        body: 'All history lives in a local SQLite database with zero background networking or telemetry. Choose any local disk path you prefer.',
      },
      {
        title: 'Privacy markers (macOS & Windows)',
        body: 'Skips clipboard items marked sensitive by password managers on macOS and Windows (requires the password manager to emit OS markers; not available on Linux, and unmarked random passwords may still be recorded), plus per-app exclusions on macOS.',
      },
    ],
    themesHeading: 'Three papers. One click.',
    themeCards: [
      { id: 'paper' as const, name: 'Paper', sub: 'Light' },
      { id: 'ink' as const, name: 'Ink', sub: 'Dark' },
      { id: 'petal' as const, name: 'Petal', sub: 'Blush' },
    ],
    downloadHeading: 'Free, fast, yours forever.',
    downloadNote: `Current version v${RELEASE_VERSION}, with built-in auto-updates.`,
    downloadGuideBtn: 'macOS first-launch guide',
    dialogTitle: 'macOS First Launch & Auto-Paste Setup',
    dialogIntro:
      'Open-source builds are self-signed (not yet Apple-notarized). Two quick one-time steps unlock everything:',
    dialogStep1Title: '1. Allow past Gatekeeper (one-time)',
    dialogStep1Body:
      'Right-click the app in Finder and choose "Open", or go to System Settings → Privacy & Security and click "Open Anyway".',
    dialogStep2Title: '2. Grant Accessibility (for instant auto-paste)',
    dialogStep2Body:
      'Go to System Settings → Privacy & Security → Accessibility and toggle the app on so pressing Enter pastes straight into your active window.',
    dialogClose: 'Got it',
    footerLicense: 'MIT Open Source',
    footerReleases: 'Releases',
    footerIssues: 'Issues',
  },
};
