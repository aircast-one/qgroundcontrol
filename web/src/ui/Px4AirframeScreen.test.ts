import { describe, expect, it } from 'vitest';
import { airframeImageAsset, initialSelection, px4Airframes } from './Px4AirframeScreen';

describe('Px4AirframeScreenTest', () => {
  it('the current airframe preselects its group and the apply text loses its html breaks', () => {
    const read = px4Airframes(
      JSON.parse(
        '{"available":true,"autostartId":4001,"custom":false,"heading":"You\'ve connected a Generic Quadcopter.","currentType":"Quadrotor x",' +
          '"currentIndex":0,"applyTitle":"Apply and Restart","applyText":"a<br><br>b","types":[{"name":"Quadrotor x","airframes":[{"name":"Generic Quadcopter","autostartId":4001}]}]}',
      ),
    );
    expect(read).not.toBeNull();
    if (read === null) return;
    expect(initialSelection(read)).toEqual({ group: 'Quadrotor x', index: 0 });
    expect(read.applyText).toBe('a\n\nb');
    expect(read.groups).toHaveLength(1);
    expect(read.groups[0]?.airframes).toHaveLength(1);
    expect(read.groups[0]?.airframes[0]?.autostartId).toBe(4001);
    expect(px4Airframes(JSON.parse('{"available":false}'))).toBeNull();
  });

  it('each airframe group shows its QGC picture', () => {
    expect(airframeImageAsset('QuadRotorX')).not.toBeNull();
    expect(airframeImageAsset('QuadRotorX')).not.toBe(airframeImageAsset('HexaRotorX'));
    expect(airframeImageAsset('QuadRotorX.svg')).toBe(airframeImageAsset('QuadRotorX'));
    expect(airframeImageAsset('')).toBeNull();
  });
});
