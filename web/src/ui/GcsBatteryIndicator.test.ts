import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { gcsBatteryPath, gcsBatteryReading, phoneBattery } from './GcsBatteryIndicator';

describe('GcsBatteryIndicatorTest', () => {
  it('the phone battery is asked of the core as a percentage', () => {
    expect(phoneBattery(54, 100, 3)).toEqual({ percent: 54, charging: false });
    expect(phoneBattery(128, 256, 2)).toEqual({ percent: 50, charging: true });
    expect(phoneBattery(-1, 100, 2)).toBeNull();
    expect(gcsBatteryPath({ percent: 54, charging: false })).toBe('view.gcsBattery(54,false)');
    const reading = gcsBatteryReading(parseJson('{"state":"low","levelText":"20%","stateText":"On battery","heading":"Ground Station","title":"Ground station battery"}'))!;
    expect(reading.state).toBe('low');
    expect(reading.title).toBe('Ground station battery');
  });
});
