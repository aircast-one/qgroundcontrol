import { describe, expect, it } from 'vitest';
import { leavingLosesWork } from './CloseGuard';

describe('CloseGuardTest', () => {
  it('only work the page would lose holds the tab open, not a live connection', () => {
    expect(leavingLosesWork({ prompts: [{ id: 'unsavedMission', message: 'A' }] })).toBe(true);
    expect(leavingLosesWork({ prompts: [{ id: 'pendingWrites', message: 'B' }] })).toBe(true);
    expect(leavingLosesWork({ prompts: [{ id: 'activeConnections', message: 'C' }] })).toBe(false);
    expect(leavingLosesWork(null)).toBe(false);
  });
});
