import { describe, expect, it } from 'vitest';
import { clipText } from './VibrationScreen';

describe('VibrationClipTextTest', () => {
  it('each clip count names its accelerometer, like VibrationPage', () => {
    expect(clipText([0, 3, 0])).toBe('Accel 1: 0 · Accel 2: 3 · Accel 3: 0');
  });
});
