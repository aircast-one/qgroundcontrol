import { describe, expect, it } from 'vitest';
import { ItemRow } from '@/map/PlanItems';
import { altitudeRange, stripRows } from '@/map/WaypointPanel';
import { stepperDecimals, stepperText, steppedValue } from './SettingStepper';

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

  it('the waypoint strip leaves out the planned home, which is not flown to', () => {
    const rows = [0, 1, 2].map((it) => ItemRow({ index: it, number: `${it}`, name: 'Waypoint', detail: '', colour: '#ffffff', placed: true }));
    expect(stripRows(rows).map((it: { index: number }) => it.index)).toEqual([1, 2]);
  });
});
