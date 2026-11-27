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

export function detectMobile(): boolean {
  const ua = navigator.userAgent.toLowerCase();
  if (/iphone|ipad|ipod|android|mobile/.test(ua)) return true;
  return ua.includes('macintosh') && navigator.maxTouchPoints > 1;
}

export function isApplePlatform(): boolean {
  return /macintosh|mac os x|iphone|ipad/.test(navigator.userAgent.toLowerCase());
}

export function detectOs(): OsKind | null {
  if (detectMobile()) return null;
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('macintosh')) return 'mac-arm';
  if (ua.includes('windows nt')) return 'win';
  if (ua.includes('linux') && (ua.includes('x86_64') || ua.includes('amd64'))) return 'linux';
  return null;
}

export async function detectMacArchitecture(): Promise<'mac-arm' | 'mac-x64'> {
  if (!navigator.userAgentData) return 'mac-arm';
  const { architecture } = await navigator.userAgentData.getHighEntropyValues(['architecture']);
  return architecture === 'x86' ? 'mac-x64' : 'mac-arm';
}

export function splitPhrases(text: string): string[] {
  return text.match(/[^，、。：；！？]+[，、。：；！？]*/g) ?? [text];
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
        preview:
          'SELECT id, content FROM clips_fts WHERE clips_fts MATCH ? ORDER BY created_at DESC;',
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
      preview:
        'SELECT id, content FROM clips_fts WHERE clips_fts MATCH ? ORDER BY created_at DESC;',
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
    heroTitle: '离线、轻量的本地剪贴板',
    heroSub: '一键纯文本粘贴 · 几万条历史秒搜 · 数据只留本机 · 免费开源',
    downloadFor: '下载',
    chooseInstaller: '选择平台安装包',
    allPlatforms: '全部平台',
    macGuideTrigger: 'macOS 首次打开指引',
    macIntelLink: 'Intel 芯片版',
    macArmLink: 'Apple 芯片版',
    mobileCopyLink: '复制下载链接',
    mobileNote: 'ClipLeaf 是电脑端应用，支持 macOS、Windows 和 Linux。',
    mobileLinkCopied: '已复制链接，在电脑上打开即可下载。',
    mobileLinkFailed: (url: string) => `无法写入剪贴板，请手动复制：${url}`,
    sandboxShortcutTip: (keys: string) => `点击聚焦搜索（或按 ${keys}）`,
    sandboxCoach: '复制本页任意文字，都会收进这里',
    sandboxCopyVerb: '复制',
    sandboxDockTry: '复制本页任意文字试试',
    sandboxDockCount: (count: number) => `已收进 ${count} 条`,
    sandboxDockView: '查看',
    sandboxDockClose: '关闭',
    sandboxNow: '刚刚',
    sandboxCharCount: (count: number) => `${count} 个字符`,
    sandboxSearchPlaceholder: '搜索历史记录，或按 ↑ ↓ 切换…',
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
      heading: '粘进 Word 和 PPT，不再带进一堆乱格式。',
      dragHint: '拖动滑块，对比普通粘贴与纯文本粘贴',
      tabPpt: 'PPT 幻灯片',
      tabWord: 'Word 文档',
      labelMessy: '普通粘贴',
      labelPlain: 'ClipLeaf 纯文本',
      pptHeader: 'Q4 产品复盘',
      pptTitle: '本地架构让搜索响应降至毫秒级',
      pptMetric: '重复调格式耗时减少',
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
          desc: '复制时自动清除网页底色、字号和超链接，直接按系统粘贴也是干净的纯文本。',
        },
        {
          id: 'quickbar' as PlainModeId,
          title: 'QuickBar 去格式',
          badge: 'Enter',
          desc: '剪贴板保留原格式，从悬浮窗选中粘贴时自动转为纯文本。',
        },
        {
          id: 'alt' as PlainModeId,
          title: '单次临时切换',
          badge: '⌥Enter',
          desc: '平时保留原格式，需要纯文本时按 ⌥Enter 临时去格式粘贴一次。',
        },
      ],
    },
    craftHeading: '随手唤出，用完即走。',
    craftItems: [
      {
        title: '几万条历史秒搜，中文输入不误触',
        body: '输入关键词即可高亮匹配。切换到图片、文件或链接分类时，会沿用当前关键词重新搜索。在 macOS 上用回车选字也不会触发粘贴。',
      },
      {
        title: '色值、图片、文件，预览即所得',
        body: '复制色号自动识别，点击色块即可复制 HEX / RGB / HSL；透明 PNG 棋盘格高清预览，链接一键打开，本地文件直达所在目录。',
      },
      {
        title: '常用内容收藏，支持在线改字与合并粘贴',
        body: '把高频 Prompt、终端命令或常用回复加入收藏；粘贴前可在预览栏直接修改文字，或按 ⇧↑↓ 多选后一次合并粘贴。',
      },
    ],
    detailGridHeading: '顺手好用的细节，藏在每一次操作里。',
    detailGridItems: [
      {
        title: '回车直粘原窗口',
        body: '选中按 Enter 自动切回刚才的窗口光标处完成粘贴，支持在 macOS 全屏应用空间直接唤出，也可切换为仅复制模式。',
      },
      {
        title: '直接新建收藏片段',
        body: '不必先复制再收藏，支持直接在收藏夹新建常用 Prompt 与文本模板、自定义备注标题，并按最近使用或收藏时间排序。',
      },
      {
        title: '来源应用与智能去重',
        body: '自动记录内容来自哪个应用并展示原生应用图标；重复复制同一段内容自动合并置顶，不会把历史列表刷满重复项。',
      },
      {
        title: '批量多选与面板伸缩',
        body: '支持 ⇧+点击 连续多选、⌘+点击 跳选，一键批量收藏、批量删除或合并粘贴；悬浮窗可在单栏紧凑与双栏预览间随时切换。',
      },
      {
        title: '菜单栏 / 托盘即点即取',
        body: '常驻顶部菜单栏与系统托盘，不想按快捷键时，点开托盘菜单即可直接复制最近的剪贴板记录与常用收藏。',
      },
      {
        title: 'Rust 原生轻量常驻',
        body: '基于 Rust 与 Tauri 2 构建，安装包仅数 MB，后台常驻几乎不占内存与 CPU；支持设置历史保留上限与数据库空间自动回收。',
      },
    ],
    privacyHeading: '不联网，免登录，数据只存在你电脑里。',
    privacyLead:
      '剪贴板里不只有日常文字，还有密码、密钥与私人对话。ClipLeaf 完全离线运行，所有记录仅保存在本机 SQLite 数据库。',
    privacyFacts: [
      {
        title: '存储位置由你掌控',
        body: '所有数据保存在本地 SQLite 单文件中，支持按习惯自定义存储目录或随时迁移，无任何后台上传与数据统计。',
      },
      {
        title: '敏感内容与指定应用过滤',
        body: '自动跳过密码管理器里复制的密码，以及私钥和 API 令牌。macOS 上还可以排除指定应用。',
      },
    ],
    themesHeading: '三套主题，冷暖自选。',
    themeCards: [
      { id: 'paper' as const, name: '素纸' },
      { id: 'ink' as const, name: '墨夜' },
      { id: 'petal' as const, name: '落樱' },
    ],
    downloadHeading: '免费开源，即装即用。',
    downloadNote: `当前版本 v${RELEASE_VERSION} · MIT 开源协议 · 支持应用内自动更新`,
    downloadGuideBtn: 'macOS 首次打开指引',
    dialogTitle: 'macOS 首次打开与自动粘贴指引',
    dialogIntro:
      '当前开源版本使用自签名证书发布、尚未做 Apple 公证，首次打开按下面两步即可永久放行：',
    dialogDownloadStarted: '安装包已开始下载。下载完成后，按下面两步打开 ClipLeaf。',
    dialogStep1Title: '1. 解除 Gatekeeper 拦截（仅首次）',
    dialogStep1Body:
      '在访达中右键点击应用 → 选择「打开」；或前往「系统设置 → 隐私与安全性」，在底部点击「仍要打开」。',
    dialogStep2Title: '2. 开启辅助功能权限（用于自动粘贴）',
    dialogStep2Body:
      '前往「系统设置 → 隐私与安全性 → 辅助功能」，勾选应用即可回车自动粘回原窗口（未授权时仍可正常复制，手动 ⌘V 粘贴）。',
    dialogClose: '知道了',
    footerReleases: 'Releases',
    footerIssues: 'Issues',
  },
  en: {
    navDownload: 'Download',
    themePaper: 'Paper',
    themeInk: 'Ink',
    themePetal: 'Petal',
    heroTitle: 'A fast, local clipboard manager',
    heroSub: 'One-key plain-text paste · Instant local search · 100% offline & open source',
    downloadFor: 'Download for',
    chooseInstaller: 'Choose your installer',
    allPlatforms: 'All platforms',
    macGuideTrigger: 'macOS first-launch guide',
    macIntelLink: 'Intel version',
    macArmLink: 'Apple silicon version',
    mobileCopyLink: 'Copy download link',
    mobileNote: 'ClipLeaf is a desktop app for macOS, Windows and Linux.',
    mobileLinkCopied: 'Link copied. Open it on your computer to download.',
    mobileLinkFailed: (url: string) => `Clipboard access blocked — please copy manually: ${url}`,
    sandboxShortcutTip: (keys: string) => `Focus search (or press ${keys})`,
    sandboxCoach: 'Copy any text on this page. It lands here.',
    sandboxCopyVerb: 'Copy',
    sandboxDockTry: 'Try copying any text here',
    sandboxDockCount: (count: number) => `${count} new clip${count === 1 ? '' : 's'}`,
    sandboxDockView: 'View',
    sandboxDockClose: 'Close',
    sandboxNow: 'Now',
    sandboxCharCount: (count: number) => `${count} characters`,
    sandboxSearchPlaceholder: 'Search history, or use ↑ ↓…',
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
      heading: 'Paste into docs and slides without ruining your layout.',
      dragHint: 'Drag the handle to compare standard paste vs. plain paste',
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
          desc: 'Strips web styling immediately on copy so even a regular system paste stays clean.',
        },
        {
          id: 'quickbar' as PlainModeId,
          title: 'QuickBar Plain Paste',
          badge: 'Enter',
          desc: 'Keep rich formatting on the clipboard and strip styles automatically when pasting from QuickBar.',
        },
        {
          id: 'alt' as PlainModeId,
          title: 'One-Time Toggle',
          badge: '⌥Enter',
          desc: 'Keep formatting by default and press ⌥Enter whenever you need a single plain-text paste.',
        },
      ],
    },
    craftHeading: 'Fast to summon. Effortless to use.',
    craftItems: [
      {
        title: 'Search thousands of clips instantly — CJK IME friendly',
        body: 'Matching words are highlighted as you type. Switch to Images, Files, or Links to search that category with the same keywords. On macOS, pressing Enter to choose an input method candidate keeps the search open.',
      },
      {
        title: 'Colors, images, and files — clear at a glance',
        body: 'Auto-detects copied color codes (click any swatch to copy HEX / RGB / HSL), previews transparent PNGs on a crisp checkerboard, opens links, and reveals local files in Finder.',
      },
      {
        title: 'Save favorite snippets, edit inline, or merge-paste',
        body: 'Star frequent prompts, commands, and replies into Favorites. Edit text inline right inside the preview pane, or select multiple items with ⇧↑↓ to merge-paste in one stroke.',
      },
    ],
    detailGridHeading: 'Thoughtful details in every interaction.',
    detailGridItems: [
      {
        title: 'Auto-paste back to your active window',
        body: 'Press Enter to restore focus and paste directly where your cursor was — even over macOS fullscreen Spaces — or switch to copy-only mode.',
      },
      {
        title: 'Create snippets from scratch',
        body: 'No need to copy first. Create reusable prompts and templates directly in Favorites, add custom labels, and sort by recently used.',
      },
      {
        title: 'Source app icons & smart deduplication',
        body: 'Shows which app each clip came from with native app icons on macOS. Re-copying identical content bumps the existing item instead of cluttering history.',
      },
      {
        title: 'Batch selection & flexible layout',
        body: 'Use ⇧+Click or ⌘+Click to batch-star, batch-delete, or merge-paste multiple clips. Switch QuickBar between compact single-column and wide split-preview modes.',
      },
      {
        title: 'Menu bar & system tray access',
        body: 'Lives quietly in your menu bar or system tray so you can grab recent clips and favorite snippets with a single click.',
      },
      {
        title: 'Lightweight native Rust core',
        body: 'Built with Rust and Tauri 2 in a few-megabyte installer with minimal RAM and CPU footprint, configurable history limits, and automatic SQLite space reclamation.',
      },
    ],
    privacyHeading: 'Offline by design. No account required.',
    privacyLead:
      'Your clipboard holds more than everyday text — it sees passwords, API keys, and private notes. ClipLeaf runs 100% offline and stores everything in a local SQLite database on your machine.',
    privacyFacts: [
      {
        title: 'Your disk, your directory',
        body: 'All history lives in a single local SQLite file with zero background requests or telemetry. Move or customize your storage folder anytime.',
      },
      {
        title: 'Password manager & app filtering',
        body: 'Passwords from password managers, private keys and API tokens are skipped automatically. On macOS you can also exclude specific apps.',
      },
    ],
    themesHeading: 'Three themes. Easy on the eyes.',
    themeCards: [
      { id: 'paper' as const, name: 'Paper' },
      { id: 'ink' as const, name: 'Ink' },
      { id: 'petal' as const, name: 'Petal' },
    ],
    downloadHeading: 'Free, open source, ready to go.',
    downloadNote: `Version v${RELEASE_VERSION} · MIT License · Built-in auto-updates`,
    downloadGuideBtn: 'macOS first-launch guide',
    dialogTitle: 'macOS First Launch & Auto-Paste Setup',
    dialogIntro:
      'Open-source builds are self-signed (not yet Apple-notarized). Two quick one-time steps unlock everything:',
    dialogDownloadStarted:
      'Your download has started. Once it finishes, open ClipLeaf in two steps.',
    dialogStep1Title: '1. Allow past Gatekeeper (one-time)',
    dialogStep1Body:
      'Right-click the app in Finder and choose "Open", or go to System Settings → Privacy & Security and click "Open Anyway".',
    dialogStep2Title: '2. Grant Accessibility (for instant auto-paste)',
    dialogStep2Body:
      'Go to System Settings → Privacy & Security → Accessibility and toggle the app on so pressing Enter pastes straight into your active window.',
    dialogClose: 'Got it',
    footerReleases: 'Releases',
    footerIssues: 'Issues',
  },
};
