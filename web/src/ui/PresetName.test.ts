import { describe, expect, it } from 'vitest';
import { presetNameError } from './PatternPresets';

describe('PresetNameTest', () => {
  it('the name is checked as it is typed, like TransectStyleComplexItemEditor', () => {
    expect(presetNameError('  ')).toBe('Preset name cannot be blank.');
    expect(presetNameError('a/b')).toBe('Preset name cannot include the "/" character.');
    expect(presetNameError('Farm 2')).toBeNull();
  });
});
