import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { motorLoads } from './MotorLoadWidget';

describe('MotorLoadWidgetTest', () => {
  it("draws one rotor per motor of the core's frame", () => {
    const loads = motorLoads(
      parseJson(
        '{"shown":true,"frame":[' +
          '{"motor":1,"angle":90,"reporting":false,"current":"0.0","temperature":"0 C","temperatureLevel":"normal","currentLevel":"normal","fill":0},' +
          '{"motor":5,"angle":30,"reporting":true,"current":"50.4","temperature":"75 C","temperatureLevel":"elevated","currentLevel":"critical","fill":0.83}]}',
      ),
    );
    expect(loads.map((load) => load.motor)).toEqual([1, 5]);
    expect(loads[1]).toMatchObject({ angle: 30, reporting: true, current: '50.4', temperatureLevel: 'elevated', currentLevel: 'critical', fill: 0.83 });
    expect(motorLoads(parseJson('{"shown":false}'))).toEqual([]);
    expect(motorLoads(parseJson('{"shown":true,"motors":[]}'))).toEqual([]);
  });
});
