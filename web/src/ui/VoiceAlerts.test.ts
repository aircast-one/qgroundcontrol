import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { returnAlert, speechBatch, speechPath } from './VoiceAlerts';

describe('InstrumentDetailTest', () => {
  it('return now is said once a flight, however often the charge crosses the turn-back point', () => {
    const first = returnAlert(false, true, true);
    expect(first).toEqual({ speak: true, alerted: true });
    const wobble = returnAlert(returnAlert(first.alerted, false, true).alerted, true, true);
    expect(wobble).toEqual({ speak: false, alerted: true });
    expect(returnAlert(true, false, false).alerted).toBe(false);
  });

  it("the core's spoken lines are read in order after the last one heard", () => {
    const batch = speechBatch(parseJson('{"class":"Speech","last":4,"lines":[{"sequence":3,"text":"communication lost","clip":"communication_lost","volume":0.5},{"sequence":4,"text":"armed","clip":"armed","volume":1}]}'));
    expect(batch).toEqual({
      last: 4,
      muted: false,
      lines: [
        { sequence: 3, text: 'communication lost', clip: 'communication_lost', volume: 0.5 },
        { sequence: 4, text: 'armed', clip: 'armed', volume: 1 },
      ],
    });
    expect(speechBatch(parseJson('{"class":"Speech","last":4,"muted":true,"lines":[]}'))?.muted).toBe(true);
    expect(speechPath(4)).toBe('view.speech(4)');
    expect(speechBatch(null)).toBeNull();
  });
});
