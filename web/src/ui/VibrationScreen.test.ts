import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { PARTIAL_TITLE, type VibrationReading, bandCaption, scaleLabels, severityLabel, silentState, vibrationEmptyState, vibrationHeading, vibrationReading, vibrationVerdict } from './VibrationScreen';

const json = (text: string): Json => JSON.parse(text) as Json;

describe('VibrationScreenTest', () => {
  const served = `
    {"available":true,"units":"m/s²","scaleMaximum":90,"warningLevel":30,"dangerLevel":60,
     "axes":[
       {"axis":"X","value":15.0,"fraction":0.1667,"severity":"normal"},
       {"axis":"Y","value":45.0,"fraction":0.5,"severity":"warning"},
       {"axis":"Z","value":75.0,"fraction":0.8333,"severity":"danger"}],
     "clipCounts":[0,3,12],"worst":"danger","clipping":true}
  `;

  it('each severity keeps the word this screen has always shown', () => {
    expect(severityLabel('normal')).toBe('Healthy');
    expect(severityLabel('warning')).toBe('Watch');
    expect(severityLabel('danger')).toBe('Unsafe');
  });

  it('an absent severity is blank rather than a guess', () => {
    expect(severityLabel(null)).toBe('');
    expect(severityLabel('something the core added later')).toBe('');
  });

  it('the reading carries the axes and clip counts the core served', () => {
    const reading = vibrationReading(json(served));
    expect(reading?.axes.map((it) => it.axis)).toEqual(['X', 'Y', 'Z']);
    expect(reading?.axes.map((it) => it.value)).toEqual([15.0, 45.0, 75.0]);
    expect(reading?.axes.map((it) => it.severity)).toEqual(['normal', 'warning', 'danger']);
    expect(reading?.clipCounts).toEqual([0, 3, 12]);
  });

  it('a null axis reads as absent, not as zero at the bottom of the scale', () => {
    const reading = vibrationReading(
      json(`{"available":true,"units":"m/s²","scaleMaximum":90,
        "warningLevel":30,"dangerLevel":60,
        "axes":[{"axis":"X","value":null,"fraction":null,"severity":null}],
        "clipCounts":[]}`),
    );
    expect(reading?.axes[0]?.value).toBeNull();
    expect(reading?.axes[0]?.severity).toBeNull();
    expect(reading?.axes[0]?.fraction).toBeCloseTo(0, 6);
  });

  it('an unavailable view is no reading at all', () => {
    expect(vibrationReading(null)).toBeNull();
    expect(vibrationReading(json('{"available":false}'))).toBeNull();
  });

  it('a healthy three-axis reading draws the bars rather than an empty state', () => {
    const healthy = json(
      `{"kind":"object","class":"Vibration","connected":true,"available":true,
         "silentReason":null,"silentText":null,"units":"m/s^2","scaleMaximum":90,
         "warningLevel":30,"dangerLevel":60,"clipCounts":[0,0,0],
         "axes":[{"axis":"x","label":"X","value":12.0,"fraction":0.13,"severity":"normal"},
                 {"axis":"y","label":"Y","value":14.0,"fraction":0.15,"severity":"normal"},
                 {"axis":"z","label":"Z","value":16.0,"fraction":0.17,"severity":"normal"}]}`,
    );
    expect(vibrationReading(healthy)).not.toBeNull();
    expect(vibrationEmptyState(healthy, vibrationReading(healthy)), 'an empty state here means the screen never draws its bars at all').toBeNull();
  });

  it('some axes reported and some not is neither a reading nor a silence', () => {
    const partial = json(
      `{"kind":"object","class":"Vibration","connected":true,"available":false,
         "silentReason":null,"silentText":null,"units":"m/s^2",
         "axes":[{"axis":"x","label":"X","value":12.0,"fraction":0.13,"severity":"normal"},
                 {"axis":"y","label":"Y","value":null,"fraction":null,"severity":null},
                 {"axis":"z","label":"Z","value":null,"fraction":null,"severity":null}]}`,
    );
    expect(silentState(partial), 'silentReason is set only when NO axis has a value, so a vehicle sending NaN in one axis of a VIBRATION message leaves this null').toBeNull();
    expect(
      vibrationReading(partial),
      'available is all three, so the same view produces no reading - the screen used to read reading!! after a silentState guard and threw on exactly this input',
    ).toBeNull();
    expect(
      vibrationEmptyState(partial, vibrationReading(partial))?.title,
      'neither half covers it, so the screen has to; a guard that asks silentState alone sends this input to the bars it has no reading for',
    ).toBe(PARTIAL_TITLE);
  });

  it('partly reported without a vehicle is a connect prompt, not a claim about one', () => {
    const latched = json(
      `{"kind":"object","class":"Vibration","connected":false,"available":false,
         "silentReason":null,"silentText":null,"units":"m/s^2",
         "axes":[{"axis":"x","label":"X","value":12.0,"fraction":0.13,"severity":"normal"},
                 {"axis":"y","label":"Y","value":null,"fraction":null,"severity":null},
                 {"axis":"z","label":"Z","value":null,"fraction":null,"severity":null}]}`,
    );
    expect(
      vibrationEmptyState(latched, vibrationReading(latched))?.title,
      'vibration facts latch after the vehicle goes, so silentReason stays null with nothing connected - saying what THIS VEHICLE is reporting would be a claim about one that is not there',
    ).toBe('No vehicle connected');
  });

  it("the scale and the caption are built from the core's own levels", () => {
    expect(scaleLabels(90.0, 30.0, 60.0)).toEqual(['90', '60', '30', '0']);
    expect(bandCaption(30.0, 60.0)).toBe('Under 30 healthy · 30-60 watch · over 60 unsafe');
  });

  it('a blank unit leaves no empty brackets in the heading', () => {
    expect(vibrationHeading('m/s²')).toBe('Vibration (m/s²)');
    expect(vibrationHeading('')).toBe('Vibration');
  });
});

