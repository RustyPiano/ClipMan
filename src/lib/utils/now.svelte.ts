// 窗口隐藏时组件仍保留，共用时钟让各行的相对时间持续更新。
let now = $state(Date.now());

setInterval(() => {
  now = Date.now();
}, 30_000);

export function getNow(): number {
  return now;
}
