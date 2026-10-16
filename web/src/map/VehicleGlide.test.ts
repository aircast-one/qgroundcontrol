import { describe, expect, it } from 'vitest';
import { VehicleChoice } from '@/map/VehiclePicker';
import { Glide, UNPLACED, VehiclePose, glidedAt, toward } from './VehicleGlide';

const ms = 1_000_000;
const first = VehiclePose({ latitude: 41.0, longitude: 44.0, heading: 350.0 });
const second = VehiclePose({ latitude: 41.001, longitude: 44.0, heading: 10.0 });
const flying = Glide.settled(UNPLACED, 0).toward(first, 0).toward(second, 400 * ms);

describe('VehicleGlideTest', () => {
  it('the marker moves at a steady speed from one fix to the next instead of jumping', () => {
    expect(Glide.settled(UNPLACED, 0).toward(first, 0).at(0)).toEqual(first);
    expect(flying.at(600 * ms).latitude).toBeCloseTo(41.0005, 9);
    expect(flying.at(800 * ms).latitude).toBeCloseTo(41.001, 9);
    expect(flying.at(5_000 * ms).latitude).toBeCloseTo(41.001, 9);
  });

  it('the arrow turns the short way across north', () => {
    expect(flying.at(600 * ms).heading).toBeCloseTo(360.0, 9);
  });

  it('a heading update between fixes keeps the position on schedule', () => {
    const turned = flying.toward({ ...second, heading: 20.0 }, 500 * ms);

    expect(flying.toward(second, 500 * ms), 'an unchanged pose does not keep the map redrawing').toBe(flying);
    expect(turned.endNs).toBe(flying.endNs);
    expect(turned.at(600 * ms).latitude).toBeCloseTo(flying.at(600 * ms).latitude, 9);
  });

  it('a fix after a long silence glides for at most a second', () => {
    const resumed = flying.toward(VehiclePose({ latitude: 41.002, longitude: 44.0, heading: 10.0 }), 60_000 * ms);

    expect(resumed.endNs - resumed.startNs).toBe(1_000 * ms);
  });

  it('every vehicle in a fleet glides, not only the active one', () => {
    const lead = VehicleChoice({ id: 1, name: 'Lead', state: '', link: '', contactLost: false, active: true, latitude: 41.0, longitude: 44.0, heading: 0.0 });
    const wing = { ...lead, id: 2, name: 'Wing', active: false, longitude: 45.0 };
    const moved = [{ ...lead, latitude: 41.001 }, { ...wing, latitude: 41.001 }];
    const gliding = toward(toward(new Map(), [lead, wing], 0), moved, 400 * ms);

    expect(glidedAt(moved, gliding, 600 * ms).map((it) => it.latitude)).toEqual([41.0005, 41.0005].map((it) => expect.closeTo(it, 9)));
    expect([...toward(gliding, moved.slice(0, 1), 500 * ms).keys()]).toEqual([1]);
  });

  it('losing the position hides the marker at once', () => {
    expect(flying.toward(UNPLACED, 500 * ms).at(500 * ms).latitude).toBeNaN();
  });
});
