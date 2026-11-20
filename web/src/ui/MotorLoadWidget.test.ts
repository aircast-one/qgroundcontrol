import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { besideWidget, motorLoads, rotorTitle } from './MotorLoadWidget';

describe('MotorLoadWidgetTest', () => {
  it("draws one rotor per motor of the core's frame", () => {
    const loads = motorLoads(
      parseJson(
        '{"shown":true,"frame":[' +
          '{"motor":1,"angle":90,"state":"none","current":"0.0","temperature":"0 C","temperatureLevel":"normal","currentLevel":"normal"},' +
          '{"motor":3,"angle":-30,"state":"lost","current":"0.0","temperature":"0 C","temperatureLevel":"normal","currentLevel":"normal"},' +
          '{"motor":5,"angle":30,"state":"reporting","current":"50.4","rpm":"4200","temperature":"75 C","temperatureLevel":"elevated","currentLevel":"critical"}]}',
      ),
    );
    expect(loads.map((load) => [load.motor, load.state])).toEqual([
      [1, 'none'],
      [3, 'lost'],
      [5, 'reporting'],
    ]);
    expect(loads[2]).toMatchObject({ angle: 30, current: '50.4', rpm: '4200', temperatureLevel: 'elevated', currentLevel: 'critical' });
    expect(motorLoads(parseJson('{"shown":false}'))).toEqual([]);
    expect(motorLoads(parseJson('{"shown":true,"motors":[]}'))).toEqual([]);
  });

  it('names each rotor by the letter the Motors page tests it by, and by its number when the frame is not known', () => {
    const [lettered, numbered] = motorLoads(
      parseJson(
        '{"shown":true,"frame":[' +
          '{"motor":5,"name":"A","angle":30,"state":"reporting","current":"5.0","rpm":"5000","temperature":"30 °C","temperatureLevel":"normal","currentLevel":"normal"},' +
          '{"motor":2,"angle":-90,"state":"lost","current":"0.0","temperature":"0 °C","temperatureLevel":"normal","currentLevel":"normal"}]}',
      ),
    );
    expect([rotorTitle(lettered!), rotorTitle(numbered!)]).toEqual(['Motor A: 5.0 A, 5000 rpm, 30 °C', 'Motor 2: ESC telemetry lost']);
  });

  it('opens its panel beside the frame, on whichever side has room', () => {
    expect(besideWidget({ left: 16, right: 156, top: 80 }, 1280)).toEqual({ x: 164, y: 80 });
    expect(besideWidget({ left: 1100, right: 1240, top: 80 }, 1280), 'a frame moved to the right edge opens the panel to its left').toEqual({ x: 772, y: 80 });
  });
});
