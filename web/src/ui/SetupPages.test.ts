import { describe, expect, it } from 'vitest';
import { FLIGHT_MODES_PAGE, MOTORS, RADIO, REMOTE_SUPPORT, SENSORS, headPage } from '@/ui/SetupPages';
import { SetupComponent } from '@/ui/SetupScreen';

const component = (name: string, known: string | null = null) => SetupComponent({ index: 0, name, known, needsAttention: false });

describe('SetupPagesTest', () => {
  it('a component with a stable identity is routed by it, not by its display name', () => {
    expect(headPage(component('Sensoren', 'sensors'))).toBe(SENSORS);
    expect(headPage(component('Funk', 'radio'))).toBe(RADIO);
    expect(headPage(component('Flugmodi', 'flightModes'))).toBe(FLIGHT_MODES_PAGE);
  });

  it('a component QGC has no identity for falls back to its name, which is all there is', () => {
    expect(headPage(component(MOTORS))).toBe(MOTORS);
    expect(headPage(component(REMOTE_SUPPORT))).toBe(REMOTE_SUPPORT);
  });

  it('an identity the head has no page for keeps the name rather than resolving to nothing', () => {
    expect(headPage(component('Power', 'power'))).toBe('Power');
    expect(headPage(component('Safety', 'safety'))).toBe('Safety');
  });
});
