import { describe, expect, it } from 'vitest';
import { FenceCircle, FencePolygon } from './FenceBridge';
import { fenceWindow } from './PlanMapContent';
import { selectedFence } from './SelectionValidity';

const polygon = (index: number, inclusion: boolean) =>
  FencePolygon({ index, inclusion, vertices: Array.from({ length: 4 }, (_, it) => ({ latitude: 41.0 + it, longitude: 44.0 })), kindText: `Polygon ${index + 1}` });

const circle = (index: number, inclusion: boolean) => FenceCircle({ index, inclusion, centre: { latitude: 41.0, longitude: 44.0 }, radius: 150.0, kindText: `Circle ${index + 1}` });

describe('FenceInclusionTest', () => {
  it('a keep-in fence offers to become keep-out, and the other way', () => {
    const keepIn = selectedFence({ kind: 'FenceVertex', polygon: 0, vertex: 0 }, [polygon(0, true)], []);
    const keepOut = selectedFence({ kind: 'FenceVertex', polygon: 0, vertex: 0 }, [polygon(0, false)], []);
    expect(keepIn?.keepsIn).toBe(true);
    expect(keepOut?.keepsIn).toBe(false);
  });

  it('a circle is named by its edge or its centre and only described, like GeoFenceEditor', () => {
    const circles = [circle(2, false)];
    expect(selectedFence({ kind: 'Circle', index: 2 }, [], circles)?.keepsIn).toBe(false);
    expect(selectedFence({ kind: 'CircleCentre', index: 2 }, [], circles)?.keepsIn).toBe(false);
    expect(selectedFence({ kind: 'Circle', index: 2 }, [], circles)?.flip).toBeNull();
    expect(selectedFence({ kind: 'FenceVertex', polygon: 0, vertex: 0 }, [polygon(0, true)], [])?.flip != null).toBe(true);
  });

  it('a selection that is not a fence offers no flip', () => {
    expect(selectedFence({ kind: 'Waypoint', index: 0 }, [polygon(0, true)], [])).toBeNull();
    expect(selectedFence(null, [polygon(0, true)], [])).toBeNull();
    expect(selectedFence({ kind: 'FenceVertex', polygon: 9, vertex: 0 }, [polygon(0, true)], [])).toBeNull();
  });

  it("a new fence is sized from the visible map, like GeoFenceEditor's viewport corners", () => {
    const view = [
      { latitude: 48.0, longitude: 8.0 },
      { latitude: 48.0, longitude: 9.0 },
      { latitude: 47.0, longitude: 9.0 },
      { latitude: 47.0, longitude: 8.0 },
    ];
    expect(fenceWindow(view, { latitude: 47.5, longitude: 8.5 })).toEqual([
      { latitude: 48.0, longitude: 8.0 },
      { latitude: 47.0, longitude: 9.0 },
    ]);
    expect(fenceWindow([], { latitude: 47.5, longitude: 8.5 })).toEqual([
      { latitude: 47.502, longitude: 8.498 },
      { latitude: 47.498, longitude: 8.502 },
    ]);
  });
});
