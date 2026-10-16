import { describe, expect, it } from 'vitest';
import { same } from '@/kotlin/Std';
import { DEFAULT_INSTRUMENTS, defaultInstruments } from './InstrumentChoice';
import { NO_COLOUR, RangeType, ValueDisplay, displayColour, displayFrom, displayIcon, displayJson, displayOpacity, withRangeType, withRow, withoutRow } from './InstrumentDisplay';

describe('InstrumentDisplayTest', () => {
  const ranged = ValueDisplay({ rangeType: RangeType.Color, values: [10.0, 20.0], colours: [1, 2, 3] });

  it('a value takes the colour of the range it falls in', () => {
    expect(displayColour(ranged, 5.0)).toBe(1);
    expect(displayColour(ranged, 20.0)).toBe(2);
    expect(displayColour(ranged, 25.0)).toBe(3);
    expect(displayColour(ranged, null), 'an unknown value is the first range, as QGC treats NaN').toBe(1);
    expect(displayColour(ValueDisplay(), 5.0)).toBeNull();
    expect(displayColour({ ...ranged, colours: [NO_COLOUR, 2, 3] }, -1.0), 'an unchecked colour slot falls back to the palette colour').toBeNull();
  });

  it('forward flight vehicles keep airspeed among their default values like QGCCorePlugin', () => {
    expect(defaultInstruments('fixedWing')).toEqual([...DEFAULT_INSTRUMENTS, 'airSpeed']);
    expect(defaultInstruments('multiRotor')).toEqual(DEFAULT_INSTRUMENTS);
  });

  it('removing a threshold drops the band above it, as InstrumentValueData removeRangeValue does', () => {
    const coloured = ValueDisplay({ rangeType: RangeType.Color, values: [0.0, 50.0], colours: [1, 2, 3] });
    expect(withoutRow(coloured, 0).colours).toEqual([1, 3]);
  });

  it('switching the range type resets the rows like _resetRangeInfo', () => {
    const icons = withRangeType(ValueDisplay(), RangeType.Icon, 'airplane.svg');
    expect(icons.values).toEqual([0.0, 100.0]);
    expect(icons.icons).toEqual(['airplane.svg', 'airplane.svg', 'airplane.svg']);
    expect(icons.colours).toEqual([]);
    expect(withRow(icons, 'x.svg').values.length).toBe(3);
    expect(withRow(icons, 'x.svg').icons.at(-1)).toBe('x.svg');
    expect(withoutRow(icons, 0).values).toEqual([100.0]);
    expect(same(withRangeType(icons, RangeType.None, 'airplane.svg'), ValueDisplay())).toBe(true);
  });

  it('opacity and icon ranges pick by value, and a fixed icon replaces the label', () => {
    const faded = { ...withRangeType(ValueDisplay(), RangeType.Opacity, 'a.svg'), opacities: [0.2, 0.5, 1.0] };
    expect(displayOpacity(faded, -1.0)).toBe(0.2);
    expect(displayOpacity(faded, 150.0)).toBe(1);
    expect(displayOpacity(ValueDisplay(), 5.0)).toBe(1);
    const swapped = { ...withRangeType(ValueDisplay(), RangeType.Icon, 'a.svg'), icons: ['low.svg', 'mid.svg', 'high.svg'] };
    expect(displayIcon(swapped, 50.0)).toBe('mid.svg');
    expect(displayIcon(ValueDisplay({ showIcon: true, icon: 'plane.svg' }), 5.0)).toBe('plane.svg');
    expect(displayIcon(ValueDisplay(), 5.0)).toBeNull();
  });

  it('a display survives a round trip through storage', () => {
    const display = ValueDisplay({ text: 'Alt', showIcon: true, icon: 'plane.svg', rangeType: RangeType.Opacity, values: [1.0], opacities: [0.5, 1.0] });
    expect(same(displayFrom(displayJson(display)), display)).toBe(true);
  });
});
