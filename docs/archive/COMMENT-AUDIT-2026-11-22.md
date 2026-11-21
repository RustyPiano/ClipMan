> 归档于 2026-11-22：一次性注释审查记录，不作为当前指导。规则见 `docs/adr/0001-no-code-comments.md`。

# 注释审查记录（2026-11-22）

逐条审查基于提交 `3c9cab7`。Rust 与 JS/TS/Svelte 共 667 个注释 token（432 组）：同义重复 354，过时 3，开脱 16，设计决定 294。另有 CSS、YAML、Shell、Python、Swift、TOML 和 tsconfig 的注释按下表处理。设计决定写进 `docs/adr/`；开脱类注释对应 Issue #3–#11。`GeneralSettings.svelte` 空 catch 块里的注释暂留：删除后 `no-empty` 报错，修复 #4 时一并删除。

| 文件 | 原行号（3c9cab7） | 分类 | 去向 |
| --- | --- | --- | --- |
| `.github/workflows/ci.yml` | 18,86,155-156 | 同义重复 |  |
| `.github/workflows/ci.yml` | 81-82,138-141 | 设计决定 | ADR 0022 |
| `.github/workflows/prepare-release.yml` | 69 | 同义重复 |  |
| `.github/workflows/prepare-release.yml` | 3-17 | 设计决定 | ADR 0022 |
| `.github/workflows/release.yml` | 6,18,77,90,120,122,128,160,170 | 同义重复 |  |
| `.github/workflows/release.yml` | 8-9,96-105,132-133,145-150,155-157,161 | 设计决定 | ADR 0022 |
| `e2e/quickbar.spec.ts` | 117, 413, 426, 493, 533, 547-548, 569, 613 | 同义重复 |  |
| `e2e/quickbar.spec.ts` | 224-225 | 设计决定 | ADR 0021 |
| `e2e/settings.spec.ts` | 98, 278 | 同义重复 |  |
| `rust-toolchain.toml` | 5 | 同义重复 |  |
| `scripts/check-versions.sh` | 2-10 | 同义重复 |  |
| `scripts/check-versions.sh` | 11-12 | 设计决定 | ADR 0022 |
| `scripts/native-qa/control.swift` | 1,85 | 同义重复 |  |
| `scripts/native-qa/control.swift` | 60 | 设计决定 | ADR 0014 |
| `scripts/native-qa/run.py` | 46,77,141,149,156,180,187 | 同义重复 |  |
| `scripts/native-qa/run.py` | 134 | 开脱 | #6 |
| `scripts/native-qa/run.py` | 125 | 设计决定 | ADR 0014 |
| `scripts/release.sh` | 2-18,40,53,62,83 | 同义重复 |  |
| `scripts/release.sh` | 33,41-42,45,47,54-56 | 设计决定 | ADR 0022 |
| `scripts/verify-release.py` | 146,153 | 设计决定 | ADR 0022 |
| `src-tauri/Cargo.toml` | 15,25,29,34-35,41,49,53,62 | 同义重复 |  |
| `src-tauri/Cargo.toml` | 83-84 | 设计决定 | ADR 0022 |
| `src-tauri/src/accessibility.rs` | 25, 27, 31, 40, 43-45, 48, 109-110 | 同义重复 |  |
| `src-tauri/src/accessibility.rs` | 1-9 | 设计决定 | ADR 0012 |
| `src-tauri/src/clipboard.rs` | 59, 170-171, 330, 472-475, 499, 547, 833-834, 893, 900-901, 913, 931-933, 942-943, 952 | 同义重复 |  |
| `src-tauri/src/clipboard.rs` | 704-707 | 开脱 | #3 |
| `src-tauri/src/clipboard.rs` | 31-34, 41-44, 134-135, 202, 382-385, 399, 555-558 | 设计决定 | ADR 0002 |
| `src-tauri/src/clipboard.rs` | 72-74, 89, 179-180, 205-206 | 设计决定 | ADR 0003 |
| `src-tauri/src/clipboard.rs` | 247, 263, 265, 522 | 设计决定 | ADR 0004 |
| `src-tauri/src/clipboard.rs` | 694-695, 732, 743, 774, 787 | 设计决定 | ADR 0017 |
| `src-tauri/src/commands.rs` | 1, 15-16, 70, 178, 191-192, 198-199, 219, 227-229, 282, 309, 423-425, 632, 714 | 同义重复 |  |
| `src-tauri/src/commands.rs` | 236-237, 286 | 设计决定 | ADR 0004 |
| `src-tauri/src/commands.rs` | 884-885, 903 | 设计决定 | ADR 0010 |
| `src-tauri/src/commands.rs` | 639-648 | 设计决定 | ADR 0011 |
| `src-tauri/src/commands.rs` | 54-55 | 设计决定 | ADR 0016 |
| `src-tauri/src/commands.rs` | 527-528, 561-562 | 设计决定 | ADR 0018 |
| `src-tauri/src/main.rs` | 1, 34, 45, 119, 144-147, 188, 240, 290, 321, 399, 421 | 同义重复 |  |
| `src-tauri/src/main.rs` | 73-75, 169-171, 253-254 | 设计决定 | ADR 0010 |
| `src-tauri/src/main.rs` | 200-204, 207-210, 234-236 | 设计决定 | ADR 0016 |
| `src-tauri/src/migration.rs` | 110-112, 283, 316 | 同义重复 |  |
| `src-tauri/src/migration.rs` | 15, 42-43 | 设计决定 | ADR 0008 |
| `src-tauri/src/migration.rs` | 261-264 | 设计决定 | ADR 0010 |
| `src-tauri/src/paste.rs` | 79, 136, 185, 219, 247, 316, 718 | 同义重复 |  |
| `src-tauri/src/paste.rs` | 579 | 开脱 | #7 |
| `src-tauri/src/paste.rs` | 32, 309-310, 340-341, 362-365, 513-514 | 设计决定 | ADR 0003 |
| `src-tauri/src/paste.rs` | 277 | 设计决定 | ADR 0004 |
| `src-tauri/src/paste.rs` | 84-85, 542-545, 560-565, 574-576, 582, 651-658 | 设计决定 | ADR 0012 |
| `src-tauri/src/paste.rs` | 383-396, 404-407, 436-437 | 设计决定 | ADR 0013 |
| `src-tauri/src/secrets.rs` | 38-40, 73-74, 96-97, 117, 131-132, 185 | 同义重复 |  |
| `src-tauri/src/secrets.rs` | 1-7, 174-175 | 设计决定 | ADR 0005 |
| `src-tauri/src/settings.rs` | 34, 81, 83, 85, 87, 89, 234 | 同义重复 |  |
| `src-tauri/src/settings.rs` | 204 | 开脱 | #11 |
| `src-tauri/src/settings.rs` | 70 | 设计决定 | ADR 0004 |
| `src-tauri/src/settings.rs` | 50, 163, 332, 339 | 设计决定 | ADR 0011 |
| `src-tauri/src/storage.rs` | 48, 78, 80, 100, 105, 167, 170, 172, 176, 551, 647-649, 663-664, 1024, 1029, 1034, 1042, 1189-1194, 1639-1640, 1652-1653, 1845, 1891-1893, 1925, 2198, 2345, 2604-2605, 2615, 2696, 2714-2715 | 同义重复 |  |
| `src-tauri/src/storage.rs` | 625 | 开脱 | #10 |
| `src-tauri/src/storage.rs` | 415 | 开脱 | #9 |
| `src-tauri/src/storage.rs` | 9-10, 384-388, 751-754 | 设计决定 | ADR 0006 |
| `src-tauri/src/storage.rs` | 256-259, 278-282, 284-287 | 设计决定 | ADR 0007 |
| `src-tauri/src/storage.rs` | 234, 1081-1083, 1347 | 设计决定 | ADR 0008 |
| `src-tauri/src/storage.rs` | 562, 615 | 设计决定 | ADR 0009 |
| `src-tauri/src/storage.rs` | 684 | 设计决定 | ADR 0017 |
| `src-tauri/src/tray.rs` | 1, 13, 16-17, 20, 57, 76, 81, 84, 98, 118, 153, 180, 186, 193, 211, 227, 240-241, 256, 317, 332 | 同义重复 |  |
| `src-tauri/src/tray.rs` | 334 | 设计决定 | ADR 0002 |
| `src-tauri/src/tray.rs` | 286, 289-291 | 设计决定 | ADR 0011 |
| `src-tauri/src/window.rs` | 8-11, 52, 261-263, 316-317, 539 | 同义重复 |  |
| `src-tauri/src/window.rs` | 270-271, 285, 298-301, 322, 494 | 设计决定 | ADR 0014 |
| `src-tauri/src/window.rs` | 159-161, 216-218, 243-246, 310-313, 349 | 设计决定 | ADR 0015 |
| `src/app.css` | 37,79,105-106,115,141-142,172-173,245,261,278 | 同义重复 |  |
| `src/app.css` | 65-70,163-166 | 设计决定 | ADR 0015 |
| `src/lib/components/ClipPreview.svelte` | 54-55 | 设计决定 | ADR 0009 |
| `src/lib/components/ClipPreview.svelte` | 28 | 设计决定 | ADR 0019 |
| `src/lib/components/ClipboardItem.svelte` | 121, 187, 213, 246, 346, 428 | 同义重复 |  |
| `src/lib/components/ClipboardItem.svelte` | 83-84, 110, 127-129, 144-145, 219, 406, 439-440 | 设计决定 | ADR 0019 |
| `src/lib/components/PermissionCheck.svelte` | 17-18, 22, 59-60, 67-68 | 同义重复 |  |
| `src/lib/components/PermissionCheck.svelte` | 85-87 | 设计决定 | ADR 0019 |
| `src/lib/components/Toast.svelte` | 7 | 同义重复 |  |
| `src/lib/components/settings/AppearanceSettings.svelte` | 34, 70 | 同义重复 |  |
| `src/lib/components/settings/GeneralSettings.svelte` | 85 | 开脱 | #4（暂留） |
| `src/lib/components/settings/GeneralSettings.svelte` | 115 | 设计决定 | ADR 0018 |
| `src/lib/components/settings/Sidebar.svelte` | 19 | 同义重复 |  |
| `src/lib/components/ui/ConfirmDialog.svelte` | 12 | 同义重复 |  |
| `src/lib/constants.ts` | 1, 3 | 同义重复 |  |
| `src/lib/constants.ts` | 6-10 | 设计决定 | ADR 0019 |
| `src/lib/i18n/index.svelte.ts` | 1, 6, 22, 27, 43, 47, 53, 58, 61, 69, 83, 98, 106, 114, 122, 130, 144, 148, 155, 158, 162, 170, 176, 185, 190, 198, 202, 208, 215, 231, 236, 252, 256, 262, 267, 270, 278, 292, 308, 316, 324, 332, 340, 354, 358, 365, 368, 372, 380, 387, 396, 401, 409, 413, 419, 426, 442, 447, 463, 467, 473, 478, 481, 489, 504, 522, 530, 538, 546, 554, 568, 572, 579, 582, 586, 596, 604, 613, 618, 627, 632, 639, 663 | 同义重复 |  |
| `src/lib/i18n/index.svelte.ts` | 647 | 设计决定 | ADR 0011 |
| `src/lib/i18n/index.ts` | 1 | 同义重复 |  |
| `src/lib/stores/app-icons.svelte.ts` | 5, 16 | 同义重复 |  |
| `src/lib/stores/app-icons.svelte.ts` | 8, 29 | 设计决定 | ADR 0017 |
| `src/lib/stores/clipboard.svelte.ts` | 41, 190-196, 213, 246-251, 274, 285, 372, 412, 441-446, 452-456, 486, 491, 530 | 同义重复 |  |
| `src/lib/stores/clipboard.svelte.ts` | 138-140, 144-145, 225-227 | 设计决定 | ADR 0006 |
| `src/lib/stores/clipboard.svelte.ts` | 61, 65-66 | 设计决定 | ADR 0009 |
| `src/lib/stores/clipboard.svelte.ts` | 87-89, 91-92, 180-181, 209, 311-314, 323-324, 337-339 | 设计决定 | ADR 0020 |
| `src/lib/stores/confirm.svelte.ts` | 15 | 同义重复 |  |
| `src/lib/stores/selection.svelte.ts` | 15, 30, 37, 45 | 设计决定 | ADR 0019 |
| `src/lib/stores/theme.svelte.ts` | 13 | 同义重复 |  |
| `src/lib/types.ts` | 7, 21, 46, 49, 51, 53, 57 | 同义重复 |  |
| `src/lib/types.ts` | 30 | 开脱 | #11 |
| `src/lib/types.ts` | 10 | 设计决定 | ADR 0006 |
| `src/lib/utils/clip-items.ts` | 76, 96, 101, 114, 120 | 同义重复 |  |
| `src/lib/utils/clip-items.ts` | 63 | 开脱 | #8 |
| `src/lib/utils/now.svelte.ts` | 1 | 同义重复 |  |
| `src/routes/+page.svelte` | 53, 304-307 | 同义重复 |  |
| `src/routes/+page.svelte` | 137-138 | 设计决定 | ADR 0004 |
| `src/routes/+page.svelte` | 80, 167, 200 | 设计决定 | ADR 0019 |
| `src/routes/settings/+page.svelte` | 192, 216, 261, 264, 317 | 同义重复 |  |
| `src/routes/settings/+page.svelte` | 182 | 设计决定 | ADR 0011 |
| `svelte.config.js` | 4-5 | 过时 |  |
| `tests/env.d.ts` | 18 | 同义重复 |  |
| `tests/env.d.ts` | 13-14 | 开脱 | #5 |
| `tests/env.d.ts` | 1-5 | 设计决定 | ADR 0021 |
| `tests/frontend/app-icons.test.ts` | 24 | 同义重复 |  |
| `tests/frontend/app-icons.test.ts` | 3 | 设计决定 | ADR 0021 |
| `tests/frontend/clip-items.test.ts` | 227 | 同义重复 |  |
| `tests/frontend/clipboard-store.test.ts` | 108, 131, 136, 247, 293, 302, 325, 341, 345, 351, 377, 402, 404, 415, 419, 511-512, 515, 572, 582, 593, 595 | 同义重复 |  |
| `tests/frontend/clipboard-store.test.ts` | 11-13 | 开脱 | #5 |
| `tests/frontend/confirm-store.test.ts` | 7 | 同义重复 |  |
| `tests/release-prepare.test.js` | 149 | 同义重复 |  |
| `tests/selection.test.ts` | 3 | 同义重复 |  |
| `tests/tsconfig.json` | 12 | 同义重复 |  |
| `tests/tsconfig.json` | 2-6,9-10 | 设计决定 | ADR 0021 |
| `tsconfig.json` | 9,17,23 | 同义重复 |  |
| `vite.config.js` | 6 | 过时 |  |
