import { describe, expect, it } from 'vitest';
import { gimbalAzimuths } from './GimbalWedge';

describe('GimbalWedgeTest', () => {
  it("each gimbal's yaw is read for the map wedge", () => {
    const read = gimbalAzimuths({ gimbals: [{ yaw: 270.0, active: true }, { yaw: 10.0, active: false }] });
    expect(read).toEqual([{ yaw: 270.0, active: true }, { yaw: 10.0, active: false }]);
    expect(gimbalAzimuths(null)).toEqual([]);
  });
});
