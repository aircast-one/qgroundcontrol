import { describe, expect, it } from 'vitest';
import { RcControlType, RcControlTypeEntries, channelOwner, channelUsable, firstFreeChannel, parseRcControls, rcControlsAdded, rcControlsPatched, rcControlsRemoved, rcSendDue } from './RcControls';

const entryAt = (json: string, index: number): Record<string, unknown> => (JSON.parse(json) as Record<string, unknown>[])[index] ?? {};

describe('RcControlsTest', () => {
  it('an unset setting yields no controls', () => {
    expect(parseRcControls('[]')).toEqual([]);
    expect(parseRcControls(null)).toEqual([]);
    expect(parseRcControls('')).toEqual([]);
  });

  it('each supported type is read', () => {
    const parsed = parseRcControls(
      `[{"label":"Gimbal","channel":7,"type":"slider"},
                {"label":"Light","channel":8,"type":"button"},
                {"label":"Mode","channel":9,"type":"switch3"},
                {"label":"Drop","channel":10,"type":"momentary"}]`,
    );

    expect(parsed).toEqual([
      { label: 'Gimbal', channel: 7, type: RcControlType.Slider },
      { label: 'Light', channel: 8, type: RcControlType.Button },
      { label: 'Mode', channel: 9, type: RcControlType.Switch3 },
      { label: 'Drop', channel: 10, type: RcControlType.Momentary },
    ]);
  });

  it('a control with no channel is dropped rather than sent to channel zero', () => {
    expect(parseRcControls('[{"label":"X","type":"slider"}]')).toEqual([]);
  });

  it('an unknown type is dropped rather than guessed', () => {
    expect(parseRcControls('[{"label":"X","channel":5,"type":"dial"}]')).toEqual([]);
  });

  it('a control with no label falls back to its channel', () => {
    expect(parseRcControls('[{"channel":6,"type":"slider"}]')).toEqual([{ label: 'CH6', channel: 6, type: RcControlType.Slider }]);
  });

  it('malformed json yields nothing rather than throwing', () => {
    expect(parseRcControls('not json')).toEqual([]);
  });
});

describe('RcSendRateTest', () => {
  it('a drag does not send on every pixel', () => {
    expect(rcSendDue(1_040, 1_000, false)).toBe(false);
    expect(rcSendDue(1_099, 1_000, false)).toBe(false);
  });

  it('a drag still sends often enough to feel live', () => {
    expect(rcSendDue(1_100, 1_000, false)).toBe(true);
    expect(rcSendDue(2_000, 1_000, false)).toBe(true);
  });

  it('letting go always sends, so the vehicle ends where the finger did', () => {
    expect(rcSendDue(1_001, 1_000, true)).toBe(true);
    expect(rcSendDue(1_000, 1_000, true)).toBe(true);
  });

  it('the first send of a drag is never throttled', () => {
    expect(rcSendDue(5_000, 0, false)).toBe(true);
  });
});

describe('RcControlsEditTest', () => {
  const served = `
        [{"label":"Winch","channel":7,"type":"slider","orientation":"vertical"},
         {"label":"Light","channel":9,"type":"button"}]
    `;

  it('editing a control keeps the fields this head does not understand', () => {
    const next = rcControlsPatched(served, 0, 'Hoist', 8, RcControlType.Button);
    const entry = entryAt(next, 0);

    expect(entry.label).toBe('Hoist');
    expect(entry.channel).toBe(8);
    expect(entry.type).toBe('button');
    expect(entry.orientation).toBe('vertical');
  });

  it('adding and removing keep the rest of the list intact', () => {
    const added = rcControlsAdded(served, 'Zoom', 11, RcControlType.Switch3);
    expect(parseRcControls(added).map((it) => it.label)).toEqual(['Winch', 'Light', 'Zoom']);
    expect(parseRcControls(added)[2]?.type).toBe(RcControlType.Switch3);

    const removed = rcControlsRemoved(added, 1);
    expect(parseRcControls(removed).map((it) => it.label)).toEqual(['Winch', 'Zoom']);
    expect(entryAt(removed, 0).orientation).toBe('vertical');
  });

  it('a channel already driving something names what has it', () => {
    const reserved = new Map([[5, 'Gimbal tilt']]);

    expect(channelOwner(served, 5, -1, reserved)).toBe('Gimbal tilt');
    expect(channelOwner(served, 9, 0, reserved)).toBe('Light');
    expect(channelOwner(served, 9, 1, reserved)).toBeNull();
    expect(channelOwner(served, 12, -1, reserved)).toBeNull();
  });

  it('an unnamed control still identifies itself in a clash', () => {
    const unnamed = '[{"label":"","channel":4,"type":"button"}]';

    expect(channelOwner(unnamed, 4, -1, new Map())).toBe('control 1');
  });

  it('a new control lands on the first channel nothing else is using', () => {
    expect(firstFreeChannel(served, new Map())).toBe(1);
    expect(firstFreeChannel(served, new Map([[1, 'Gimbal tilt']]))).toBe(2);
    expect(
      firstFreeChannel(
        '[{"channel":1,"type":"button"},{"channel":2,"type":"button"}]',
        new Map([
          [3, 'a'],
          [4, 'b'],
          [5, 'c'],
          [6, 'd'],
          [7, 'e'],
        ]),
      ),
    ).toBe(8);
  });

  it("channels outside the radio's range are refused", () => {
    expect(channelUsable(0)).toBe(false);
    expect(channelUsable(19)).toBe(false);
    expect(channelUsable(1)).toBe(true);
    expect(channelUsable(18)).toBe(true);
  });

  it('a type round trips through its stored name', () => {
    expect(RcControlTypeEntries.map((type) => parseRcControls(rcControlsAdded(null, 'x', 3, type)).map((it) => it.type))).toEqual(RcControlTypeEntries.map((type) => [type]));
  });
});
