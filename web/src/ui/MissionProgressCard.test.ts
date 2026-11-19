import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { missionProgress, missionProgressLine } from './MissionProgressCard';

const json = (text: string): Json => parseJson(text) ?? {};

describe('MissionProgressCardTest', () => {
  it("the card reads the core's progress and offers the next waypoint until the last", () => {
    const view = json('{"shown":true,"current":3,"last":6,"fraction":0.5,"distanceToNext":"84","distanceUnits":"m","canSkip":true,"skipTo":4}');
    const progress = missionProgress(view)!;
    expect(missionProgressLine(progress)).toBe('To waypoint 3 of 6 · 84 m');
    expect(progress.skipTo).toBe(4);
    expect(missionProgress({ ...view, shown: false })).toBeNull();
    expect(missionProgress(json('{"shown":true,"current":6,"last":6,"canSkip":false,"skipTo":6}'))!.skipTo).toBeNull();
  });

  it('the card counts the plan rows the pilot sees, so a takeoff after a camera setting is the takeoff, row 1 of 3, not waypoint 2 of 140', () => {
    const view = json('{"shown":true,"current":2,"last":140,"row":1,"rows":3,"rowName":"takeoff","fraction":0.01,"canSkip":true,"skipTo":3}');
    expect(missionProgressLine(missionProgress(view)!)).toBe('To takeoff 1 of 3');
    expect(missionProgress(view)!.skipTo, 'skipping still names the vehicle command after this one').toBe(3);
  });
});
