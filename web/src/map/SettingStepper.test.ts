import { describe, expect, it } from 'vitest';
import { ItemRow } from '@/map/PlanItems';
import { MissionItem } from '@/map/PlanBridge';
import { altitudeNote, altitudeRange, stripRows } from '@/map/WaypointPanel';
import { clampNote, stepperDecimals, stepperText, steppedValue } from './SettingStepper';

describe('SettingStepperTest', () => {
  it('a step moves by its size and stops at the ends of the range', () => {
    const range = { start: 0.0, endInclusive: 500.0 };
    expect([steppedValue(50.0, 1.0, 1, range), steppedValue(50.0, 1.0, -1, range), steppedValue(500.0, 1.0, 1, range), steppedValue(0.0, 1.0, -1, range), steppedValue(-2.0, 1.0, -1, null)]).toEqual([51.0, 49.0, 500.0, 0.0, -3.0]);
  });

  it('whole numbers read whole and a half step shows its decimal', () => {
    expect([stepperDecimals(50.0, 1.0), stepperDecimals(5.0, 0.5), stepperDecimals(45.5, 1.0)]).toEqual([0, 1, 1]);
    expect([stepperText(50.0, 0), stepperText(5.5, 1), stepperText(null, 0)]).toEqual(['50', '5.5', '—']);
  });

  it('the altitude slider spans the same height in either unit', () => {
    expect(altitudeRange('m').endInclusive).toBe(500.0);
    expect(altitudeRange('ft').endInclusive).toBe(1640.0);
  });

  it('an AMSL altitude is measured from the launch elevation, not from sea level', () => {
    expect(altitudeRange('m', 2, 584.0)).toEqual({ start: 584.0, endInclusive: 1084.0 });
    expect(altitudeRange('m', 1, 584.0)).toEqual({ start: 0.0, endInclusive: 500.0 });
    expect(altitudeRange('m', 2, NaN)).toEqual({ start: 0.0, endInclusive: 500.0 });
    const amsl = MissionItem({ index: 3, sequence: 3, latitude: 0, longitude: 0, command: 'Waypoint', selected: true, altitudeMode: 2, altitudeFrameText: 'AMSL', altitudeEditUnits: 'm' });
    expect(altitudeNote(amsl, 584.0)).toBe('AMSL · launch is 584 m');
    expect(altitudeNote({ ...amsl, altitudeMode: 1, altitudeFrameText: '' }, 584.0)).toBeNull();
  });

  it('a typed value outside the range says it was capped instead of changing silently', () => {
    const range = { start: 584.0, endInclusive: 1084.0 };
    expect(clampNote(1650.0, range, 'm')).toBe('Capped at 1,084 m, the highest allowed.');
    expect(clampNote(50.0, range, 'm')).toBe('Raised to 584 m, the lowest allowed.');
    expect([clampNote(650.0, range, 'm'), clampNote(null, range, 'm'), clampNote(9000.0, null, 'm')]).toEqual([null, null, null]);
  });

  it('the waypoint strip leaves out the planned home, which is not flown to', () => {
    const rows = [0, 1, 2].map((it) => ItemRow({ index: it, number: `${it}`, name: 'Waypoint', detail: '', colour: '#ffffff', placed: true }));
    expect(stripRows(rows).map((it: { index: number }) => it.index)).toEqual([1, 2]);
  });
});
