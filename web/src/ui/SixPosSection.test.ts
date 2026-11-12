import { describe, expect, it } from 'vitest';
import { sixPosOf } from '@/ui/SixPosSection';

describe('sixPosOf', () => {
  it('reads whether the 6POS is being learned, how far, and whether a table is kept', () => {
    expect(sixPosOf({ learning: true, heard: 2, learned: null, blocked: null, text: '2 of 6 positions heard.' })).toEqual({
      learning: true,
      heard: 2,
      learned: false,
      blocked: null,
      text: '2 of 6 positions heard.',
    });
    expect(sixPosOf({ learning: false, heard: 0, learned: [1411, 1423, 1437, 1450, 1464, 1477], blocked: null, text: 'learned' })?.learned).toBe(true);
    expect(sixPosOf(null)).toBeNull();
  });

  it('says why learning cannot start', () => {
    expect(sixPosOf({ learning: false, heard: 0, learned: null, blocked: 'Connect the radio on USB to learn its 6POS.', text: '' })?.blocked).toBe(
      'Connect the radio on USB to learn its 6POS.',
    );
  });
});
