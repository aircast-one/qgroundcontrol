import { describe, expect, it } from 'vitest';
import { vehicleAltitudePath } from './ItemEditor';
import { cameraRecording, rcCameraChannels } from './RcCameraControls';

describe('RcCameraControlsTest', () => {
  it('a pan channel shared with tilt is dropped, as CameraControlLayer reads it', () => {
    expect(rcCameraChannels(7, 7, 0, 0, 0).pan).toBe(0);
    expect(rcCameraChannels(7, 8, 0, 0, 0).pan).toBe(8);
    expect(rcCameraChannels(0, 0, 0, 0, 0).any).toBe(false);
    expect(rcCameraChannels(0, 0, 9, 0, 0).any).toBe(true);
  });

  it('record shows on when either the stream or the record channel records, as _recording does', () => {
    expect(cameraRecording(0, false, true)).toBe(true);
    expect(cameraRecording(5, true, false)).toBe(true);
    expect(cameraRecording(0, true, false)).toBe(false);
  });
});

describe('VehicleAltitudeTest', () => {
  it("the vehicle altitude copied is the one in the item's frame, as EditPositionDialog picks it", () => {
    expect(vehicleAltitudePath(1)).toBe('vehicle.altitudeRelative');
    expect(vehicleAltitudePath(2)).toBe('vehicle.altitudeAMSL');
    expect(vehicleAltitudePath(3)).toBe('vehicle.altitudeAboveTerr');
    expect(vehicleAltitudePath(4)).toBe('vehicle.altitudeAboveTerr');
    expect(vehicleAltitudePath(null)).toBeNull();
  });
});
