import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { altitudeModesView } from '@/map/AltitudeModes';
import { globalAltitudeFrame, missionFrameChoice, missionFramePicks } from './MissionAltitudeFrame';

const json = (text: string): Json => parseJson(text)!;

const ITEMS_ADDED = `
{"class": "AltitudeModes", "context": "mission", "current": 1, "modes": [
  {"raw": 1, "title": "Relative To Launch", "enabled": true, "current": true},
  {"raw": 2, "title": "AMSL", "enabled": false, "current": false, "reason": "locked"},
  {"raw": 0, "title": "Mixed Modes", "enabled": true, "current": false}
 ], "omitted": []}
`;

const MIXED_ONLY = `
{"class": "AltitudeModes", "context": "mission", "current": 0, "modes": [
  {"raw": 1, "title": "Relative To Launch", "enabled": false, "current": false},
  {"raw": 0, "title": "Mixed Modes", "enabled": true, "current": true}
 ], "omitted": []}
`;

describe('MissionAltitudeFrameTest', () => {
  it('the mission frame is read from the plan view', () => {
    expect(globalAltitudeFrame(json('{"globalAltitudeFrame":1}'))).toBe(1);
    expect(globalAltitudeFrame(json('{"globalAltitudeFrame":null}'))).toBeNull();
  });

  it("mixed stays pickable for the plan once items exist, like MissionSettingsEditor's AltModeMenu", () => {
    const view = altitudeModesView(json(ITEMS_ADDED));
    expect(missionFramePicks(view).map((it) => it.raw)).toEqual([1, 2, 0]);
    expect(missionFrameChoice(view)).toBe(true);
  });

  it('only the current mode enabled leaves nothing to pick', () => {
    expect(missionFrameChoice(altitudeModesView(json(MIXED_ONLY)))).toBe(false);
  });
});
