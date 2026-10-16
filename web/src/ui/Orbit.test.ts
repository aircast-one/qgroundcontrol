import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { orbitLabel, orbitReading } from './Orbit';

const json = (text: string): Json => parseJson(text) ?? {};

const view = ({ orbiting = 'true', radiusText = '"120 m"', clockwise = 'true', reason = '""' }: { orbiting?: string; radiusText?: string; clockwise?: string; reason?: string } = {}): Json =>
  json(`{"kind":"object","class":"Orbit","available":true,"orbiting":${orbiting},
         "radiusText":${radiusText},"clockwise":${clockwise},"reason":${reason}}`);

describe('OrbitTest', () => {
  it('another view is not an orbit reading', () => {
    expect(orbitReading(null)).toBeNull();
    expect(orbitReading(json('{"kind":"object","class":"Orbit2"}'))).toBeNull();
  });

  it('an orbit states its radius and which way round it goes', () => {
    expect(orbitLabel(orbitReading(view()))).toBe('Orbiting 120 m clockwise');
    expect(orbitLabel(orbitReading(view({ clockwise: 'false' })))).toBe('Orbiting 120 m anticlockwise');
  });

  it('a vehicle not orbiting shows nothing, because the chip is the state', () => {
    expect(orbitLabel(orbitReading(view({ orbiting: 'false', radiusText: 'null', clockwise: 'null' })))).toBeNull();
  });

  it('no contact is not a stopped orbit, and neither draws the chip', () => {
    const quiet = orbitReading(
      view({
        orbiting: 'null',
        radiusText: 'null',
        clockwise: 'null',
        reason: '"No contact, so whether the vehicle is still orbiting is unknown."',
      }),
    )!;

    expect(quiet.orbiting).toBeNull();
    expect(orbitLabel(quiet)).toBeNull();
    expect(quiet.reason).toBe('No contact, so whether the vehicle is still orbiting is unknown.');
  });

  it('an orbit whose radius the core will not state is still an orbit', () => {
    expect(orbitLabel(orbitReading(view({ radiusText: 'null' })))).toBe('Orbiting clockwise');
  });
});
