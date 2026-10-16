import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { holdTakeoffHeight } from './FlyDeck';
import { type GuidedTakeoff, guidedTakeoff, guidedTakeoffPath, takeoffRangeUsable } from './GuidedTakeoff';

const json = (text: string): Json => parseJson(text) ?? {};

const served = (text: string) => {
  const takeoff = guidedTakeoff(json(text));
  if (takeoff === null) throw new Error('the fixture is an available takeoff');
  return takeoff;
};

describe('GuidedTakeoffTest', () => {
  it('the served takeoff carries its range, its sentence and the metric target', () => {
    const takeoff = served(`{"available":true,"label":"Takeoff altitude","unit":"m",
      "initial":10.0,"minimum":3.0,"maximum":121.0,"minimumMeters":3.0,
      "target":10.0,"targetMeters":10.0,
      "sentence":"The aircraft will climb to 10.0 m and hold."}`);

    expect(takeoff.initial).toBeCloseTo(10.0, 6);
    expect(takeoff.targetMeters).toBeCloseTo(10.0, 6);
    expect(takeoff.sentence).toBe('The aircraft will climb to 10.0 m and hold.');
    expect(takeoffRangeUsable(takeoff)).toBe(true);
  });

  it('a vehicle with no range yet offers no slider', () => {
    const takeoff = served(`{"available":true,"label":"Takeoff altitude","unit":"m",
      "initial":null,"minimum":null,"maximum":null,"minimumMeters":null}`);

    expect(takeoff.initial).toBeNull();
    expect(takeoffRangeUsable(takeoff)).toBe(false);
  });

  it('an unavailable takeoff is no dialog', () => {
    expect(guidedTakeoff(null)).toBeNull();
    expect(guidedTakeoff(json('{"available":false}'))).toBeNull();
    expect(takeoffRangeUsable(null)).toBe(false);
  });

  it('the argument path carries the target and no separators', () => {
    expect(guidedTakeoffPath(10.0)).toBe('view.guidedTakeoff(10.00)');
    expect(guidedTakeoffPath(10.0).includes(',')).toBe(false);
  });

  it('holding Takeoff climbs to the default height and refuses a vehicle with no range', () => {
    const takeoff = (initial: number | null, minimum: number | null, maximum: number | null): GuidedTakeoff => ({
      label: 'Takeoff altitude',
      unit: 'm',
      initial,
      minimum,
      maximum,
      sentence: '',
      targetMeters: 10.0,
    });
    expect(holdTakeoffHeight(takeoff(10.0, 3.0, 121.0))).toBe(10.0);
    expect(holdTakeoffHeight(takeoff(10.0, null, 121.0))).toBeNull();
    expect(holdTakeoffHeight(takeoff(10.0, 5.0, 5.0))).toBeNull();
    expect(holdTakeoffHeight(null)).toBeNull();
  });
});
