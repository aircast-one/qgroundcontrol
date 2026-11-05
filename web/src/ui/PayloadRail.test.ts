import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { bayTileLabel, lockLabel, payloadRailOf } from './PayloadRail';

const SHOWN = '{"shown":true,"armed":true,"armedBy":"radio","selected":3,"selectedBy":"radio","bays":[{"bay":1,"loaded":true},{"bay":2,"loaded":false}],"dropReady":false,"dropLabel":"No bay 3","reason":"This drone has 2 bays."}';

describe('PayloadRail', () => {
  it("reads the core's rail and hides when the drone cannot drop", () => {
    const rail = payloadRailOf(parseJson(SHOWN));
    expect(rail).toEqual({ shown: true, armed: true, armedBy: 'radio', selected: 3, selectedBy: 'radio', bays: [{ bay: 1, loaded: true }, { bay: 2, loaded: false }], dropReady: false, dropLabel: 'No bay 3', reason: 'This drone has 2 bays.' });
    expect(payloadRailOf(parseJson('{"shown":false}'))).toBeNull();
    expect(payloadRailOf(null)).toBeNull();
    expect(payloadRailOf(parseJson('{"shown":true,"armedBy":"martian","reason":null}'))?.armedBy).toBe('app');
    expect(payloadRailOf(parseJson('{"shown":true,"reason":null}'))?.reason).toBeNull();
  });

  it('names the tiles by their state and who owns the switches', () => {
    const radio = payloadRailOf(parseJson(SHOWN));
    if (radio === null) throw new Error('shown');
    expect(lockLabel(radio)).toBe('Payload armed by radio');
    expect(lockLabel({ ...radio, armed: false, armedBy: 'app' })).toBe('Payload safe, tap to arm');
    expect(lockLabel({ ...radio, armedBy: 'app' })).toBe('Payload armed, tap to make safe');
    expect(bayTileLabel(radio, 3, true)).toBe('Bay 3, loaded, selected');
    expect(bayTileLabel(radio, 2, false)).toBe('Bay 2, empty');
  });
});
