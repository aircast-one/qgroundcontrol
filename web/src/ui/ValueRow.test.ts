import { describe, expect, it } from 'vitest';
import { makeFact } from '@/bridge/Qgc';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { rowChoiceLabel } from '@/ui/SettingsScreen';
import {
  VALUE_NO_LIMIT, VALUE_OFF, dangerousChoice, derivedSlider, grabsThumb, inlineSlider, nudged, opensAsValue, roundedValue, safeFirst, sliderFor,
  sliderPosition, steppedValue, thumbFraction, turnOnValue, valueAtPosition, valueStep, valueText, valueTextOf,
} from './ValueRow';

const number = (
  value: string,
  { longDescription = '', min = '', max = '', bounded = false, isString = false, name = 'MPC_XY_VEL_MAX', fallback = '', units = 'm' }: { longDescription?: string; min?: string; max?: string; bounded?: boolean; isString?: boolean; name?: string; fallback?: string; units?: string } = {},
) =>
  makeFact({
    path: 'vehicle.parameterManager.getParameter(-1,GF_MAX_VER_DIST)',
    name,
    description: 'Max vertical distance from Home',
    units,
    valueString: value,
    value: toDoubleOrNull(value),
    enumStrings: [],
    enumIndex: -1,
    isBool: false,
    isString,
    readOnly: false,
    minString: min,
    maxString: max,
    minIsDefaultForType: !bounded,
    maxIsDefaultForType: !bounded,
    longDescription,
    defaultValueString: fallback,
  });

const disabledAtZero = { longDescription: 'Disabled if 0.' };

describe('ValueRowTest', () => {
  it('a limit that zero disables reads Off at zero and its value otherwise', () => {
    expect(valueText(number('0', disabledAtZero))).toBe(VALUE_OFF);
    expect(valueText(number('120', disabledAtZero))).toBe('120 m');
    expect(valueText(number('0', { longDescription: 'Height above home.' }))).toBe('0 m');
  });

  it("steps follow the value's own precision and stay inside its range", () => {
    expect(valueStep(number('12.0'))).toBeCloseTo(0.1, 9);
    expect(valueStep(number('30'))).toBeCloseTo(1.0, 9);
    expect(steppedValue(number('20', { min: '0', max: '20', bounded: true }), 1)).toBeCloseTo(20.0, 9);
    expect(steppedValue(number('30'), -1)).toBeCloseTo(29.0, 9);
    expect(steppedValue(number('0.7'), 1)).toBeCloseTo(0.8, 9);
    expect(roundedValue(number('30'), 3456.237)).toBeCloseTo(3456.0, 9);
  });

  it('a slider only appears for a bounded, sensible range', () => {
    expect(derivedSlider(number('0', { min: '0', max: '10000', bounded: true }))?.to).toBe(10000);
    expect(derivedSlider(number('30'))).toBeNull();
    expect(derivedSlider(number('30', { min: '0', max: '340282346638528859811704183484516925440', bounded: true }))).toBeNull();
  });

  it('numbers open as a value row and text stays a field', () => {
    expect(opensAsValue(number('30'))).toBe(true);
    expect(opensAsValue(number('127.0.0.1', { isString: true }))).toBe(false);
  });

  it('turning a limit on lands on a safe value, never one step above zero', () => {
    expect(turnOnValue(number('0', { ...disabledAtZero, fallback: '0', name: 'GF_MAX_VER_DIST' }))).toBeCloseTo(120.0, 9);
    expect(turnOnValue(number('0', { ...disabledAtZero, name: 'GF_MAX_HOR_DIST' }))).toBeCloseTo(500.0, 9);
    expect(turnOnValue(number('0', { ...disabledAtZero, name: 'FENCE_ALT_MAX', units: 'cm' }))).toBeCloseTo(12000.0, 9);
    expect(turnOnValue(number('0', { ...disabledAtZero, name: 'OTHER', fallback: '80' }))).toBeCloseTo(80.0, 9);
    expect(nudged(number('0', { ...disabledAtZero, name: 'GF_MAX_VER_DIST' }), 0.0, 1, 0)).toBeCloseTo(120.0, 9);
  });

  it('altitudes and distances step in whole metres whatever their precision', () => {
    expect(valueStep(number('30.0', { name: 'RTL_RETURN_ALT' }))).toBeCloseTo(1.0, 9);
    expect(valueStep(number('3000', { name: 'RTL_ALT', units: 'cm' }))).toBeCloseTo(100.0, 9);
  });

  it('holding a step button speeds up after a while', () => {
    expect(nudged(number('30'), 30.0, 1, 3)).toBeCloseTo(31.0, 9);
    expect(nudged(number('30'), 30.0, 1, 12)).toBeCloseTo(40.0, 9);
  });

  it('return and fence sliders cover the useful band, not the whole type range', () => {
    const slider = sliderFor(number('30', { name: 'RTL_RETURN_ALT' }));
    expect(slider?.from).toBe(20);
    expect(slider?.to).toBe(500);
    expect(sliderFor(number('0', { name: 'GF_MAX_HOR_DIST', min: '0', max: '10000', bounded: true }))?.from).toBe(50);
  });

  it('a pending value reads like the row', () => {
    expect(valueTextOf(number('120', disabledAtZero), 0.0)).toBe(VALUE_OFF);
    expect(valueTextOf(number('30.0'), 45.5)).toBe('45.5 m');
  });

  it('a long choice label is cut at its first clause in the row', () => {
    expect(rowChoiceLabel('Return at critical level, land at emergency level')).toBe('Return at critical level');
    expect(rowChoiceLabel('Hold mode')).toBe('Hold mode');
  });

  it("a limit's slider ends on No limit, like DJI's Max Distance", () => {
    const limit = number('0', { ...disabledAtZero, name: 'GF_MAX_HOR_DIST' });
    const slider = inlineSlider(limit);
    expect(slider).not.toBeNull();
    if (slider === null) return;
    expect(valueTextOf(limit, 0.0)).toBe(VALUE_NO_LIMIT);
    expect(sliderPosition(slider, 0.0)).toBe(slider.end);
    expect(valueAtPosition(limit, slider, slider.end)).toBeCloseTo(0.0, 9);
    expect(valueAtPosition(limit, slider, slider.top)).toBeCloseTo(5000.0, 9);
    expect(valueAtPosition(limit, slider, 1234.4)).toBeCloseTo(1234.0, 9);
  });

  it('a plain range has no No limit stop', () => {
    const speed = number('12.0', { min: '0', max: '20', bounded: true });
    const slider = inlineSlider(speed);
    expect(slider?.noLimitAt).toBeNull();
    if (slider === null) return;
    expect(valueAtPosition(speed, slider, slider.end)).toBeCloseTo(20.0, 9);
  });

  it('only a touch on the knob grabs it, so scrolling past cannot move a limit', () => {
    expect(grabsThumb(105, 100, 24)).toBe(true);
    expect(grabsThumb(300, 100, 24)).toBe(false);
    expect(thumbFraction(50, { start: 0, endInclusive: 100 })).toBe(0.5);
    expect(thumbFraction(500, { start: 0, endInclusive: 100 })).toBe(1);
  });

  it('motor-stopping failsafes sink below the safe ones', () => {
    const options = ['Do nothing', 'Hover', 'Stop motors', 'Return home', 'Disarm', 'Land'];
    expect(safeFirst(options)).toEqual([0, 1, 3, 5, 2, 4]);
    expect(dangerousChoice('Terminate')).toBe(true);
    expect(dangerousChoice('Return home')).toBe(false);
  });
});
