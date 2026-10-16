import { describe, expect, it } from 'vitest';
import type { PlanStat } from '@/map/PlanStrip';
import { withoutHeroFields } from '@/ui/ItemEditor';
import { factFromControl } from '@/ui/ParameterForm';
import { headerLine, planStatsLine } from '@/ui/PlanTab';
import { routeAltitude } from './RouteSettings';

const stats: PlanStat[] = [
  { label: 'Items', value: '3' },
  { label: 'Distance', value: '93 m' },
  { label: 'Time', value: '00:35' },
  { label: 'Max alt', value: '50.0 m' },
];

const fact = (label: string) => factFromControl({ label, path: label })!;

describe('RouteSettingsTest', () => {
  it('the title pill reads the route at a glance', () => {
    expect(planStatsLine(stats)).toBe('3 items · 93 m · 00:35 · max 50.0 m');
    expect(planStatsLine([{ label: 'Items', value: '1' }])).toBe('1 item');
    expect(planStatsLine([{ label: 'Items', value: '0' }])).toBe('');
  });

  it("a transfer in flight takes the line, and an empty plan falls back to the core's status", () => {
    expect(headerLine('Uploading', true, stats)).toBe('Uploading');
    expect(
      headerLine('Empty plan', false, [
        { label: 'Items', value: '0' },
        { label: 'Distance', value: '0 m' },
        { label: 'Time', value: '00:00' },
      ]),
    ).toBe('Empty plan');
  });

  it('the route altitude is the default every new waypoint takes', () => {
    const plan = { defaults: { altitude: { label: 'Default altitude', path: 'settings.appSettings.defaultMissionItemAltitude', value: 60.0, units: 'm' } } };
    expect(routeAltitude(plan)).toEqual({ value: 60.0, units: 'm', path: 'settings.appSettings.defaultMissionItemAltitude' });
    expect(routeAltitude({})).toBeNull();
  });

  it('the hold has its own row, so the editor below does not repeat it', () => {
    const hold = fact('Hold');
    const accept = fact('Acceptance radius');
    expect(withoutHeroFields([hold, accept], { hold: { value: 0, units: 's', path: 'p' } })).toEqual([accept]);
    expect(withoutHeroFields([hold, accept], { hold: null })).toEqual([hold, accept]);
  });
});
