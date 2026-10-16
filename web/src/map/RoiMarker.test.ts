import { describe, expect, it } from 'vitest';
import { roiPoint } from './RoiMarker';

describe('RoiMarkerTest', () => {
  it('the marker shows only while the vehicle reports an active roi', () => {
    expect(roiPoint({ roiActive: true, roi: { latitude: 47.4, longitude: 8.5 } })).toEqual({ latitude: 47.4, longitude: 8.5 });
    expect(roiPoint({ roiActive: false, roi: null })).toBeNull();
    expect(roiPoint({ roi: { latitude: 0.0, longitude: 0.0 } })).toBeNull();
    expect(roiPoint(null)).toBeNull();
  });
});
