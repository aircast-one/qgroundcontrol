import { describe, expect, it } from 'vitest';
import { radioView } from '@/ui/RadioView';

describe('CenteredThrottleTest', () => {
  it('the radio view carries centered throttle and joystick mode', () => {
    const read = radioView({ class: 'Radio', connected: true, centeredThrottle: true, joystickMode: false });
    expect(read?.centeredThrottle).toBe(true);
    expect(read?.joystickMode).toBe(false);
  });
});
