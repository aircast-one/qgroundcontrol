import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { guidedSpeed, guidedSpeedPath } from './GuidedSpeed';
import { speedReading } from './GuidedValueFlow';

const json = (text: string): Json => parseJson(text) ?? {};

const served = (text: string) => {
  const speed = guidedSpeed(json(text));
  if (speed === null) throw new Error('the fixture is an available speed');
  return speed;
};

describe('GuidedSpeedTest', () => {
  it('the core picks the command so the head never chooses airspeed or ground speed', () => {
    const multirotor = served(`{"available":true,"label":"Speed","unit":"m/s",
      "command":"guidedModeChangeGroundSpeedMetersSecond",
      "initial":5.0,"minimum":1.0,"maximum":20.0}`);
    const forward = served(`{"available":true,"label":"Airspeed","unit":"m/s",
      "command":"guidedModeChangeEquivalentAirspeedMetersSecond",
      "initial":15.0,"minimum":5.0,"maximum":30.0}`);

    expect(multirotor.command).toBe('guidedModeChangeGroundSpeedMetersSecond');
    expect(forward.command).toBe('guidedModeChangeEquivalentAirspeedMetersSecond');
    expect(multirotor.label).toBe('Speed');
    expect(forward.label).toBe('Airspeed');
  });

  it('a vehicle with no speed command offers no dialog', () => {
    const none = served(`{"available":true,"label":null,"unit":"m/s","command":null,
      "initial":null,"minimum":null,"maximum":null}`);

    expect(none.command).toBeNull();
    expect(none.label).toBe('Speed');
    expect(speedReading(none)?.range ?? null).toBeNull();
  });

  it('a usable range needs a command and both bounds', () => {
    const usable = speedReading(
      guidedSpeed(
        json(`{"available":true,"command":"guidedModeChangeGroundSpeedMetersSecond",
          "label":"Speed","unit":"m/s","minimum":1.0,"maximum":20.0,"initial":5.0}`),
      ),
    );
    expect(usable?.range).toEqual({ start: 1.0, endInclusive: 20.0 });
    expect(speedReading(null)).toBeNull();
    expect(guidedSpeed(json('{"available":false}'))).toBeNull();
  });

  it('the argument path carries the target and no separators', () => {
    expect(guidedSpeedPath(7.5)).toBe('view.guidedSpeed(7.50)');
    expect(guidedSpeedPath(7.5).includes(',')).toBe(false);
  });
});
