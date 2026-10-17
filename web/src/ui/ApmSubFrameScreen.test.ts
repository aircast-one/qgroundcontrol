import { describe, expect, it } from 'vitest';
import { subFrames } from './ApmSubFrameScreen';

describe('ApmSubFrameScreenTest', () => {
  it('frames, the selection and the confirm rule come from the core', () => {
    const read = subFrames(
      JSON.parse(`{"available":true,"selected":1,"confirmFirst":true,"loadingDefaults":false,"loadError":"",
        "frames":[{"name":"BlueROV1","value":0,"hasDefaults":false},{"name":"BlueROV2/Vectored","value":1,"hasDefaults":true}]}`),
    );
    expect(read?.frames.map((it) => it.name)).toEqual(['BlueROV1', 'BlueROV2/Vectored']);
    expect(read?.selected).toBe(1);
    expect(read?.confirmFirst).toBe(true);
    expect(read?.frames[1]?.hasDefaults).toBe(true);
    expect(subFrames(JSON.parse('{"available":false}'))).toBeNull();
  });
});
