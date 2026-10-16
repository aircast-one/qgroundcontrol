import { describe, expect, it } from 'vitest';
import { deviceCameraRotation } from '@/ui/DeviceCameraHost';

describe('DeviceCameraHostTest', () => {
  it('a device camera is turned upright for every way the screen can face', () => {
    expect(deviceCameraRotation(90, false, 0)).toBe(90);
    expect(deviceCameraRotation(90, false, 90)).toBe(0);
    expect(deviceCameraRotation(90, false, 270)).toBe(180);
    expect(deviceCameraRotation(270, true, 0)).toBe(270);
    expect(deviceCameraRotation(270, true, 90)).toBe(0);
    expect(deviceCameraRotation(270, true, 270)).toBe(180);
  });
});
