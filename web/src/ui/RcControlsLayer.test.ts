import { describe, expect, it } from 'vitest';
import { RcHolder } from './RcControlsLayer';

const queueing = (): { holder: RcHolder; queued: () => number } => {
  const queue: { blocks: readonly (() => Promise<unknown>)[] } = { blocks: [] };
  const holder = new RcHolder((block) => {
    queue.blocks = [...queue.blocks, block];
  });
  return { holder, queued: () => queue.blocks.length };
};

describe('RcHolderTest', () => {
  it('a control releases every channel it sent, not the ones it is set to now', () => {
    const { holder, queued } = queueing();
    holder.hold(7, 1600);
    holder.hold(8, 1600);
    expect(holder.holding(), 'tilt moved from channel 7 to 8 mid-flight; both were sent, so both are held').toEqual(new Set([7, 8]));
    holder.release();
    expect(holder.holding()).toEqual(new Set());
    expect(queued(), 'two sends and one release, in that order on one queue').toBe(3);
  });

  it('an unassigned channel is never sent or held', () => {
    const { holder, queued } = queueing();
    holder.hold(0, 1600);
    holder.release();
    expect(holder.holding()).toEqual(new Set());
    expect(queued()).toBe(0);
  });

  it('giving everything back forgets what was held without a second release', () => {
    const { holder, queued } = queueing();
    holder.hold(9, 2000);
    holder.forget();
    holder.release();
    expect(queued()).toBe(1);
  });
});