describe('SilentStateTest', () => {
  const view = (reason: string, text: string) =>
    json(`{"kind":"object","class":"Vibration","connected":true,"available":false,
       "silentReason":${reason},"silentText":${text},"axes":[],"units":"m/s^2"}`);

  it('a reporting vehicle has no empty state', () => {
    expect(silentState(view('null', 'null'))).toBeNull();
  });

  it("the title is the core's sentence and the body is this head's instruction", () => {
    const gone = silentState(view('"noVehicle"', '"No vehicle is connected."'));
    expect(gone?.title).toBe('No vehicle is connected.');
    expect(gone?.body).toBe('Connect a vehicle from the Fly view to see its vibration levels.');
  });

  it('connected and silent gets the other instruction, chosen by token not by wording', () => {
    const mute = silentState(view('"notReported"', '"This vehicle reports no vibration measurements."'));
    expect(mute?.title).toBe('This vehicle reports no vibration measurements.');
    expect(mute?.body.startsWith('The autopilot has not sent a VIBRATION message')).toBe(true);
  });

  it('a token this head has never seen still gets a body', () => {
    const odd = silentState(view('"somethingNew"', '"Something new happened."'));
    expect(odd?.title).toBe('Something new happened.');
    expect((odd?.body ?? '').trim()).not.toBe('');
  });

  it('no view at all is the disconnected case', () => {
    expect(silentState(null)?.title).toBe('No vehicle connected');
  });

  const reading = (severities: readonly (string | null)[], clips: readonly number[] = [0, 0, 0]): VibrationReading => ({
    units: 'm/s/s',
    scaleMaximum: 90.0,
    warningLevel: 30.0,
    dangerLevel: 60.0,
    axes: severities.map((severity, index) => ({ axis: ['X', 'Y', 'Z'][index] ?? '', value: 1.0, fraction: 0.1, severity })),
    clipCounts: clips,
  });

  it('theVerdictNamesTheWorstAxesAndTheClipping', () => {
    expect(vibrationVerdict(reading(['normal', 'warning', 'danger'], [0, 3, 12]))).toBe('Vibration on Z is over the unsafe limit of 60. The accelerometers clipped 15 times; expect zero in flight.');
    expect(vibrationVerdict(reading(['warning', 'warning', 'normal']))).toBe('Vibration on X and Y is above 30; watch it. No clipping.');
    expect(vibrationVerdict(reading(['normal', 'normal', 'normal']))).toBe('Vibration is well under the limit. No clipping.');
  });
});
