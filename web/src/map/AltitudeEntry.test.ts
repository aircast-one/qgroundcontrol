import { describe, expect, it } from 'vitest';
import { LATITUDE_LIMIT, LONGITUDE_LIMIT, RALLY_ALTITUDE_DECIMALS, altitudeFieldText, metresPerUnit, parsedAltitude, parsedCoordinate, parsedSurfaceDistance } from './AltitudeEntry';

describe('AltitudeEntryTest', () => {
  it('a plain number is an altitude', () => {
    expect(parsedAltitude('47')).toBeCloseTo(47.0, 9);
    expect(parsedAltitude(' 47.5 ')).toBeCloseTo(47.5, 9);
    expect(parsedAltitude('0')).toBeCloseTo(0.0, 9);
  });

  it('nonsense and half typed values are refused rather than sent', () => {
    expect(parsedAltitude('')).toBeNull();
    expect(parsedAltitude('.')).toBeNull();
    expect(parsedAltitude('4x')).toBeNull();
  });

  it('below the launch point and a comma decimal are altitudes, as SimpleMissionItem sets no minimum', () => {
    expect(parsedAltitude('-5')).toBeCloseTo(-5.0, 9);
    expect(parsedAltitude('45,5')).toBeCloseTo(45.5, 9);
    expect(parsedAltitude('1,000.5'), 'both separators is ambiguous').toBeNull();
    expect(parsedAltitude('1,000,000')).toBeNull();
  });

  it('the field keeps the decimals QGC shows, so Done on an untouched field writes the same height back', () => {
    expect(altitudeFieldText(75.0, 1)).toBe('75');
    expect(altitudeFieldText(45.5, 1)).toBe('45.5');
    expect(altitudeFieldText(164.042, RALLY_ALTITUDE_DECIMALS)).toBe('164.04');
    expect(altitudeFieldText(NaN, 1)).toBe('');
    expect(altitudeFieldText(-0.04, 1)).toBe('0');
  });

  it('a survey keeps CameraCalc\'s 0_1 m floor above the surface', () => {
    expect(parsedSurfaceDistance('0', 1.0)).toBeNull();
    expect(parsedSurfaceDistance('0,1', 1.0)).toBeCloseTo(0.1, 9);
    expect(parsedSurfaceDistance('0.2', metresPerUnit('ft')), '0.2 ft is under the 0.1 m floor').toBeNull();
  });

  it('a rally coordinate is typed within the globe, like RallyPoint\'s textFieldFacts', () => {
    expect([parsedCoordinate('47.39', LATITUDE_LIMIT), parsedCoordinate('91', LATITUDE_LIMIT), parsedCoordinate('-180', LONGITUDE_LIMIT), parsedCoordinate('x', LONGITUDE_LIMIT)]).toEqual([47.39, null, -180.0, null]);
    expect(parsedCoordinate('-33,87', LATITUDE_LIMIT)).toBeCloseTo(-33.87, 9);
  });
});
