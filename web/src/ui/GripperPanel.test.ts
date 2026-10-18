import { describe, expect, it } from 'vitest';
import { agoText, bayLabel, payloadStatus } from './GripperPanel';

const at = 1_000_000;

describe('PayloadStatusTest', () => {
  it('says nothing until a payload command has been sent', () => {
    expect(payloadStatus(null, at)).toBeNull();
    expect(payloadStatus({ action: 'release', outcome: 'lost', atMs: at }, at)).toBeNull();
  });

  it('tells the pilot what the flight controller answered and how long ago', () => {
    expect(payloadStatus({ action: 'release', outcome: 'sent', atMs: at }, at + 500)).toEqual({ text: 'Drop sent, waiting for the aircraft', warning: false });
    expect(payloadStatus({ action: 'release', outcome: 'confirmed', atMs: at }, at + 12_400)).toEqual({ text: 'Dropped 12 s ago', warning: false });
    expect(payloadStatus({ action: 'grab', outcome: 'confirmed', atMs: at }, at + 65_000)).toEqual({ text: 'Grabbed 1 min ago', warning: false });
    expect(payloadStatus({ action: 'release', outcome: 'refused', atMs: at }, at + 2_000)).toEqual({ text: 'Drop refused by the flight controller 2 s ago', warning: true });
  });

  it('a drop the aircraft never answered is a warning, not a failure', () => {
    expect(payloadStatus({ action: 'release', outcome: 'unanswered', atMs: at }, at + 3_000)).toEqual({ text: 'Drop not confirmed 3 s ago. It may have dropped, check before dropping again.', warning: true });
    expect(payloadStatus({ action: 'grab', outcome: 'unanswered', atMs: at }, at + 3_000)).toEqual({ text: 'Grab not confirmed 3 s ago. Check the gripper before trying again.', warning: true });
  });

  it('a basket names the bay in every line', () => {
    expect(payloadStatus({ action: 'release', bay: 2, outcome: 'confirmed', atMs: at }, at + 4_000)).toEqual({ text: 'Bay 2 dropped 4 s ago', warning: false });
    expect(payloadStatus({ action: 'release', bay: 3, outcome: 'unanswered', atMs: at }, at)).toEqual({ text: 'Drop 3 not confirmed 0 s ago. It may have dropped, check before dropping again.', warning: true });
    expect([bayLabel(1, { bays: 3, empty: [2], next: 1, problems: [] }), bayLabel(2, { bays: 3, empty: [2], next: 1, problems: [] })]).toEqual(['Slide to drop bay 1', 'Bay 2 empty, slide to drop']);
  });

  it('time since is coarse and never negative', () => {
    expect([agoText(at, at - 5_000), agoText(at, at + 59_999), agoText(at, at + 3_600_000)]).toEqual(['0 s ago', '59 s ago', '1 h ago']);
  });
});
