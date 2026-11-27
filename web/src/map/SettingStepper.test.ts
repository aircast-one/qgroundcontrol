import { describe, expect, it } from 'vitest';
import { ItemRow } from '@/map/PlanItems';
import { MissionItem } from '@/map/PlanBridge';
import { altitudeMetres, altitudeNote, altitudeRange, launchIn, stripRows } from '@/map/WaypointPanel';
import { clampNote, stepperDecimals, stepperText, steppedValue } from './SettingStepper';

describe('SettingStepperTest', () => {
  it('a step moves by its size and stops at the ends of the range', () => {
    const range = { start: 0.0, endInclusive: 500.0 };
    expect([steppedValue(50.0, 1.0, 1, range), steppedValue(50.0, 1.0, -1, range), steppedValue(500.0, 1.0, 1, range), steppedValue(0.0, 1.0, -1, range), steppedValue(-2.0, 1.0, -1, null)]).toEqual([51.0, 49.0, 500.0, 0.0, -3.0]);
  });

  it('a step from outside the range moves one step toward it and never jumps to the bound', () => {
    const range = { start: 585.0, endInclusive: 1085.0 };
    expect([steppedValue(560.0, 1.0, -1, range), steppedValue(560.0, 1.0, 1, range)]).toEqual([560.0, 561.0]);
    expect([steppedValue(1200.0, 1.0, 1, range), steppedValue(1200.0, 1.0, -1, range)]).toEqual([1200.0, 1199.0]);
  });

  it('whole numbers read whole and a half step shows its decimal', () => {
    expect([stepperDecimals(50.0, 1.0), stepperDecimals(5.0, 0.5), stepperDecimals(45.5, 1.0)]).toEqual([0, 1, 1]);
    expect([stepperText(50.0, 0), stepperText(5.5, 1), stepperText(null, 0)]).toEqual(['50', '5.5', '—']);
  });

  it('the altitude slider spans the same height in either unit', () => {
    expect(altitudeRange('m', 1, null)?.endInclusive).toBe(500.0);
    expect(altitudeRange('ft', 1, null)?.endInclusive).toBe(1640.0);
  });

  it('an AMSL waypoint can sit below the launch, as on a ridge-to-valley plan', () => {
    expect(altitudeRange('m', 2, 585.0)).toEqual({ start: 85.0, endInclusive: 1085.0 });
    expect(altitudeRange('m', 1, 585.0)).toEqual({ start: 0.0, endInclusive: 500.0 });
    expect(altitudeRange('m', 2, null)).toBeNull();
    expect(clampNote(560.0, altitudeRange('m', 2, 585.0), 'm')).toBeNull();
  });

  it('the launch is read in metres and shown in the waypoint units, whatever units the home was in', () => {
    const home = MissionItem({ index: 0, sequence: 0, latitude: 0, longitude: 0, command: 'Home', selected: false, altitude: 1919.29, altitudeEditUnits: 'ft' });
    expect(altitudeMetres(home)).toBeCloseTo(585.0, 1);
    expect(launchIn('m', home)).toBeCloseTo(585.0, 1);
    expect(launchIn('ft', MissionItem({ ...home, altitude: 585.0, altitudeEditUnits: 'm' }))).toBeCloseTo(1919.29, 1);
    expect([launchIn('m', null), launchIn('m', MissionItem({ ...home, altitude: NaN }))]).toEqual([null, null]);
    const amsl = MissionItem({ index: 3, sequence: 3, latitude: 0, longitude: 0, command: 'Waypoint', selected: true, altitudeMode: 2, altitudeFrameText: 'AMSL', altitudeEditUnits: 'm' });
    expect(altitudeNote(amsl, launchIn('m', home))).toBe('AMSL · launch is 585 m');
    expect(altitudeNote({ ...amsl, altitudeMode: 1, altitudeFrameText: '' }, 585.0)).toBeNull();
  });

  it('a typed value outside the range says it was capped instead of changing silently', () => {
    const range = { start: 85.0, endInclusive: 1085.0 };
    expect(clampNote(1650.0, range, 'm')).toBe('Capped at 1,085 m, the highest allowed.');
    expect(clampNote(50.0, range, 'm')).toBe('Raised to 85 m, the lowest allowed.');
    expect([clampNote(560.0, range, 'm'), clampNote(null, range, 'm'), clampNote(9000.0, null, 'm')]).toEqual([null, null, null]);
  });

  it('the waypoint strip leaves out the planned home, which is not flown to, and the actions, which are listed under their waypoint', () => {
    const rows = [0, 1, 2].map((it) => ItemRow({ index: it, number: `${it}`, name: 'Waypoint', detail: '', colour: '#ffffff', placed: true }));
    const action = ItemRow({ index: 3, number: '', name: 'Delay', detail: '', colour: '#ffffff', placed: false });
    expect(stripRows([...rows, action]).map((it: { index: number }) => it.index)).toEqual([1, 2]);
  });
});
