import { describe, expect, it } from 'vitest';
import type { GimbalIndicatorState } from './GimbalIndicator';
import { RecenterTarget, recenterTarget, tiltText } from './RecenterCamera';
import { DEFAULT_TILT_DEG } from './SyntheticView';

describe('RecenterCameraTest', () => {
  const gimbal = (pitch: number | null, yaw: number | null): GimbalIndicatorState => ({
    statusText: '',
    pitchText: '',
    yawText: '',
    yawLockLabel: '',
    yawLocked: false,
    yawLockOffered: false,
    retractOffered: false,
    controlOffered: false,
    controlLabel: '',
    haveControl: false,
    gimbals: [],
    pitchDegrees: pitch,
    yawDegrees: yaw,
  });

  it('recenter shows only once the camera points away from centre', () => {
    expect(recenterTarget(true, DEFAULT_TILT_DEG, 0.0, null)).toBeNull();
    expect(recenterTarget(true, DEFAULT_TILT_DEG, 40.0, null)).toBe(RecenterTarget.Synthetic);
    expect(recenterTarget(false, null, null, gimbal(0.5, -1.0))).toBeNull();
    expect(recenterTarget(false, null, null, gimbal(-45.0, 0.0))).toBe(RecenterTarget.Gimbal);
  });

  it('an aimable synthetic view is recentred rather than the gimbal behind it', () => {
    expect(recenterTarget(true, DEFAULT_TILT_DEG, 0.0, gimbal(-45.0, 0.0))).toBeNull();
  });

  it('the readout rounds to whole degrees', () => {
    expect(tiltText(-11.4)).toBe('-11°');
    expect(tiltText(0.2)).toBe('0°');
  });

  it('nothing to recenter without an aimable view or a gimbal', () => {
    expect(recenterTarget(false, -60.0, 90.0, null)).toBeNull();
  });
});
