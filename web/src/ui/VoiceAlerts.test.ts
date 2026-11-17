import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { linkAlert, linkBuzz, returnAlert, speechBatch, speechPath } from './VoiceAlerts';

describe('InstrumentDetailTest', () => {
  it('return now is said once a flight, however often the charge crosses the turn-back point', () => {
    const first = returnAlert(false, true, true);
    expect(first).toEqual({ speak: true, alerted: true });
    const wobble = returnAlert(returnAlert(first.alerted, false, true).alerted, true, true);
    expect(wobble).toEqual({ speak: false, alerted: true });
    expect(returnAlert(true, false, false).alerted).toBe(false);
  });

  it('a dropped aircraft buzzes and is said aloud, once, where a silent one only buzzes because the core already said it', () => {
    const dropped = linkAlert(false, false, false, true);
    expect(dropped).toEqual({ heard: true, buzz: linkBuzz(true), speak: true });
    expect(linkAlert(dropped.heard, false, false, true), 'once').toEqual({ heard: true, buzz: null, speak: false });
    const silent = linkAlert(false, true, true, false);
    expect(silent).toEqual({ heard: true, buzz: linkBuzz(true), speak: false });
    const gone = linkAlert(silent.heard, false, false, false);
    expect(gone, 'the moment between the vehicle closing and its sighting being kept says nothing').toEqual({ heard: true, buzz: null, speak: false });
    expect(linkAlert(gone.heard, false, false, true), 'and the drop after a silence is not said twice').toEqual({ heard: true, buzz: null, speak: false });
    expect(linkAlert(true, true, false, false)).toEqual({ heard: false, buzz: linkBuzz(false), speak: false });
  });

  it('a vehicle that is simply not there, or arrives healthy, raises nothing', () => {
    expect(linkAlert(null, false, false, false)).toEqual({ heard: null, buzz: null, speak: false });
    expect(linkAlert(null, true, false, false)).toEqual({ heard: false, buzz: null, speak: false });
    expect(linkAlert(false, false, false, false)).toEqual({ heard: false, buzz: null, speak: false });
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
