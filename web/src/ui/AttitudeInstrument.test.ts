import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { NO_VEHICLE_ATTITUDE, attitude, pitchOffset } from './AttitudeInstrument';

const json = (text: string): Json => parseJson(text) ?? {};

describe('AttitudeInstrumentTest', () => {
  it('reads the angles and hides pointers the core withholds', () => {
    const reading = attitude(
      json('{"available":true,"roll":-12.5,"pitch":4,"heading":7.4,"headingText":"7°","courseOverGround":null,"headingToHome":190,"headingToNextWaypoint":null,"noseUp":false}'),
    )!;
    expect(reading.roll).toBe(-12.5);
    expect(reading.headingText).toBe('7°');
    expect(reading.courseOverGround).toBeNull();
    expect(reading.headingToHome).toBe(190);
    expect(reading.noseUp).toBe(false);
  });

  it('no vehicle reads as nothing, and the panel draws the bare dial at north', () => {
    expect(attitude(json('{"available":false}'))).toBeNull();
    expect(attitude(null)).toBeNull();
    expect(NO_VEHICLE_ATTITUDE.heading).toBe(0);
    expect(NO_VEHICLE_ATTITUDE.headingText).toBe('');
  });

  it('the dial reads home and the pilot whatever the compass setting', () => {
    const read = attitude(json('{"available":true,"heading":10,"homeBearing":190,"pilotBearing":45.5,"headingToHome":null}'))!;
    expect(read.homeBearing).toBe(190);
    expect(read.pilotBearing).toBe(45.5);
    expect(attitude(json('{"available":true,"heading":10,"homeBearing":null,"pilotBearing":null}'))!.pilotBearing).toBeNull();
  });

  it('pitch moves the horizon by the span QGC uses', () => {
    expect(pitchOffset(45, 20)).toBe(40);
    expect(pitchOffset(0, 20)).toBe(0);
  });
});
