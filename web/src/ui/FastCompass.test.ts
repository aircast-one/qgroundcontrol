import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { TrackPoint } from '@/map/VehicleMap';
import { fastCompass, fastCompassArguments, initialFastCompassChoice } from './FastCompass';

const json = (text: string): Json => parseJson(text) ?? {};

describe('FastCompassTest', () => {
  const withGcs = fastCompass(
    json('{"invocation":"sensorsCal.calibrateCompassNorth","help":"h","vehicleHasPosition":false,"gcsPosition":{"valid":true,"latitude":41.7,"longitude":44.8}}'),
  )!;
  const withoutGcs = fastCompass(
    json('{"invocation":"sensorsCal.calibrateCompassNorth","help":"h","vehicleHasPosition":true,"gcsPosition":{"valid":false,"latitude":null,"longitude":null}}'),
  )!;

  it('a valid gcs position is used unless the operator unticks it, like qgc', () => {
    const choice = { ...initialFastCompassChoice(withGcs), enabled: true };
    expect(choice.useGcs).toBe(true);
    expect(fastCompassArguments(withGcs, choice)).toEqual([41.7, 44.8]);
    expect(fastCompassArguments(withGcs, { ...choice, useGcs: false, latitude: '1.5', longitude: '2.5' })).toEqual([1.5, 2.5]);
  });

  it('without a gcs position the typed coordinates go to the core, which rejects garbage', () => {
    const choice = { ...initialFastCompassChoice(withoutGcs), enabled: true };
    expect(choice.useGcs).toBe(false);
    expect(withoutGcs.gcsLatitude).toBeNull();
    expect(fastCompassArguments(withoutGcs, choice)).toEqual([0.0, 0.0]);
    expect(fastCompassArguments(withoutGcs, { ...choice, latitude: 'x' })).toEqual(['x', 0.0]);
  });

  it('the fly map centre stands in for the position when ticked, like APMSensorsComponent', () => {
    const choice = { ...initialFastCompassChoice(withoutGcs, TrackPoint(41.71, 44.79)), enabled: true };
    expect(fastCompassArguments(withoutGcs, choice)).toEqual([0.0, 0.0]);
    expect(fastCompassArguments(withoutGcs, { ...choice, useMap: true })).toEqual([41.71, 44.79]);
    expect(fastCompassArguments(withoutGcs, { ...choice, useMap: true, mapPosition: null })).toEqual([0.0, 0.0]);
  });

  it('px4 serves no fast compass', () => {
    expect(fastCompass(null)).toBeNull();
  });
});
