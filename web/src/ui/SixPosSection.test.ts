import { describe, expect, it } from 'vitest';
import { sixPosOf } from '@/ui/SixPosSection';

describe('sixPosOf', () => {
  it('reads whether the 6POS is being learned, how far, and whether a table is kept', () => {
    expect(sixPosOf({ learning: true, heard: 2, learned: null, text: '2 of 6 positions heard.' })).toEqual({
      learning: true,
      heard: 2,
      learned: false,
      text: '2 of 6 positions heard.',
    });
    expect(sixPosOf({ learning: false, heard: 0, learned: [1411, 1423, 1437, 1450, 1464, 1477], text: 'learned' })?.learned).toBe(true);
    expect(sixPosOf(null)).toBeNull();
  });
});
