import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { RADIO_ENTER_ACTIONS, RADIO_LEAVE_ACTIONS, stickReading } from '@/ui/RadioScreen';
import { RadioStick, type RadioView, calibrationStep, radioCalAction, radioView } from '@/ui/RadioView';

const TWO_DEAD = `
{"class": "Radio", "connected": true, "channelCount": 8, "liveChannels": 6,
 "summary": "8 channels reported, 6 carrying a signal.",
 "sticks": [{"key": "yaw", "title": "Yaw", "value": 1173, "valueText": "1173",
             "fraction": 0.173, "mapped": true, "reversed": true}],
 "channels": [{"index": 6, "label": "7", "value": 0, "valueText": "—", "fraction": 0.0, "live": false},
              {"index": 0, "label": "1", "value": 1607, "valueText": "1607", "fraction": 0.607, "live": true}]}
`;

const MID_CALIBRATION = `
{"class": "Radio", "connected": true, "channelCount": 8, "liveChannels": 8,
 "summary": "8 channels reported, 8 carrying a signal.",
 "calibrating": true, "statusText": "Move the Throttle stick all the way up and hold it there...",
 "nextText": "Next", "nextEnabled": true, "cancelEnabled": true, "skipEnabled": true,
 "sticks": [], "channels": []}
`;

const json = (text: string): Json => parseJson(text) ?? {};

const read = (view: Json): RadioView => {
  const radio = radioView(view);
  if (radio === null) throw new Error('not the radio view');
  return radio;
};

describe('RadioViewTest', () => {
  it('a calibration in progress carries the step text and the buttons the core enables', () => {
    const cal = read(json(MID_CALIBRATION)).calibration;
    expect(cal.running).toBe(true);
    expect(cal.nextText).toBe('Next');
    expect(cal.cancelEnabled).toBe(true);
    expect(cal.skipEnabled, 'a step the operator cannot perform has to be skippable').toBe(true);
    expect(cal.statusText.startsWith('Move the Throttle stick')).toBe(true);
  });

  it('an idle radio offers the start label and nothing to cancel', () => {
    const idle = read(json(TWO_DEAD)).calibration;
    expect(idle.running).toBe(false);
    expect(idle.cancelEnabled, 'nothing is running, so cancel would cancel nothing').toBe(false);
  });

  it('too few channels carries the dialog QGC shows instead of starting', () => {
    const short = read({ ...json(TWO_DEAD), notReady: { title: 'Not Ready', message: 'Please turn on RC transmitter.' } });
    expect(short.notReady).toEqual(['Not Ready', 'Please turn on RC transmitter.']);
    expect(read(json(TWO_DEAD)).notReady).toBeNull();
  });

  it('the action path names the controller the core reads its state from', () => {
    expect(radioCalAction('nextButtonClicked')).toBe('radioCal.nextButtonClicked');
    expect(radioCalAction('cancelButtonClicked')).toBe('radioCal.cancelButtonClicked');
    expect(RADIO_ENTER_ACTIONS.map(radioCalAction)).toEqual(['radioCal.start', 'radioCal.open']);
    expect(RADIO_LEAVE_ACTIONS.map(radioCalAction)).toEqual(['radioCal.cancelButtonClicked', 'radioCal.close']);
  });

  it('a channel carrying no signal is a dash, never a pwm of zero', () => {
    const dead = read(json(TWO_DEAD)).channels.find((it) => !it.live);
    expect(dead?.valueText).toBe('—');
    expect(dead?.valueText === '0', 'a reading of 0 would claim the receiver measured the stick at its floor').toBe(false);
  });

  it('the summary counts the channels carrying a signal, not the channels reported', () => {
    const view = read(json(TWO_DEAD));
    expect(view.channelCount).toBe(8);
    expect(view.summary).toBe('8 channels reported, 6 carrying a signal.');
    expect(view.summary.includes('6 carrying'), 'the count the header used to show cannot say two are dead').toBe(true);
  });

  it('a reversed stick keeps that flag, which is the only thing marking it on the row', () => {
    const sticks = read(json(TWO_DEAD)).sticks;
    expect(sticks).toHaveLength(1);
    expect(sticks[0]?.reversed).toBe(true);
  });

  it('anything that is not the radio view is refused rather than read as an empty radio', () => {
    expect(radioView({ class: 'Track', points: [] })).toBeNull();
    expect(radioView(null)).toBeNull();
  });
});

describe('CalibrationStepTest', () => {
  it('the line telling the operator to click is dropped, because the button is right there', () => {
    const served = 'Lower the Throttle stick all the way down.\n\n' + 'Reset all transmitter trims to center.\n\nClick Next to continue';
    expect(calibrationStep(served)).toBe('Lower the Throttle stick all the way down.\n\nReset all transmitter trims to center.');
  });

  it('a step that does not end in an instruction to click is left alone', () => {
    const served = 'Move the Throttle stick all the way up and hold it there...';
    expect(calibrationStep(served)).toBe(served);
  });

  it('nothing served is nothing shown', () => {
    expect(calibrationStep('')).toBe('');
    expect(calibrationStep('   \n  ')).toBe('');
  });

  it('the zero trims prompt comes from the core', () => {
    expect(radioView({ class: 'Radio', startPrompt: { title: 'Zero Trims', message: 'Before calibrating' } })?.startPrompt).toEqual(['Zero Trims', 'Before calibrating']);
    expect(radioView({ class: 'Radio', startPrompt: null })?.startPrompt).toBeNull();
  });

  it('a stick reading carries its pulse unit like the Penpot radio rows', () => {
    expect(stickReading(RadioStick({ title: 'Roll', valueText: '1605', fraction: 0.6, mapped: true, reversed: false }))).toBe('1605 µs');
    expect(stickReading(RadioStick({ title: 'Yaw', valueText: '1595', fraction: 0.6, mapped: true, reversed: true }))).toBe('1595 µs · reversed');
    expect(stickReading(RadioStick({ title: 'Yaw', valueText: '', fraction: 0, mapped: false, reversed: false }))).toBe('Not mapped');
    expect(stickReading(RadioStick({ title: 'Throttle', valueText: '1100', fraction: 0.1, mapped: true, reversed: false, channel: 3 }))).toBe('Ch 3 · 1100 µs');
  });
});
