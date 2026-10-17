import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { trimmedNumber } from './CameraSectionExtras';
import { type CameraExtras, actionLabel, cameraChoices, cameraExtras, itemCameraNote, itemCameraText, itemCameraTextBeside, namedAction } from './ItemCamera';

const parsed = (text: string): Json => JSON.parse(text) as Json;

const extrasOf = (text: string): CameraExtras => {
  const extras = cameraExtras(parsed(text));
  if (extras === null) throw new Error('no camera extras');
  return extras;
};

describe('CameraExtrasTest', () => {
  it("the camera section's interval, mode and gimbal come from the core", () => {
    const extras = extrasOf(
      '{"available":true,"intervalTime":{"value":4.0},"intervalDistance":null,"cameraModeSupported":true,"commandsMode":true,"cameraMode":{"choice":1},"commandsGimbal":true,"gimbalPitch":{"value":-45.0},"gimbalYaw":null}',
    );
    expect(extras.intervalTime).toBe(4.0);
    expect(extras.intervalDistance).toBeNull();
    expect(extras.mode).toBe(1);
    expect(extras.pitch).toBe(-45.0);
    expect(extras.yaw).toBe(0.0);
    expect(cameraExtras(parsed('{"available":false}'))).toBeNull();
    expect(trimmedNumber(10.0)).toBe('10');
  });

  it("the gimbal angles slide over the fact's user range like CameraSection's FactTextFieldSlider", () => {
    const extras = extrasOf(
      '{"available":true,"commandsGimbal":true,"gimbalPitch":{"value":45.0,"slider":{"from":90.0,"to":0.0,"decimals":0}},"gimbalYaw":{"value":0.0,"slider":{"from":-180.0,"to":180.0,"decimals":0}}}',
    );
    expect(extras.pitchRange).toEqual({ start: 0.0, endInclusive: 90.0 });
    expect(extras.yawRange).toEqual({ start: -180.0, endInclusive: 180.0 });
    expect(extrasOf('{"available":true,"gimbalPitch":{"value":-45.0}}').pitchRange).toBeNull();
  });

  it('the photo distance is labelled in the units its fact is cooked to', () => {
    const feet = extrasOf('{"available":true,"intervalDistance":{"value":100.0,"units":"ft"}}');
    expect(feet.intervalDistance).toBe(100.0);
    expect(feet.distanceUnits).toBe('ft');
    expect(extrasOf('{"available":true,"intervalDistance":{"value":3.0}}').distanceUnits).toBe('m');
  });
});

describe('ItemCameraDuplicateTest', () => {
  const camera = (commandsGimbal: boolean): Json =>
    parsed(`{"available":true,"commandsGimbal":${commandsGimbal},
           "cameraAction":{"choices":["No change","Take photo"],"choice":0,"text":"No change"},
           "gimbalPitch":{"text":"-90 deg"},"gimbalYaw":{"text":"45 deg"}}`);

  it('the summary is dropped when the picker beside it already says the same word', () => {
    const view = camera(false);
    expect(itemCameraText(view)).toBe('No change');
    expect(itemCameraTextBeside(view, 'No change')).toBeNull();
  });

  it('the summary stays when it carries the gimbal the picker cannot show', () => {
    expect(itemCameraTextBeside(camera(true), 'No change')).toBe('No change · gimbal -90 deg / 45 deg');
  });

  it('with no picker at all the summary is whatever it was', () => {
    expect(itemCameraTextBeside(camera(false), null)).toBe('No change');
  });
});

describe('ItemCameraNoteTest', () => {
  it('the mission start note is shown when the core serves it, like MissionSettingsEditor', () => {
    const note = 'Camera commands above take effect immediately at mission start.';
    expect(itemCameraNote(parsed(`{"available":true,"note":"${note}"}`))).toBe(note);
    expect(itemCameraNote(parsed('{"available":true,"note":null}'))).toBeNull();
    expect(itemCameraNote(parsed(`{"available":false,"note":"${note}"}`))).toBeNull();
  });
});

