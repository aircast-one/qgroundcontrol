import { describe, expect, it } from 'vitest';
import { inGroup, parameterKeys, parameterMatches, parameterPath, parameterSubtitle, parameterTree } from './ParametersScreen';

describe('ParametersScreenTest', () => {
  it('a parameter path is addressable through the bridge', () => {
    expect(parameterPath('ACRO_BAL_PITCH')).toBe('vehicle.parameterManager.getParameter(-1,ACRO_BAL_PITCH)');
  });

  it('the path a row writes to is the one the bridge can resolve', () => {
    const path = parameterPath('RTL_ALT');
    expect(path).toContain('.');
    expect(path.startsWith('vehicle.parameterManager.')).toBe(true);
    expect(path.endsWith('(-1,RTL_ALT)')).toBe(true);
  });

  it('a parameter is found by what it does, not only by its name', () => {
    expect(parameterMatches('BATT_FS_LOW_ACT', ['Low battery failsafe action'], 'failsafe')).toBe(true);
    expect(parameterMatches('BATT_FS_LOW_ACT', ['Low battery failsafe action'], 'BATT')).toBe(true);
    expect(parameterMatches('BATT_FS_LOW_ACT', ['Low battery failsafe action'], 'compass')).toBe(false);
  });

  it('every word must match and each word is a regular expression, as ParameterEditorController does', () => {
    expect(parameterMatches('BATT_VOLT_MULT', ['Voltage multiplier'], 'batt volt')).toBe(true);
    expect(parameterMatches('BATT_VOLT_MULT', ['Voltage multiplier'], 'batt compass')).toBe(false);
    expect(parameterMatches('RC1_MIN', ['RC min PWM'], '^RC')).toBe(true);
    expect(parameterMatches('SERVO_RC_ON', ['Servo'], '^RC')).toBe(false);
    expect(parameterMatches('ATC_RATE_P', ['Roll axis rate P gain'], 'RATE.*P')).toBe(true);
    expect(parameterMatches('A(B', [''], 'A(B')).toBe(true);
  });

  it('the long description is searched too, as ParameterEditorController does', () => {
    expect(parameterMatches('RTL_ALT', ['Return altitude', 'The minimum height to climb to before returning'], 'climb')).toBe(true);
    expect(parameterMatches('RTL_ALT', ['Return altitude'], 'climb')).toBe(false);
  });

  it('searching is case insensitive on both halves', () => {
    expect(parameterMatches('RTL_ALT', ['Return to launch altitude'], 'rtl_alt')).toBe(true);
    expect(parameterMatches('RTL_ALT', ['Return to launch altitude'], 'LAUNCH')).toBe(true);
  });

  it('a parameter whose description has not loaded yet still matches by name', () => {
    expect(parameterMatches('RTL_ALT', [''], 'RTL')).toBe(true);
    expect(parameterMatches('RTL_ALT', [''], 'launch')).toBe(false);
  });

  it('an empty search keeps every parameter', () => {
    expect(parameterMatches('ANY', [''], '')).toBe(true);
  });
});

describe('ParameterSubtitleTest', () => {
  it('a parameter says what it is and what it is measured in', () => {
    expect(parameterSubtitle('RTL Altitude', 'cm')).toBe('RTL Altitude · cm');
  });

  it('either half stands alone and neither leaves a stray separator', () => {
    expect(parameterSubtitle('RTL Altitude', '')).toBe('RTL Altitude');
    expect(parameterSubtitle('', 'cm')).toBe('cm');
    expect(parameterSubtitle('', '')).toBe('');
  });

  it('parameters fall into categories and groups the way the Qt editor builds its tree', () => {
    const placement = new Map([
      ['A_ONE', ['Advanced', 'Misc'] as const],
      ['A_TWO', ['Advanced', 'Attitude'] as const],
      ['B_ONE', ['Other', 'Misc'] as const],
      ['C_ONE', ['Standard', 'Battery'] as const],
    ]);
    const tree = parameterTree(['A_ONE', 'A_TWO', 'B_ONE', 'C_ONE'], placement);
    expect(tree.map((it) => it.name)).toEqual(['Standard', 'Advanced', 'Other']);
    expect(tree[1]?.groups).toEqual(['Attitude', 'Misc']);
    expect(inGroup('A_TWO', placement, 'Advanced', 'Attitude')).toBe(true);
    expect(inGroup('A_ONE', placement, 'Advanced', 'Attitude')).toBe(false);
  });

  it('every component is listed and a non-autopilot parameter carries its component', () => {
    expect(
      parameterKeys(
        new Map([
          [1, ['B', 'A']],
          [154, ['MNT_TYPE']],
        ]),
      ),
    ).toEqual(['A', 'B', '154:MNT_TYPE']);
    expect(parameterPath('154:MNT_TYPE')).toBe('vehicle.parameterManager.getParameter(154,MNT_TYPE)');
    expect(parameterPath('RTL_ALT')).toBe('vehicle.parameterManager.getParameter(-1,RTL_ALT)');
    expect(parameterMatches('154:MNT_TYPE', [], '^MNT')).toBe(true);
  });
});
