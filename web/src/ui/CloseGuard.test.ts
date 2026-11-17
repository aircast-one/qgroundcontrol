import { describe, expect, it } from 'vitest';
import { leavingLosesWork, leavingWarns } from './CloseGuard';

describe('CloseGuardTest', () => {
  it('only a prompt the core marks as losing work holds the tab open, not a live connection', () => {
    expect(leavingLosesWork({ prompts: [{ id: 'unsavedMission', message: 'A', losesWork: true }] })).toBe(true);
    expect(leavingLosesWork({ prompts: [{ id: 'pendingWrites', message: 'B', losesWork: true }] })).toBe(true);
    expect(leavingLosesWork({ prompts: [{ id: 'activeConnections', message: 'C', losesWork: false }] })).toBe(false);
    expect(leavingLosesWork(null)).toBe(false);
  });

  it('the browser warns before leaving while a vehicle is connected, as well as when work would be lost', () => {
    const connected = { prompts: [{ id: 'activeConnections', message: 'C', losesWork: false }] };
    expect(leavingWarns(connected, true)).toBe(true);
    expect(leavingWarns(connected, false)).toBe(false);
    expect(leavingWarns({ prompts: [{ id: 'unsavedMission', message: 'A', losesWork: true }] }, false)).toBe(true);
    expect(leavingWarns(null, false)).toBe(false);
  });
});
