import { describe, expect, it } from 'vitest';
import { leavingLosesWork } from './CloseGuard';

describe('CloseGuardTest', () => {
  it('only a prompt the core marks as losing work holds the tab open, not a live connection', () => {
    expect(leavingLosesWork({ prompts: [{ id: 'unsavedMission', message: 'A', losesWork: true }] })).toBe(true);
    expect(leavingLosesWork({ prompts: [{ id: 'pendingWrites', message: 'B', losesWork: true }] })).toBe(true);
    expect(leavingLosesWork({ prompts: [{ id: 'activeConnections', message: 'C', losesWork: false }] })).toBe(false);
    expect(leavingLosesWork(null)).toBe(false);
  });
});
