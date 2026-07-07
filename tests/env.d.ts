// Ambient shims so the bun:test suites type-check under `bun run check` without
// pulling in @types/bun or @types/node. `bun test` supplies the real runtime
// implementations; these declarations only satisfy the type checker so that
// the code under test (factories, stores, helpers) is verified against the real
// `$lib` types — which is the point of bringing tests into the type check.

declare module 'bun:test' {
  type Hook = () => void | Promise<void>;
  export function describe(label: string, fn: () => void): void;
  export function test(label: string, fn: Hook): void;
  export function beforeEach(fn: Hook): void;
  export function afterEach(fn: Hook): void;
  // The matcher surface the suites use; `any` keeps the shim small while still
  // type-checking the code under test (the real target of this config).
  export function expect(value: unknown): any;
}

// Node's Buffer, used by one suite to build UTF-8 base64 fixtures.
declare const Buffer: {
  from(data: string, encoding: string): { toString(encoding: string): string };
};
