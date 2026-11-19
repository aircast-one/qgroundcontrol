// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { GimbalSwitch, gimbalSwitchText, railShowsGimbalMode } from '@/ui/RcCameraControls';

const fpv = { name: 'SITL FPV camera', managerCompid: 1, deviceId: 1, active: false };
const payload = { name: 'SITL payload camera', managerCompid: 1, deviceId: 2, active: true };

describe('the camera rail says which gimbal it steers and switches to another', () => {
  afterEach(cleanup);

  it('names the active gimbal, or asks for one when none is active', () => {
    expect(gimbalSwitchText([fpv, payload])).toBe('SITL payload camera');
    expect(gimbalSwitchText([fpv, { ...payload, active: false }])).toBe('Choose a gimbal');
  });

  it('lists every gimbal with the active one checked, and picks the one clicked', async () => {
    const onPick = vi.fn();
    render(<GimbalSwitch choices={[fpv, payload]} onPick={onPick} />);
    fireEvent.click(screen.getByRole('button', { name: 'Gimbal: SITL payload camera. Switch gimbal' }));
    expect((await screen.findByRole('menuitem', { name: 'SITL payload camera' })).getAttribute('aria-current')).toBe('true');
    fireEvent.click(screen.getByRole('menuitem', { name: 'SITL FPV camera' }));
    expect(onPick).toHaveBeenCalledWith(fpv);
  });

  it('the rail carries the gimbal mode whenever a gimbal is there and the picture does not show the switch', () => {
    expect(railShowsGimbalMode(true, false)).toBe(true);
    expect(railShowsGimbalMode(true, true)).toBe(false);
    expect(railShowsGimbalMode(false, false)).toBe(false);
  });
});
