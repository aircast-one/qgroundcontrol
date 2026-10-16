import { describe, expect, it } from 'vitest';
import { centresOnOperator, keepsZoom, missionArrived, otherMissions, pipZoom } from './FlyMap';

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
