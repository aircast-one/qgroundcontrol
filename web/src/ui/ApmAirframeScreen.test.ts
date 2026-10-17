import { describe, expect, it } from 'vitest';
import { apmAirframe } from './ApmAirframeScreen';

describe('ApmAirframeScreenTest', () => {
  it('classes, their filtered types and the invalid flag come from the core', () => {
    const read = apmAirframe(
      JSON.parse(`{"available":true,"help":"Currently set","frameClass":2,"frameType":3,"invalidText":"Invalid setting for FRAME_TYPE. Click to Reset.",
        "classes":[{"name":"Quad","value":1,"chosen":false,"image":"QuadRotorX.svg","types":[{"name":"X","value":1}],"valid":true},
                   {"name":"Hexa","value":2,"chosen":true,"image":"AirframeUnknown.svg","types":[{"name":"X","value":1},{"name":"Plus","value":0}],"valid":false}]}`),
    );
    expect(read).not.toBeNull();
    expect(read?.classes.map((it) => it.name)).toEqual(['Quad', 'Hexa']);
    expect(read?.frameType).toBe(3);
    expect(read?.classes[1]?.valid).toBe(false);
    expect(read?.classes[1]?.types.map((it) => it.value)).toEqual([1, 0]);
    expect(apmAirframe(JSON.parse('{"available":false}'))).toBeNull();
  });
});
