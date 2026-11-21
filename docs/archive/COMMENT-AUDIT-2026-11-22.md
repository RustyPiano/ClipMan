# Comment audit · 2026-11-22

Archived audit of commit `3c9cab7`. The columns identify every reviewed comment by its original file and inclusive line numbers. Adjacent lines in one category share a range. Inline comments use the line containing the code. Cargo generates its two lockfile header lines. Interpreter directives, runtime strings and documentation examples are outside the prose count.

The decision records cover [capture and storage](../adr/0001-capture-and-storage.md), [QuickBar and native integration](../adr/0002-quickbar.md), and [build, test and release](../adr/0003-build-test-release.md).

The cleanup removed 853 standalone lines and stripped trailing comments from 13 more. The 866 comment-bearing lines comprised 398 tautologies, 12 stale lines, 9 excuse lines and 447 decision-record lines.

| Shortcut | Tracking issue |
| --- | --- |
| AppKit thread assumptions during capture | [#3](https://github.com/RustyPiano/ClipMan/issues/3) |
| Swallowed listener teardown errors | [#4](https://github.com/RustyPiano/ClipMan/issues/4) |
| Unchecked types in the Bun test harness | [#5](https://github.com/RustyPiano/ClipMan/issues/5) |
| Fixed waits in the Finder rename test | [#6](https://github.com/RustyPiano/ClipMan/issues/6) |
| Fixed delay before simulated paste | [#7](https://github.com/RustyPiano/ClipMan/issues/7) |
| Code-font heuristic for ordinary text | [#8](https://github.com/RustyPiano/ClipMan/issues/8) |

| File | Tautology | Stale | Excuse | Decision Record |
| --- | --- | --- | --- | --- |
| `.github/workflows/ci.yml` | 18, 86 | — | — | 81–82, 138–141, 155–156 |
| `.github/workflows/prepare-release.yml` | 3–7, 69 | — | — | 8–17 |
| `.github/workflows/release.yml` | 6, 18, 77, 90, 120, 128, 160 | 150 | — | 8–9, 96–105, 122, 132–133, 145–149, 155–157, 161, 170 |
| `.gitignore` | 1, 10, 13, 16, 22, 26, 35, 39, 48, 61 | — | — | 51–52, 58–59 |
| `e2e/quickbar.spec.ts` | 117, 413, 493, 533, 569, 613 | — | — | 224–225, 426, 547–548 |
| `e2e/settings.spec.ts` | 98, 278 | — | — | — |
| `rust-toolchain.toml` | 5 | — | — | — |
| `scripts/check-versions.sh` | 2–12 | — | — | — |
| `scripts/native-qa/control.swift` | 1 | — | — | 60, 85 |
| `scripts/native-qa/run.py` | 77, 141, 149, 156, 187 | — | 134 | 46, 125, 180 |
| `scripts/release.sh` | 2–18, 40, 45, 47, 53, 62, 83 | — | — | 33, 41–42, 54–56 |
| `scripts/verify-release.py` | — | — | — | 146, 153 |
| `src-tauri/Cargo.toml` | 15, 25, 29, 34, 41, 49, 53, 62 | — | — | 35, 80–81 |
| `src-tauri/src/accessibility.rs` | 25, 27, 31, 48 | — | — | 1–9, 40, 43–45, 109–110 |
| `src-tauri/src/clipboard.rs` | 59, 170–171, 263, 330, 472–475, 499, 547, 704–705, 833–834, 893, 900–901, 913, 931–933, 942–943, 952 | — | 706–707 | 31–34, 41–44, 72–74, 89, 134–135, 179–180, 202, 205–206, 247, 265, 382–385, 399, 522, 555–558, 694–695, 732, 743, 774, 787 |
| `src-tauri/src/commands.rs` | 1, 15–16, 178, 191–192, 198–199, 219, 227–229, 282, 286, 309, 714 | — | — | 54–55, 70, 236–237, 423–425, 527–528, 561–562, 632, 639–648, 884–885, 903 |
| `src-tauri/src/main.rs` | 1, 34, 45, 119, 144–147, 188, 240, 290, 321, 399, 421 | — | — | 73–75, 169–171, 200–204, 207–210, 234–236, 253–254 |
| `src-tauri/src/migration.rs` | 283, 316 | — | — | 15, 42–43, 110–112, 261–264 |
| `src-tauri/src/paste.rs` | 79, 185, 219, 247, 316, 718 | 574–576 | 579 | 32, 84–85, 136, 277, 309–310, 340–341, 362–365, 383–396, 404–407, 436–437, 513–514, 542–545, 560–565, 582, 651–658 |
| `src-tauri/src/secrets.rs` | 38–40, 73–74, 96–97, 117, 131–132, 185 | — | — | 1–7, 174–175 |
| `src-tauri/src/settings.rs` | 70, 81, 83, 85, 87, 204, 234 | — | — | 34, 50, 89, 163, 332, 339 |
| `src-tauri/src/storage.rs` | 78, 80, 100, 167, 170, 172, 415, 551, 1024, 1029, 1034, 1042, 1347, 1639–1640, 1652–1653, 1845, 1891–1893, 1925, 2198, 2345, 2604–2605, 2615, 2696, 2714–2715 | — | — | 9–10, 48, 105, 176, 234, 256–259, 278–282, 284–287, 384–388, 562, 615, 625, 647–649, 663–664, 684, 751–754, 1081–1083, 1189–1194 |
| `src-tauri/src/tray.rs` | 1, 13, 16–17, 20, 57, 76, 81, 84, 98, 118, 153, 180, 186, 211, 227, 240–241, 256, 317, 332 | — | — | 193, 286, 289–291, 334 |
| `src-tauri/src/window.rs` | 52, 261–263, 539 | — | — | 8–11, 159–161, 216–218, 243–246, 270–271, 285, 298–301, 310–313, 316–317, 322, 349, 494 |
| `src/app.css` | 37, 79, 115, 245, 261, 278 | — | — | 65–70, 105–106, 141–142, 163–166, 172–173 |
| `src/lib/components/ClipPreview.svelte` | — | — | — | 28, 54–55 |
| `src/lib/components/ClipboardItem.svelte` | 83–84, 121, 187, 213, 219, 246, 346, 428 | — | — | 110, 127–129, 144–145, 406, 439–440 |
| `src/lib/components/PermissionCheck.svelte` | 17–18, 22, 85–87 | — | — | 59–60, 67–68 |
| `src/lib/components/Toast.svelte` | 7 | — | — | — |
| `src/lib/components/settings/AppearanceSettings.svelte` | 34, 70 | — | — | — |
| `src/lib/components/settings/GeneralSettings.svelte` | — | — | 85 | 115 |
| `src/lib/components/settings/Sidebar.svelte` | 19 | — | — | — |
| `src/lib/components/ui/ConfirmDialog.svelte` | — | — | — | 12 |
| `src/lib/constants.ts` | 1 | 3 | — | 6–10 |
| `src/lib/i18n/index.svelte.ts` | 1, 6, 22, 27, 43, 47, 53, 58, 61, 69, 83, 98, 106, 114, 122, 130, 144, 148, 155, 158, 162, 170, 176, 185, 190, 198, 202, 208, 215, 231, 236, 252, 256, 262, 267, 270, 278, 292, 308, 316, 324, 332, 340, 354, 358, 365, 368, 372, 380, 387, 396, 401, 409, 413, 419, 426, 442, 447, 463, 467, 473, 478, 481, 489, 504, 522, 530, 538, 546, 554, 568, 572, 579, 582, 586, 596, 604, 613, 618, 627, 632, 639, 663 | — | — | 647 |
| `src/lib/i18n/index.ts` | 1 | — | — | — |
| `src/lib/stores/app-icons.svelte.ts` | 16 | — | — | 5, 8, 29 |
| `src/lib/stores/clipboard.svelte.ts` | 190–196, 213, 274, 412, 452–456, 486, 491 | — | — | 41, 61, 65–66, 87–89, 91–92, 138–140, 144–145, 180–181, 209, 225–227, 246–251, 285, 311–314, 323–324, 337–339, 372, 441–446, 530 |
| `src/lib/stores/confirm.svelte.ts` | — | — | — | 15 |
| `src/lib/stores/selection.svelte.ts` | — | — | — | 15, 30, 37, 45 |
| `src/lib/stores/theme.svelte.ts` | 13 | — | — | — |
| `src/lib/types.ts` | 21, 46, 49, 51, 57 | — | — | 7, 10, 30, 53 |
| `src/lib/utils/clip-items.ts` | 76, 96, 101, 114, 120 | — | 63 | — |
| `src/lib/utils/now.svelte.ts` | — | — | — | 1 |
| `src/routes/+page.svelte` | 137–138 | — | — | 53, 80, 167, 200, 304–307 |
| `src/routes/settings/+page.svelte` | 216, 261, 264, 317 | — | — | 182, 192 |
| `svelte.config.js` | — | 4–5 | — | — |
| `tests/env.d.ts` | 18 | 1–5 | 13–14 | — |
| `tests/frontend/app-icons.test.ts` | 24 | — | — | 3 |
| `tests/frontend/clip-items.test.ts` | — | — | — | 227 |
| `tests/frontend/clipboard-store.test.ts` | 108, 131, 136, 247, 293, 302, 325, 341, 345, 351, 377, 402, 404, 415, 419, 511–512, 515, 572, 582, 593, 595 | — | 13 | 11–12 |
| `tests/frontend/confirm-store.test.ts` | — | — | — | 7 |
| `tests/release-prepare.test.js` | 149 | — | — | — |
| `tests/selection.test.ts` | — | — | — | 3 |
| `tests/tsconfig.json` | 12 | — | — | 2–6, 9–10 |
| `tsconfig.json` | 9, 17, 23 | — | — | — |
| `vite.config.js` | 6 | — | — | — |
