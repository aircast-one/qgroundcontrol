import { describe, expect, it } from 'vitest';
import { beamFeatures, cameraBeam } from './CameraBeam';

describe('CameraBeamTest', () => {
  const view = JSON.parse(
    '{"available":true,"beam":[{"latitude":-35.36,"longitude":149.16},{"latitude":-35.3597,"longitude":149.1604},{"latitude":-35.358,"longitude":149.162},{"latitude":-35.3585,"longitude":149.1628},{"latitude":-35.36,"longitude":149.1606},{"latitude":-35.36,"longitude":149.16}]}',
  );

  it('the beam is the polygon the core places in front of the drone', () => {
    const beam = cameraBeam(view);
    expect(beam.length).toBe(6);
    expect(beam[0]).toEqual({ latitude: -35.36, longitude: 149.16 });
    expect(beamFeatures(beam).features.length).toBe(1);
  });

  it('nothing is drawn while the core cannot place the camera', () => {
    expect(cameraBeam({ available: false, beam: [] })).toEqual([]);
    expect(cameraBeam(null)).toEqual([]);
    expect(beamFeatures([]).features.length).toBe(0);
  });
});
