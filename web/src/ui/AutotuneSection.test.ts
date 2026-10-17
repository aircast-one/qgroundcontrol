import { describe, expect, it } from 'vitest';
import { autotuneState } from './AutotuneSection';
import { tuningModes } from './Px4TuningScreen';

describe('AutotuneSectionTest', () => {
  it('autotune state and the tuning flight modes read from their views', () => {
    expect(autotuneState(JSON.parse('{"available":true,"canStart":false,"status":"Take off first - auto-tuning runs in flight","progress":0,"warning":"WARNING!"}'))).toEqual({
      canStart: false,
      status: 'Take off first - auto-tuning runs in flight',
      progress: 0,
      warning: 'WARNING!',
    });
    expect(autotuneState(JSON.parse('{"available":false}'))).toBeNull();
    expect(tuningModes(JSON.parse('{"stabilizedFlightMode":"Stabilized","pauseFlightMode":"Hold"}'))).toEqual({ stabilized: 'Stabilized', pause: 'Hold' });
  });
});
