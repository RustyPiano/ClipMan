let now = $state(Date.now());

setInterval(() => {
  now = Date.now();
}, 30_000);

export function getNow(): number {
  return now;
}
