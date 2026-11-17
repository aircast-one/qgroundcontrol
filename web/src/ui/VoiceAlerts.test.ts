import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { linkAlert, linkBuzz, lostMotors, motorLossAlert, returnAlert, speechBatch, speechPath } from './VoiceAlerts';

describe('InstrumentDetailTest', () => {
  it('a motor whose telemetry is lost in flight is said once, by number, and not again while it stays lost', () => {
    expect(lostMotors(parseJson('{"shown":true,"frame":[{"motor":1,"state":"reporting"},{"motor":3,"state":"lost"},{"motor":4,"state":"none"},{"motor":6,"state":"lost"}]}'))).toBe('3,6');
    const first = motorLossAlert([], [3], true);
    expect(first).toEqual({ heard: [3], spoken: 'Motor 3 telemetry lost' });
    expect(motorLossAlert(first.heard, [3], true).spoken).toBeNull();
    expect(motorLossAlert(first.heard, [3, 6], true).spoken, 'a second motor going is news').toBe('Motor 6 telemetry lost');
    expect(motorLossAlert([], [2, 5], true).spoken).toBe('Motors 2, 5 telemetry lost');
    expect(motorLossAlert([], [3], false), 'on the ground the red rotor says it').toEqual({ heard: [3], spoken: null });
  });

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
