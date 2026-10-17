import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { cameraCalc, distanceLabel, distanceModeTitle, shownCameraFacts } from './CameraCalcSection';

const control = (suffix: string): Json => ({ class: 'Control', path: `plan.missionController.visualItems.1.${suffix}`, name: suffix, pathSuffix: suffix, value: 1, valueString: '1' });

const view = (brand: string, custom: boolean, byDistance: boolean): Json => ({
  camera: {
    brand,
    manualName: 'Manual (no camera specs)',
    custom,
    valueSetIsDistance: byDistance,
    facts: [
      'cameraCalc.sensorWidth',
      'cameraCalc.distanceToSurface',
      'cameraCalc.imageDensity',
      'cameraCalc.frontalOverlap',
      'cameraCalc.sideOverlap',
      'cameraCalc.adjustedFootprintFrontal',
      'cameraCalc.adjustedFootprintSide',
    ].map(control),
  },
});

const block = (json: Json) => {
  const parsed = cameraCalc(json);
  if (parsed === null) throw new Error('no camera block');
  return parsed;
};

const shown = (brand: string, custom: boolean, byDistance: boolean) => shownCameraFacts(block(view(brand, custom, byDistance))).map((it) => it.path.split('visualItems.1.')[1]);

describe('CameraCalcSectionTest', () => {
  it('the grid shows the figure that sets the other, as CameraCalcGrid does', () => {
    const spacing = ['cameraCalc.adjustedFootprintFrontal', 'cameraCalc.adjustedFootprintSide'];
    expect(shown('Sony', false, true)).toEqual(['cameraCalc.sensorWidth', 'cameraCalc.distanceToSurface', 'cameraCalc.frontalOverlap', 'cameraCalc.sideOverlap', ...spacing]);
    expect(shown('Sony', false, false)).toEqual(['cameraCalc.sensorWidth', 'cameraCalc.imageDensity', 'cameraCalc.frontalOverlap', 'cameraCalc.sideOverlap', ...spacing]);
    expect(shown('Custom Camera', true, true)).toEqual(['cameraCalc.sensorWidth', 'cameraCalc.distanceToSurface', 'cameraCalc.frontalOverlap', 'cameraCalc.sideOverlap', ...spacing]);
    expect(shown('Manual (no camera specs)', false, false), 'a manual camera sets its trigger distance and spacing by hand').toEqual([
      'cameraCalc.distanceToSurface',
      'cameraCalc.adjustedFootprintFrontal',
      'cameraCalc.adjustedFootprintSide',
    ]);
  });

  it('Set by names the distance the way the editor does', () => {
    const scan = { ...control('cameraCalc.distanceToSurface'), shortLabel: 'Scan distance' };
    const parsed = block({ camera: { brand: 'Sony', facts: [scan] } });
    expect(distanceLabel(parsed)).toBe('Scan distance');
    expect(distanceLabel({ ...parsed, facts: [] })).toBe('Altitude');
  });

  it('the transect altitude frame reads from the core and hides where it does not apply', () => {
    const parsed = block({
      camera: {
        brand: 'Manual (no camera specs)',
        manualName: 'Manual (no camera specs)',
        distanceMode: 3,
        distanceModePath: 'p.cameraCalc.distanceMode',
        distanceModes: [
          { raw: 1, title: 'Relative (Rel)' },
          { raw: 3, title: 'Above Terrain Calced (AGLC)' },
        ],
        facts: [],
      },
    });
    expect(distanceModeTitle(parsed)).toBe('Above Terrain Calced (AGLC)');
    expect(parsed.distanceModePath).toBe('p.cameraCalc.distanceMode');
    expect(distanceModeTitle({ ...parsed, distanceMode: 5 })).toBeNull();
  });
});