describe('ItemCameraTest', () => {
  const view = ({
    available = true,
    commandsGimbal = true,
    action = '{"value":1,"text":"Take photo","units":""}',
    pitch = '{"value":-45.0,"text":"-45.0","units":"deg"}',
    yaw = '{"value":0.0,"text":"0.0","units":"deg"}',
  }: { available?: boolean; commandsGimbal?: boolean; action?: string; pitch?: string; yaw?: string } = {}): Json =>
    parsed(`{"kind":"object","class":"ItemCamera","index":1,"available":${available},
           "commandsGimbal":${commandsGimbal},"cameraAction":${action},
           "gimbalPitch":${pitch},"gimbalYaw":${yaw}}`);

  it('an item with no camera section says nothing', () => {
    expect(itemCameraText(null)).toBeNull();
    expect(itemCameraText(view({ available: false }))).toBeNull();
  });

  it('an item that commands the gimbal states the action and both angles', () => {
    expect(itemCameraText(view())).toBe('Take photo · gimbal -45.0 deg / 0.0 deg');
  });

  it('angles are withheld for an item that does not command the gimbal', () => {
    const untouched = view({ commandsGimbal: false, pitch: 'null', yaw: 'null' });
    expect(itemCameraText(untouched)).toBe('Take photo');
  });

  it('an item that touches neither is a blank line rather than an empty bullet', () => {
    const nothing = view({ commandsGimbal: false, action: 'null', pitch: 'null', yaw: 'null' });
    expect(itemCameraText(nothing)).toBeNull();
  });

  it('a bare number is not a camera action and is not drawn as one', () => {
    const unlabelled = view({ commandsGimbal: false, action: '{"value":0,"text":"0.000","units":""}', pitch: 'null', yaw: 'null' });
    expect(itemCameraText(unlabelled)).toBeNull();
    expect(namedAction('0.000')).toBe('');
    expect(namedAction('-1')).toBe('');
    expect(namedAction('Take photo')).toBe('Take photo');
  });

  it('a number beside a gimbal angle still leaves the angle', () => {
    const mixed = view({ action: '{"value":0,"text":"0.000","units":""}' });
    expect(itemCameraText(mixed)).toBe('gimbal -45.0 deg / 0.0 deg');
  });

  it('a measure with no units is not given a trailing space', () => {
    const unitless = view({ commandsGimbal: false, action: '{"value":1,"text":"Take photo","units":null}', pitch: 'null', yaw: 'null' });
    expect(itemCameraText(unitless)).toBe('Take photo');
  });
});

describe('CameraChoicesTest', () => {
  const enums = '["No change","Take photo","Take photos (time)","Stop taking photos"]';

  const view = (action: string): Json =>
    parsed(`{"kind":"object","class":"ItemCamera","index":2,"available":true,
           "commandsGimbal":false,"cameraAction":${action},"gimbalPitch":null,"gimbalYaw":null}`);

  it('a fact with choices offers them and says which is chosen', () => {
    const choices = cameraChoices(view(`{"value":1,"text":"Take photo","units":"","choices":${enums},"choice":1}`));
    expect(choices?.labels.length).toBe(4);
    expect(choices?.chosen).toBe(1);
  });

  it('a blank label keeps its place, because choice indexes the list the core sent', () => {
    const gap = cameraChoices(view('{"value":2,"text":"","units":"","choices":["No change","","Take photo"],"choice":2}'));
    expect(
      gap === null ? null : gap.labels[gap.chosen],
      'dropping the blank shifted every later label down one, so choice 2 named the entry before the one the core chose - on a plan item that is a different camera command',
    ).toBe('Take photo');
  });

  it('the Fact metadata spelling offers nothing, because the core does not serve it', () => {
    expect(
      cameraChoices(view(`{"value":1,"text":"Take photo","units":"","enumStrings":${enums},"enumIndex":1}`)),
      'itemcamera.rs serves choices and choice - the cooked list and the chosen index - and never enumStrings or enumIndex. Reading the Qt spelling meant the picker could never populate, and the fixture that said otherwise was written by hand',
    ).toBeNull();
  });

  it('a plain numeric fact offers nothing, because it is not a choice', () => {
    expect(cameraChoices(view('{"value":-45.0,"text":"-45.0","units":"deg"}'))).toBeNull();
    expect(cameraChoices(view('{"value":0,"text":"0","units":"","choices":[]}'))).toBeNull();
    expect(cameraChoices(null)).toBeNull();
  });

  it('the label comes from the choice list when there is one', () => {
    const named = view(`{"value":1,"text":"1.000","units":"","choices":${enums},"choice":1}`);
    expect(actionLabel(named)).toBe('Take photo');
    expect(itemCameraText(named)).toBe('Take photo');
  });

  it('a fact with no metadata keeps whatever its text could render', () => {
    expect(actionLabel(view('{"value":3,"text":"Something","units":""}'))).toBe('Something');
  });

  it('an index outside the list falls back rather than crashing', () => {
    const odd = view(`{"value":9,"text":"9.000","units":"","choices":${enums},"choice":9}`);
    expect(actionLabel(odd)).toBe('');
    expect(itemCameraText(odd)).toBeNull();
  });
});
