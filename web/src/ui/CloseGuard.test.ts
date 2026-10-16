import { describe, expect, it } from 'vitest';
import { closePrompts } from './CloseGuard';

describe('CloseGuardTest', () => {
  it('close prompts are read in the order the core asks them', () => {
    const read = closePrompts({ prompts: [{ id: 'unsavedMission', message: 'A' }, { id: 'activeConnections', message: 'B' }] });
    expect(read).toEqual(['A', 'B']);
    expect(closePrompts(null)).toEqual([]);
  });
});
