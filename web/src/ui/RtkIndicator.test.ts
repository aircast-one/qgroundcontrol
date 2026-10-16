import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { RtkStatus, basePositionWrites, rtkHeadline, rtkRows, rtkStatus } from './RtkIndicator';

describe('RtkIndicatorTest', () => {
  it('a disconnected base shows nothing', () => {
    expect(rtkStatus(parseJson('{"connected":false}'))).toBeNull();
    expect(rtkStatus(null)).toBeNull();
  });

  it("a survey in progress reads like QGC's RTK section", () => {
    const status = rtkStatus(parseJson('{"connected":true,"active":true,"valid":false,"numSatellites":17,"currentDuration":42.0,"currentAccuracy":3.456,"currentAccuracyText":"11.3 ft"}'))!;
    expect(rtkHeadline(status)).toBe('Survey-in Active');
    expect(rtkRows(status)).toEqual([
      ['Satellites', '17'],
      ['Duration', '42 s'],
      ['Current Accuracy', '11.3 ft'],
    ]);
  });

  it('a finished survey streams and hides an accuracy it has not reported', () => {
    const status = rtkStatus(parseJson('{"connected":true,"active":false,"valid":true,"numSatellites":null,"currentDuration":null,"currentAccuracy":null}'))!;
    expect(rtkHeadline(status)).toBe('RTK Streaming');
    expect(rtkRows(status)).toEqual([
      ['Satellites', ''],
      ['Duration', '0 s'],
    ]);
  });

  it('the surveyed position is saved as the fixed base only once the survey is valid', () => {
    const surveying = RtkStatus({ active: true, valid: false, satellites: 9, durationS: 10.0, accuracyM: 3.0, latitude: 47.1, longitude: 8.5, altitudeM: 400.0 });
    expect(basePositionWrites(surveying)).toBeNull();
    expect(basePositionWrites({ ...surveying, valid: true })).toEqual([
      ['settings.rtkSettings.fixedBasePositionLatitude', 47.1],
      ['settings.rtkSettings.fixedBasePositionLongitude', 8.5],
      ['settings.rtkSettings.fixedBasePositionAltitude', 400.0],
      ['settings.rtkSettings.fixedBasePositionAccuracy', 3.0],
    ]);
    expect(basePositionWrites({ ...surveying, valid: true, latitude: null })).toBeNull();
  });
});
