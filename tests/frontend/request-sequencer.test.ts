import { describe, expect, test } from 'bun:test';
import { RequestSequencer } from '../../src/lib/utils/request-sequencer';

describe('RequestSequencer', () => {
  test('only treats the latest request as current', () => {
    const sequencer = new RequestSequencer();
    const first = sequencer.next();
    const second = sequencer.next();

    expect(sequencer.isCurrent(first)).toBe(false);
    expect(sequencer.isCurrent(second)).toBe(true);
  });
});
