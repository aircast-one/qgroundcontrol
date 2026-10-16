import { describe, expect, it } from 'vitest';
import { setupSummaries } from '@/ui/SetupScreen';
import { SummaryLine, summaryGlance, summaryGlanceParts } from './SetupRow';

describe('SetupSummaryTest', () => {
  it('summary rows are keyed by the component they belong to', () => {
    const read = setupSummaries({ components: [{ name: 'Radio', rows: [{ label: 'Roll', value: 'Channel 1' }, { label: 'Pitch', value: 'Setup required' }] }, { name: 'Frame', rows: [] }] });
    expect(read.get('Radio')).toEqual([SummaryLine({ label: 'Roll', value: 'Channel 1' }), SummaryLine({ label: 'Pitch', value: 'Setup required' })]);
    expect(read.get('Frame')).toEqual([]);
    expect(setupSummaries(null)).toEqual(new Map());
    const joystick = setupSummaries({ components: [{ name: 'Joystick', rows: [{ label: 'Battery', value: '15%', warn: true }] }] });
    expect(joystick.get('Joystick')).toEqual([SummaryLine({ label: 'Battery', value: '15%', warn: true })]);
  });

  it('a warned summary value keeps its warning in the glance, as JoystickComponentSummary paints a low battery red', () => {
    const parts = summaryGlanceParts([SummaryLine({ label: 'Status', value: 'Ready' }), SummaryLine({ label: 'Battery', value: '15% (Discharging)', warn: true })]);
    expect(parts.map((it) => it.warn)).toEqual([false, true]);
    expect(summaryGlance([SummaryLine({ label: 'Status', value: 'Ready' }), SummaryLine({ label: 'Battery', value: '15% (Discharging)', warn: true })])).toBe('Ready · 15% (discharging)');
  });

  it("a summary reads as one line of its values, as Penpot's setup rows", () => {
    const frame = [SummaryLine({ label: 'Frame Class', value: 'Quad' }), SummaryLine({ label: 'Frame Type', value: 'X' }), SummaryLine({ label: 'Firmware Version', value: 'Unknown' })];
    expect(summaryGlance(frame)).toBe('Quad · X');
    expect(summaryGlance([SummaryLine({ label: 'a', value: 'RTL' }), SummaryLine({ label: 'b', value: 'Land' }), SummaryLine({ label: 'c', value: 'RTL' })])).toBe('RTL · Land');
    expect(summaryGlance([])).toBeNull();
    expect(summaryGlance([SummaryLine({ label: 'Battery monitor', value: 'Analog Voltage and Current' }), SummaryLine({ label: 'Capacity', value: '5000 mAh' })])).toBe('Analog voltage and current · 5000 mAh');
    const radio = [SummaryLine({ label: 'Roll', value: 'Channel 1' }), SummaryLine({ label: 'Pitch', value: 'Channel 2' }), SummaryLine({ label: 'Throttle', value: 'Channel 3' })];
    expect(summaryGlance(radio)).toBe('Roll 1 · Pitch 2 · Throttle 3');
    expect(summaryGlance([SummaryLine({ label: 'Compass', value: 'Ready' }), SummaryLine({ label: 'Airspeed', value: 'Not Supported(Over APM 4.1)' })])).toBe('Ready · Not supported (over APM 4.1)');
  });
});
