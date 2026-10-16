import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { altitudeDelta, guidedAltitude, guidedAltitudePath, rangeLabel } from './GuidedAltitude';
import { altitudeReading } from './GuidedValueFlow';

const json = (text: string): Json => parseJson(text) ?? {};

const served = (text: string) => {
  const reading = guidedAltitude(json(text));
  if (reading === null) throw new Error('the fixture is an available altitude');
  return reading;
};

describe('GuidedAltitudeTest', () => {
  it('the served reading carries the sentence and the metric delta', () => {
    const reading = served(`{"available":true,"label":"Alt (Rel)","unit":"m","current":25.0,
      "currentMeters":25.0,"minimum":0.0,"maximum":121.0,"target":68.5,
      "targetMeters":68.5,"delta":43.5,"deltaMeters":43.5,"sends":true,
      "sentence":"The aircraft will climb 43.5 m to 68.5 m."}`);

    expect(reading.sentence).toBe('The aircraft will climb 43.5 m to 68.5 m.');
    expect(reading.deltaMeters).toBeCloseTo(43.5, 6);
    expect(reading.sends).toBe(true);
  });

  it('a vehicle with no altitude yet reads as absent, not as zero', () => {
    const reading = served(`{"available":true,"label":"Alt (Rel)","unit":"m","current":null,
      "currentMeters":null,"minimum":null,"maximum":null}`);

    expect(reading.current).toBeNull();
    expect(reading.minimum).toBeNull();
    expect(reading.maximum).toBeNull();
    expect(altitudeReading(reading)?.range ?? null).toBeNull();
  });

  it('an unavailable view offers nothing', () => {
    expect(guidedAltitude(null)).toBeNull();
    expect(guidedAltitude(json('{"available":false}'))).toBeNull();
    expect(altitudeReading(null)).toBeNull();
  });

  it('the argument path carries the target and no stray separators', () => {
    expect(guidedAltitudePath(68.5)).toBe('view.guidedAltitude(68.50)');
    expect(guidedAltitudePath(68.5).includes(',')).toBe(false);
  });

  it('a pause asks the core for the pause intent', () => {
    expect(guidedAltitudePath(25.0, true)).toBe('view.guidedAltitude(25.00,pause)');
  });

  it('the command carries the change, never the height the operator typed', () => {
    const reading = served(`{"kind":"object","class":"GuidedAltitude","available":true,"label":"Altitude","unit":"m",
      "current":25.0,"minimum":0.0,"maximum":120.0,"target":68.5,
      "targetMeters":68.5,"delta":43.5,"deltaMeters":43.5,"sends":true,
      "pause":false,"sentence":"The aircraft will climb 43.5 m to 68.5 m."}`);
    expect(altitudeDelta(reading), 'guidedModeChangeAltitude takes a delta; sending 68.5 here flies it to 93.5 m').toBeCloseTo(43.5, 9);
  });

  it('a change too small to matter sends nothing at all', () => {
    const standing = served(`{"kind":"object","class":"GuidedAltitude","available":true,"label":"Altitude","unit":"m",
      "current":25.0,"minimum":0.0,"maximum":120.0,"target":25.0,
      "targetMeters":25.0,"delta":0.0,"deltaMeters":0.0,"sends":false,
      "pause":false,"sentence":"The aircraft is already at 25 m and will not move."}`);
    expect(altitudeDelta(standing)).toBeNull();
  });
});

describe('RangeLabelTest', () => {
  it('a slider says what its ends are', () => {
    expect(rangeLabel(3.0, 121.92, 'm')).toBe('3.0 to 121.9 m');
    expect(rangeLabel(0.1, 5.0, 'm/s')).toBe('0.1 to 5.0 m/s');
  });

  it('a range with no unit still reads', () => {
    expect(rangeLabel(1.0, 8.0, '')).toBe('1.0 to 8.0');
  });

  it('a range that is not a range says nothing', () => {
    expect(rangeLabel(null, 5.0, 'm')).toBeNull();
    expect(rangeLabel(1.0, null, 'm')).toBeNull();
    expect(rangeLabel(5.0, 5.0, 'm')).toBeNull();
    expect(rangeLabel(9.0, 5.0, 'm')).toBeNull();
  });

  it('an altitude a hair below launch reads as zero, not as minus zero', () => {
    expect(rangeLabel(-0.03, 60.0, 'm')).toBe('0.0 to 60.0 m');
  });
});
