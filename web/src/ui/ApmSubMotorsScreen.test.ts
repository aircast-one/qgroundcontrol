import { describe, expect, it } from 'vitest';
import type { SubFrames } from './ApmSubFrameScreen';
import { motorDisplayFrame, subMotors } from './ApmSubMotorsScreen';

describe('ApmSubMotorsScreenTest', () => {
  it('motors, their directions and the test gate come from the core', () => {
    const read = subMotors(
      JSON.parse(`{"available":true,"armed":true,"detecting":false,"canRunManualTest":true,
        "motors":[{"motor":1,"reversed":false},{"motor":2,"reversed":true}],"warning":"w","offersAutoDetect":true,"autoDetectHelp":"h","detectionMessages":"Thruster 1 ok\\n"}`),
    );
    expect(read?.motors).toEqual([
      { motor: 1, reversed: false },
      { motor: 2, reversed: true },
    ]);
    expect(read?.canRunManualTest).toBe(true);
    expect(read?.offersAutoDetect).toBe(true);
    expect(read?.detectionMessages).toBe('Thruster 1 ok\n');
    expect(subMotors(JSON.parse('{"available":false}'))).toBeNull();
  });

  it('the frame picture shows only for frames APMSubMotorDisplay draws', () => {
    const frames = (selected: number | null): SubFrames => ({ frames: [], selected, confirmFirst: false, loading: false, loadError: '' });
    expect(motorDisplayFrame(frames(1))).toBe(1);
    expect(motorDisplayFrame(frames(5))).toBe(5);
    expect(motorDisplayFrame(frames(6))).toBeNull();
    expect(motorDisplayFrame(frames(3))).toBeNull();
    expect(motorDisplayFrame(frames(null))).toBeNull();
    expect(motorDisplayFrame(null)).toBeNull();
  });
});
