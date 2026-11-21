# 0021 前端测试环境

- 状态：已采纳

## 决定

- bun:test 套件直接导入未编译的 `.svelte.ts` store，把 `$state`、`$derived` 换成普通的全局函数。
- 测试的类型声明写在 `tests/env.d.ts`，不引入 `@types/bun` 和 `@types/node`。`bun test` 提供真实实现，这些声明只供类型检查使用，让被测代码按真实的 `$lib` 类型检查。
- 测试使用单独的 `tests/tsconfig.json`，由 `tsc` 检查（`bun run test:types`）：svelte-check 会注入真实的 rune 类型，和测试里的替身冲突。bun 运行时支持 ES2023（`toSorted`、`at`），应用仍以 ES2020 为目标，只给测试提高 lib。
- e2e 需要覆盖取消路径时，在同一个任务里派发输入、Enter 和 Escape。分开发送时，慢机器上 30 ms 防抖加 90 ms 搜索可能在 Enter 之前完成，这时 Enter 粘贴已有结果是正确行为，测试就覆盖不到取消路径。

## 已知问题

测试断言的类型是 any，见 #5。
