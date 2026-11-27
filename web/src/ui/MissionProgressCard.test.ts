import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { missionProgress, missionProgressLine } from './MissionProgressCard';

const json = (text: string): Json => parseJson(text) ?? {};

describe('MissionProgressCardTest', () => {
  it("the card reads the core's progress and offers the next waypoint until the last", () => {
    const view = json('{"shown":true,"current":5,"last":9,"line":"To waypoint 3 of 6","fraction":0.5,"distanceToNext":"84","distanceUnits":"m","canSkip":true,"skipTo":6}');
    const progress = missionProgress(view)!;
    expect(missionProgressLine(progress)).toBe('To waypoint 3 of 6 · 84 m');
    expect(progress.skipTo, 'skipping names the vehicle command after this one, not the waypoint number').toBe(6);
    expect(missionProgress({ ...view, shown: false })).toBeNull();
    expect(missionProgress(json('{"shown":true,"current":6,"last":6,"canSkip":false,"skipTo":6}'))!.skipTo).toBeNull();
  });

  it('the card says what the core says, so a takeoff after a camera setting reads as the takeoff, not waypoint 2 of 140', () => {
    const view = json('{"shown":true,"current":2,"last":140,"line":"To takeoff","fraction":0.01,"canSkip":true,"skipTo":3}');
    expect(missionProgressLine(missionProgress(view)!)).toBe('To takeoff');
  });
});
