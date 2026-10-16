import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { obstacleWarning } from './ObstacleDistance';

const json = (text: string): Json => parseJson(text) ?? {};

describe('ObstacleDistanceTest', () => {
  it('a reading whose age the sensor never reported is still drawn, deliberately', () => {
    const ageless = json('{"available":true,"stale":null,"nearest":{"distanceText":"4 m","sectorText":"ahead","close":true}}');
    expect(obstacleWarning(ageless)?.label, 'null means the sensor never said how old the reading is').toBe('4 m ahead');
  });

  it('a reading the sensor called stale is withheld', () => {
    const old = json('{"available":true,"stale":true,"nearest":{"distanceText":"4 m","sectorText":"ahead","stale":true}}');
    expect(obstacleWarning(old), 'the core serves staleness on nearest as well as beside it').toBeNull();
  });

  const view = ({ available = true, stale = false, nearest = `{"distanceText":"3.2 m","sectorText":"right","close":false,"stale":${stale}}` as string | null }: { available?: boolean; stale?: boolean; nearest?: string | null } = {}) =>
    json(`{"available":${available},"stale":${stale},"nearest":${nearest ?? 'null'}}`);

  it("a reading is the core's distance and the core's sector", () => {
    const warning = obstacleWarning(view())!;
    expect(warning.label).toBe('3.2 m right');
    expect(warning.close).toBe(false);
  });

  it("the distance is whatever the core spelled, so it follows the operator's units", () => {
    const feet = view({ nearest: '{"distanceText":"10.5 ft","sectorText":"ahead","close":true}' });
    expect(obstacleWarning(feet)!.label).toBe('10.5 ft ahead');
  });

  it('a close reading says so, for the head to colour', () => {
    const near = view({ nearest: '{"distanceText":"0.2 m","sectorText":"ahead","close":true}' });
    expect(obstacleWarning(near)!.close).toBe(true);
  });

  it('nothing is shown when the sensor is absent, stale, or reports no obstacle', () => {
    expect(obstacleWarning(null)).toBeNull();
    expect(obstacleWarning(view({ available: false }))).toBeNull();
    expect(obstacleWarning(view({ stale: true }))).toBeNull();
    expect(obstacleWarning(view({ nearest: null }))).toBeNull();
  });

  it('a reading with no distance text is not a warning', () => {
    expect(obstacleWarning(view({ nearest: '{"sectorText":"right"}' }))).toBeNull();
  });
});
