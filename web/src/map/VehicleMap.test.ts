import { describe, expect, it } from 'vitest';
import type { Feature, Point } from 'geojson';
import { lastSeenText } from './Silence';
import { vehicleChoices } from './VehiclePicker';
import { ACTIVE_PROPERTY, HEADING_PROPERTY, STALE_PROPERTY, VEHICLE_LABEL_PROPERTY, clearAreaLift, fleetFeatures, outsideCentreInset, vehicleFeature, vehicleFeatures } from './VehicleMap';

const property = (feature: Feature | undefined, name: string): unknown => feature?.properties?.[name];

const hasProperty = (feature: Feature | undefined, name: string): boolean => feature?.properties !== null && feature?.properties !== undefined && name in feature.properties;

const longitudeOf = (feature: Feature | undefined): number => (feature?.geometry as Point | undefined)?.coordinates[0] ?? NaN;

describe('FleetFeaturesTest', () => {
  const two = vehicleChoices({
    ambiguous: true,
    vehicles: [
      { id: 1, name: 'Quadrotor 1', active: true, contactLost: false, heading: 90.0, home: { latitude: 41.69, longitude: -74.01 }, coordinate: { latitude: 41.7, longitude: -74.0 } },
      { id: 2, name: 'Quadrotor 2', active: false, contactLost: true, heading: null, home: null, coordinate: { latitude: 41.8, longitude: -74.1 } },
    ],
  }).choices;

  it('with more than one aircraft each is labelled with its system id', () => {
    const drawn = fleetFeatures(two).features;
    expect(property(drawn[0], VEHICLE_LABEL_PROPERTY)).toBe('Vehicle 1');
    expect(property(drawn[1], VEHICLE_LABEL_PROPERTY)).toBe('Vehicle 2');
    expect(hasProperty(fleetFeatures(two.slice(0, 1)).features[0], VEHICLE_LABEL_PROPERTY)).toBe(false);
  });

  it('a silent aircraft keeps its last place on the map and says how long ago that was', () => {
    expect(property(fleetFeatures(two.slice(0, 1), lastSeenText(7)).features[0], VEHICLE_LABEL_PROPERTY)).toBe('Last seen 7 s ago');
    const drawn = fleetFeatures(two, lastSeenText(90)).features;
    expect(property(drawn[0], VEHICLE_LABEL_PROPERTY)).toBe('Vehicle 1\nLast seen 1 min ago');
    expect(property(drawn[1], VEHICLE_LABEL_PROPERTY)).toBe('Vehicle 2');
  });

  it('every aircraft is drawn, not only the one being flown', () => {
    const drawn = fleetFeatures(two).features;
    expect(drawn.length).toBe(2);
    expect(longitudeOf(drawn[0])).toBeCloseTo(-74.0, 9);
    expect(longitudeOf(drawn[1])).toBeCloseTo(-74.1, 9);
  });

  it('each aircraft carries its own heading, and one with none is not drawn pointing north', () => {
    const drawn = fleetFeatures(two).features;
    expect(property(drawn[0], HEADING_PROPERTY)).toBeCloseTo(90.0, 9);
    expect(hasProperty(drawn[1], HEADING_PROPERTY)).toBe(false);
  });

  it('each aircraft carries its own home, and an invalid one is none', () => {
    expect(two[0]?.home).toEqual({ latitude: 41.69, longitude: -74.01 });
    expect(two[1]?.home).toBeNull();
  });

  it('each aircraft carries its own silence and its own place in the list', () => {
    const drawn = fleetFeatures(two).features;
    expect(property(drawn[0], STALE_PROPERTY)).toBe(false);
    expect(property(drawn[1], STALE_PROPERTY)).toBe(true);
    expect(property(drawn[0], ACTIVE_PROPERTY)).toBe(true);
    expect(property(drawn[1], ACTIVE_PROPERTY)).toBe(false);
  });

  it('an aircraft that has reported no position is left off rather than drawn at zero', () => {
    const unplaced = vehicleChoices({
      vehicles: [
        { id: 1, name: 'A', active: true, coordinate: { latitude: 41.7, longitude: -74.0 } },
        { id: 2, name: 'B', active: false, coordinate: null },
        { id: 3, name: 'C', active: false, coordinate: { latitude: 0, longitude: 0 } },
      ],
    }).choices;
    expect(fleetFeatures(unplaced).features.length).toBe(1);
  });

  it('no fleet draws nothing rather than a marker at nowhere', () => {
    expect(fleetFeatures([]).features.length).toBe(0);
  });
});

describe('VehicleMarkerTest', () => {
  it('a vehicle with a position is drawn', () => {
    const features = vehicleFeatures(-35.36, 149.16, 90.0).features;
    expect(features.length).toBe(1);
    expect(property(features[0], HEADING_PROPERTY)).toBeCloseTo(90.0, 9);
  });

  it('a vehicle that has gone away is not left on the map', () => {
    expect(vehicleFeatures(NaN, NaN, 90.0).features).toEqual([]);
    expect(vehicleFeatures(0.0, 0.0, 90.0).features).toEqual([]);
  });

  it('a position without heading still draws the dot', () => {
    const features = vehicleFeatures(-35.36, 149.16, NaN).features;
    expect(features.length).toBe(1);
    expect(hasProperty(features[0], HEADING_PROPERTY)).toBe(false);
  });
});

describe('VehicleHeadingTest', () => {
  it('a known heading rides along on the feature', () => {
    const feature = vehicleFeature(41.0, 44.0, 90.0);
    expect(hasProperty(feature, HEADING_PROPERTY)).toBe(true);
    expect(property(feature, HEADING_PROPERTY)).toBeCloseTo(90.0, 9);
  });

  it('an unknown heading leaves the property off so the arrow is filtered out', () => {
    expect(hasProperty(vehicleFeature(41.0, 44.0, NaN), HEADING_PROPERTY)).toBe(false);
  });

  it('headings are normalised into a single turn', () => {
    expect(property(vehicleFeature(41.0, 44.0, 450.0), HEADING_PROPERTY)).toBeCloseTo(90.0, 9);
    expect(property(vehicleFeature(41.0, 44.0, -90.0), HEADING_PROPERTY)).toBeCloseTo(270.0, 9);
  });

  it('the position still rides along without a heading', () => {
    const point = vehicleFeature(41.5, 44.5, NaN).geometry;
    expect(point.coordinates[1]).toBeCloseTo(41.5, 9);
    expect(point.coordinates[0]).toBeCloseTo(44.5, 9);
  });
});

describe('CentreInsetTest', () => {
  it('the map only recentres once the vehicle leaves the middle of the screen', () => {
    expect(outsideCentreInset(500, 500, 1000, 1000, 0, 0)).toBe(false);
    expect(outsideCentreInset(100, 500, 1000, 1000, 0, 0)).toBe(true);
    expect(outsideCentreInset(500, 700, 1000, 1000, 0, 200)).toBe(true);
    expect(outsideCentreInset(10, 10, 0, 0, 0, 0)).toBe(false);
  });

  it('theVehicleSitsInTheMiddleOfTheClearArea', () => {
    expect(clearAreaLift(300, 100)).toBe(100);
    expect(clearAreaLift(0, 0)).toBe(0);
  });
});
