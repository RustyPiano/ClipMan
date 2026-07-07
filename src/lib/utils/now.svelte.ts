// Shared reactive clock. The tray window is hidden, not destroyed, so item
// components stay mounted and relative-time labels would otherwise freeze at
// whatever they showed on first render (every item stuck on "刚刚"). Reading
// getNow() inside a label makes it re-evaluate on each tick.
//
// ponytail: 30s tick is plenty for 刚刚 / N 分钟前 / N 小时前 granularity.
let now = $state(Date.now());

setInterval(() => {
  now = Date.now();
}, 30_000);

export function getNow(): number {
  return now;
}
