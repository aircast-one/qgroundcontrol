import { describe, expect, it } from 'vitest';
import { replayFileName } from './LogReplayBar';

describe('LogReplayNameTest', () => {
  it('theReplayLinkIsNamedAfterThePickedFileLikeLinkManager', () => {
    expect(replayFileName('2026-10-03 10-11-12.tlog')).toBe('2026-10-03 10-11-12.tlog');
    expect(replayFileName('../../flight.tlog')).toBe('flight.tlog');
    expect(replayFileName(null)).toBe('log-replay.tlog');
    expect(replayFileName('..')).toBe('log-replay.tlog');
  });
});
