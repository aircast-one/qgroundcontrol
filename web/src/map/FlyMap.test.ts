import { describe, expect, it } from 'vitest';
import { centresOnLaunch, centresOnOperator, firstKnown, keepsZoom, launchOf, missionArrived, otherMissions, pipZoom, showsWholeWorld } from './FlyMap';
import { MissionItem } from './PlanBridge';

describe('FlyMapFitTest', () => {
  it('the fly map fits whenever a new mission arrives from the vehicle, as onNewItemsFromVehicle does', () => {
    const same = (it: number) => it;
    expect(missionArrived([], [1, 2], same)).toBe(true);
    expect(missionArrived([1], [1, 2], same)).toBe(true);
    expect(missionArrived([1, 2], [1, 2], same)).toBe(false);
    expect(missionArrived([1], [], same)).toBe(false);
  });

  it('the map centres once on the operator when no vehicle has a position, as FlightMap does on the first GCS fix', () => {
    const here = { latitude: 47.0, longitude: 8.0 };
    expect(centresOnOperator(false, here, false)).toBe(true);
    expect(centresOnOperator(true, here, false)).toBe(false);
    expect(centresOnOperator(false, here, true)).toBe(false);
    expect(centresOnOperator(false, null, false)).toBe(false);
  });

  it('a map still showing the whole world opens on the home, else the launch of the flown mission, else nothing it does not know', () => {
    const home = { latitude: 41.72, longitude: 44.88 };
    const launch = { latitude: 41.7, longitude: 44.8 };
    const unset = { latitude: 0, longitude: 0 };
    const items = [MissionItem({ index: 0, sequence: 0, latitude: launch.latitude, longitude: launch.longitude, command: 'Launch', selected: false }), MissionItem({ index: 1, sequence: 1, latitude: 41.8, longitude: 44.9, command: 'Waypoint', selected: false })];
    expect(showsWholeWorld(0)).toBe(true);
    expect(showsWholeWorld(15)).toBe(false);
    expect(launchOf(items)).toEqual(launch);
    expect(firstKnown([home, launch])).toEqual(home);
    expect(firstKnown([null, unset, launch]), 'an unset home reads 0, 0, which is no place').toEqual(launch);
    expect(firstKnown([null, null])).toBeNull();
  });

  it('the map goes to the home or launch once, only when it opened on the whole world with no remembered view and no aircraft on it', () => {
    expect(centresOnLaunch(false, false, 0.4, false)).toBe(true);
    expect(centresOnLaunch(false, false, null, false), 'a map that has not reported its zoom yet is not known to show the whole world').toBe(false);
    expect(centresOnLaunch(false, true, 0.4, false), 'a remembered view goes first').toBe(false);
    expect(centresOnLaunch(true, false, 0.4, false), 'zooming back out to the world later does not snap to the launch again').toBe(false);
    expect(centresOnLaunch(false, false, 0.4, true), 'an aircraft with a position is followed instead').toBe(false);
    expect(centresOnLaunch(false, false, 12, false)).toBe(false);
  });

  it("other vehicles' missions come from the fly view, as FlyViewMap repeats PlanMapItems for every vehicle", () => {
    const view = { items: [], others: [{ linksStartToHome: true, items: [] }, { items: [] }] };
    expect(otherMissions(view).map((it) => it.linkStartToHome)).toEqual([true, false]);
    expect(otherMissions(null)).toEqual([]);
  });
});

describe('PipZoomTest', () => {
  it('the small map zooms out by three and the main map returns to its own zoom like FlyViewMap', () => {
    expect(pipZoom(15.0, true)).toBe(12.0);
    expect(pipZoom(2.5, true)).toBeNull();
    expect(pipZoom(15.0, false)).toBe(15.0);
    expect(pipZoom(0.0, false)).toBeNull();
  });

  it("the zoomed out small map level is never kept as the main map's zoom", () => {
    expect(keepsZoom(15.0, false, false)).toBe(true);
    expect(keepsZoom(12.0, true, true)).toBe(false);
    expect(keepsZoom(12.0, false, true)).toBe(false);
    expect(keepsZoom(0.5, false, false)).toBe(false);
  });
});
