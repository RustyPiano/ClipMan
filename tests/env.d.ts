declare module 'bun:test' {
  type Hook = () => void | Promise<void>;
  export function describe(label: string, fn: () => void): void;
  export function test(label: string, fn: Hook): void;
  export function beforeEach(fn: Hook): void;
  export function afterEach(fn: Hook): void;
  export function expect(value: unknown): any;
}

declare const Buffer: {
  from(data: string, encoding: string): { toString(encoding: string): string };
};
