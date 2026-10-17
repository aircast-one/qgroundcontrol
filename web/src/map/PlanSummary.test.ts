import { describe, expect, it } from 'vitest';
import { FenceCircle } from './FenceBridge';
import { MissionItem } from './PlanBridge';
import { selectionText } from './PlanSummary';
import { TrackPoint } from './VehicleMap';

const item = (index = 0, altitude = NaN) =>
  MissionItem({ index, sequence: index + 1, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude, altitudeText: Number.isNaN(altitude) ? '' : `${Math.trunc(altitude)} m` });

describe('PlanSummaryTest', () => {
  it('a selected waypoint shows its altitude against the number on its marker', () => {
    expect(
      selectionText({ kind: 'Waypoint', index: 3 }, [item(3, 75.0)], [], []),
      'the map marker and the list row are labelled with the sequence, so naming the index here would point at a different item once a survey is in the plan',
    ).toBe('#4 at 75 m');
  });

  it('a selected circle shows its radius from either handle, spelled by the core', () => {
    const circles = [FenceCircle({ index: 1, inclusion: true, centre: TrackPoint(41.0, 44.0), radius: 136.0, detailText: '446 ft radius' })];

    expect(selectionText({ kind: 'Circle', index: 1 }, [], circles, [])).toBe('446 ft radius');
    expect(selectionText({ kind: 'CircleCentre', index: 1 }, [], circles, [])).toBe('446 ft radius');
  });

  it('a selection with nothing to say adds nothing', () => {
    expect(selectionText({ kind: 'Waypoint', index: 0 }, [item()], [], [])).toBeNull();
  });
});
