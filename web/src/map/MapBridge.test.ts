import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { MapBridge, coordinateOf } from './MapBridge';

describe('MapBridgeWatchTest', () => {
  const log = { sent: [] as readonly string[] };
  const original = MapBridge.sendWatch;

  beforeEach(() => {
    log.sent = [];
    MapBridge.sendWatch = (csv) => {
      log.sent = [...log.sent, csv];
    };
  });

  afterEach(() => {
    MapBridge.sendWatch = original;
    MapBridge.forgetWatchesForTest();
  });

  it('a map leaving the screen keeps the paths another map still watches', () => {
    MapBridge.watch('view.plan');
    MapBridge.watch('view.missionKinds');
    MapBridge.watch('view.plan');
    MapBridge.unwatch('view.plan');
    expect(MapBridge.watchedPathsForTest()).toEqual(new Set(['view.plan', 'view.missionKinds']));
  });

  it('the last holder of a path unwatches it', () => {
    MapBridge.watch('view.missionSummary');
    MapBridge.unwatch('view.missionSummary');
    expect(MapBridge.watchedPathsForTest()).toEqual(new Set());
    expect(log.sent[log.sent.length - 1]).toBe('');
  });

  it('a second holder of a path sends nothing new', () => {
    MapBridge.watch('view.missionSummary');
    MapBridge.watch('view.missionSummary');
    expect(log.sent).toEqual(['view.missionSummary']);
  });
});

describe('HomePositionTest', () => {
  const coordinate = (latitude: number, longitude: number, valid = true) => ({ kind: 'coordinate', valid, latitude, longitude });

  it('a valid coordinate is read', () => {
    const point = coordinateOf(coordinate(-35.363262, 149.165237));
    expect(point?.latitude).toBeCloseTo(-35.363262, 9);
    expect(point?.longitude).toBeCloseTo(149.165237, 9);
  });

  it('a coordinate the vehicle has not established is ignored', () => {
    expect(coordinateOf(coordinate(0.0, 0.0, false))).toBeNull();
    expect(coordinateOf(coordinate(-35.36, 149.16, false))).toBeNull();
  });

  it('null island is not a home position even when flagged valid', () => {
    expect(coordinateOf(coordinate(0.0, 0.0))).toBeNull();
  });

  it('an absent or unrelated payload yields nothing', () => {
    expect(coordinateOf(null)).toBeNull();
    expect(coordinateOf({ kind: 'null' })).toBeNull();
  });
});
