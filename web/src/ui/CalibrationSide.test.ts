import { describe, expect, it } from 'vitest';
import type { CalibrationSide } from '@/ui/CalibrationView';
import { sideImage, sideStateText } from '@/ui/SensorsScreen';

const side = (stage: string, rotate = false): CalibrationSide => ({ key: 'NoseDown', title: 'Nose down', visible: true, stage, rotate });

describe('CalibrationSideTest', () => {
  it("each side shows VehicleRotationCal's picture and state", () => {
    expect(sideImage('NoseDown', false)).toBe('cal_vehicle_nose_down');
    expect(sideImage('NoseDown', true)).toBe('cal_vehicle_nose_down_rotate');
    expect(sideImage('Down', false)).toBe('cal_vehicle_down');
    expect(sideStateText(side('inProgress', true))).toBe('Rotate');
    expect(sideStateText(side('inProgress'))).toBe('Hold still');
    expect(sideStateText(side('done'))).toBe('Done');
    expect(sideStateText(side('idle'))).toBe('Pending');
  });
});
