import { describe, expect, it } from 'vitest';
import { type Json, optJSONObject, parseJson } from '@/bridge/Json';
import { speedSectionOf } from './ItemEditor';
import { planDefaults, planDefaultsNote } from './PlanDefaults';

const control = (name: string, value: number, units: string) =>
  `{"kind":"object","class":"Control","control":"number","name":"${name}","label":"${name}",
            "path":"settings.appSettings.${name}","value":${value},"valueString":"${value}",
            "units":"${units}","enabled":true,"readOnly":false,"options":[],"bits":[]}`;

const plan = (defaults: string): Json => parseJson(`{"kind":"object","class":"Plan","defaults":${defaults}}`)!;

describe('PlanDefaultsTest', () => {
  it('the three the core serves are the three offered', () => {
    const facts = planDefaults(
      plan(`{"altitude":${control('defaultMissionItemAltitude', 50.0, 'm')},
                    "cruise":${control('offlineEditingCruiseSpeed', 15.0, 'm/s')},
                    "hover":${control('offlineEditingHoverSpeed', 5.0, 'm/s')},
                    "speedUnits":"m/s"}`),
    );
    expect(
      facts.map((it) => it.name),
      'the altitude every new mission item starts at, and the speeds a plan edited with no vehicle is flown at - none of which was on any screen, so an operator adding a waypoint got a height they could not see or change from the Plan tab',
    ).toEqual(['defaultMissionItemAltitude', 'offlineEditingCruiseSpeed', 'offlineEditingHoverSpeed']);
  });

  it("the order is the list's, not whatever order the keys arrive in", () => {
    const facts = planDefaults(
      plan(`{"hover":${control('hover', 5.0, 'm/s')},
                    "cruise":${control('cruise', 15.0, 'm/s')},
                    "altitude":${control('altitude', 50.0, 'm')}}`),
    );
    expect(
      facts.map((it) => it.name),
      'JSONObject.keys() has no defined order on Android, so height before speeds is what PLAN_DEFAULT_KEYS is for - the filtering is structural and would work without it',
    ).toEqual(['altitude', 'cruise', 'hover']);
  });

  it('a value that is not an object cannot become a control', () => {
    const facts = planDefaults(plan(`{"altitude":${control('a', 50.0, 'm')},"speedUnits":"ft/s"}`));
    expect(facts).toHaveLength(1);
    expect(facts[0]?.name).toBe('a');
  });

  it('a core too old to serve defaults says so rather than showing an empty dialog', () => {
    expect(planDefaults(parseJson('{"kind":"object"}'))).toEqual([]);
    expect(planDefaults(null)).toEqual([]);
    expect(planDefaultsNote(null, [])).toContain('does not report');
  });

  it("with defaults present the note is the core's speed note, like MissionSettingsEditor", () => {
    const view = plan(`{"altitude":${control('a', 50.0, 'm')},"speedNote":"Speeds are used to estimate mission time only. They do not change the flight speed."}`);
    expect(planDefaultsNote(view, planDefaults(view))).toBe('Speeds are used to estimate mission time only. They do not change the flight speed.');
    const noSpeeds = plan(`{"altitude":${control('a', 50.0, 'm')},"speedNote":null}`);
    expect(planDefaultsNote(noSpeeds, planDefaults(noSpeeds))).toBe('');
  });

  it("the mission flight speed is the settings item's speed section with SpeedSection's user range", () => {
    const view = plan(`{"flightSpeed":{"available":true,"specified":true,"value":7.5,"units":"m/s","slider":{"from":0.0,"to":30.0,"decimals":1},
            "path":"plan.missionController.visualItems.0.speedSection.flightSpeed","specifyPath":"plan.missionController.visualItems.0.speedSection.specifyFlightSpeed"}}`);
    const speed = speedSectionOf(optJSONObject(optJSONObject(view, 'defaults'), 'flightSpeed'))!;
    expect(speed.slider).toEqual({ from: 0, to: 30, decimals: 1, hint: '' });
    expect(speed.value).toBe(7.5);
    expect(speedSectionOf({ available: false })).toBeNull();
  });

  it('a default carrying the user range gets the slider MissionDefaultsEditor draws under its field', () => {
    const ranged = control('offlineEditingCruiseSpeed', 15.0, 'm/s').replace('"bits":[]', '"bits":[],"slider":{"from":1.0,"to":30.0,"decimals":1}');
    const facts = planDefaults(plan(`{"cruise":${ranged},"altitude":${control('defaultMissionItemAltitude', 50.0, 'm')}}`));
    expect(facts[facts.length - 1]?.slider).toEqual({ from: 1, to: 30, decimals: 1, hint: '' });
    expect(facts[0]?.slider).toBeNull();
  });

  it('a specified mission flight speed disables the cruise and hover defaults like MissionDefaultsEditor', () => {
    const defaults = (specified: boolean) =>
      `{"altitude":${control('defaultMissionItemAltitude', 50.0, 'm')},
                "cruise":${control('offlineEditingCruiseSpeed', 15.0, 'm/s')},
                "hover":${control('offlineEditingHoverSpeed', 5.0, 'm/s')},
                "ascent":${control('offlineEditingAscentSpeed', 5.0, 'm/s')},
                "flightSpeed":{"available":true,"specified":${specified},"value":7.5}}`;
    expect(planDefaults(plan(defaults(true))).map((it) => it.acceptsWrite)).toEqual([true, false, false, true]);
    expect(planDefaults(plan(defaults(false))).map((it) => it.acceptsWrite)).toEqual([true, true, true, true]);
  });
});
