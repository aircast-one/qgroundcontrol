import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { bayTileLabel, lockLabel, payloadNote, payloadRailOf } from './PayloadRail';

const SHOWN = '{"shown":true,"armed":true,"selected":3,"selectedBy":"radio","bays":[{"bay":1,"loaded":true},{"bay":2,"loaded":false}],"dropReady":false,"dropLabel":"No bay 3","reason":"This drone has 2 bays."}';

describe('PayloadRail', () => {
  it("reads the core's rail and hides when the drone cannot drop", () => {
    const rail = payloadRailOf(parseJson(SHOWN));
    expect(rail).toEqual({ shown: true, armed: true, selected: 3, selectedBy: 'radio', bays: [{ bay: 1, loaded: true }, { bay: 2, loaded: false }], dropReady: false, dropLabel: 'No bay 3', dropAll: false, dropAllReady: false, reason: 'This drone has 2 bays.' });
    expect(payloadRailOf(parseJson('{"shown":false}'))).toBeNull();
    expect(payloadRailOf(null)).toBeNull();
    expect(payloadRailOf(parseJson('{"shown":true,"selectedBy":"martian","reason":null}'))?.selectedBy).toBe('app');
    expect(payloadRailOf(parseJson('{"shown":true,"reason":null}'))?.reason).toBeNull();
    expect(payloadRailOf(parseJson('{"shown":true,"dropAll":true,"dropAllReady":true}'))).toMatchObject({ dropAll: true, dropAllReady: true });
  });

  it('says nothing while the payload is safe, and explains only what blocks an armed drop or a refused press', () => {
    const rail = payloadRailOf(parseJson(SHOWN));
    if (rail === null) throw new Error('shown');
    const safe = { ...rail, armed: false, reason: 'The payload is safe. Arm it first.' };
    expect(payloadNote(safe, null)).toBeNull();
    expect(payloadNote(safe, 'The payload is safe. Arm it first.')).toBe('The payload is safe. Arm it first.');
    expect(payloadNote(rail, null)).toBe('This drone has 2 bays.');
    expect(payloadNote({ ...rail, dropReady: true }, null)).toBeNull();
  });

  it('names the tiles by their state, and the lock is always the tap that flips it', () => {
    const radio = payloadRailOf(parseJson(SHOWN));
    if (radio === null) throw new Error('shown');
    expect(lockLabel(radio)).toBe('Payload armed, tap to make safe');
    expect(lockLabel({ ...radio, armed: false })).toBe('Payload safe, tap to arm');
    expect(bayTileLabel(radio, 3, true)).toBe('Bay 3, loaded, selected');
    expect(bayTileLabel(radio, 2, false)).toBe('Bay 2, empty');
    expect(bayTileLabel({ ...radio, armed: false }, 2, false)).toBe('Bay 2, empty, tap when reloaded');
  });
});
