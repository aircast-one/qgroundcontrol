import { type Json, type JsonArray, jsonObjects, optDouble, optInt, optJSONArray, optJSONObject, optText, optBoolean } from '@/bridge/Json';

export const RADIO_VIEW = 'view.radio';

export type RadioStick = { title: string; valueText: string; fraction: number; mapped: boolean; reversed: boolean; channel: number | null };

export const RadioStick = (fields: { title: string; valueText: string; fraction: number; mapped: boolean; reversed: boolean; channel?: number | null }): RadioStick => ({ channel: null, ...fields });

export type RadioChannel = { label: string; valueText: string; fraction: number; live: boolean };

export type RadioCalibration = {
  running: boolean;
  statusText: string;
  nextText: string;
  nextEnabled: boolean;
  cancelEnabled: boolean;
  skipEnabled: boolean;
  throttleReversed: boolean;
  stickPositions: readonly number[];
};

export type RadioView = {
  connected: boolean;
  channelCount: number;
  summary: string;
  calibration: RadioCalibration;
  transmitterMode: number;
  centeredThrottle: boolean;
  joystickMode: boolean;
  sticks: readonly RadioStick[];
  channels: readonly RadioChannel[];
  startPrompt: readonly [string, string] | null;
  notReady: readonly [string, string] | null;
};

const each = <T,>(view: Json, key: string, make: (json: Json) => T): T[] => jsonObjects(optJSONArray(view, key)).map(make);

const ints = (array: JsonArray): number[] => array.map((value) => optInt({ value }, 'value'));

const NO_STICKS: readonly number[] = [0, 0, 0, 0];

const pair = (json: Json | null): readonly [string, string] | null => (json === null ? null : [optText(json, 'title'), optText(json, 'message')]);

const stickPositions = (array: JsonArray | null): readonly number[] => {
  const read = array === null ? null : ints(array);
  return read !== null && read.length === 4 ? read : NO_STICKS;
};

export const radioView = (view: Json | null): RadioView | null =>
  view === null || optText(view, 'class') !== 'Radio'
    ? null
    : {
        connected: optBoolean(view, 'connected'),
        channelCount: optInt(view, 'channelCount'),
        summary: optText(view, 'summary'),
        transmitterMode: optInt(view, 'transmitterMode', 2),
        centeredThrottle: optBoolean(view, 'centeredThrottle'),
        joystickMode: optBoolean(view, 'joystickMode'),
        startPrompt: pair(optJSONObject(view, 'startPrompt')),
        notReady: pair(optJSONObject(view, 'notReady')),
        calibration: {
          running: optBoolean(view, 'calibrating'),
          statusText: optText(view, 'statusText'),
          nextText: optText(view, 'nextText'),
          nextEnabled: optBoolean(view, 'nextEnabled'),
          cancelEnabled: optBoolean(view, 'cancelEnabled'),
          skipEnabled: optBoolean(view, 'skipEnabled'),
          throttleReversed: optBoolean(view, 'throttleReversed'),
          stickPositions: stickPositions(optJSONArray(view, 'stickPositions')),
        },
        sticks: each(view, 'sticks', (it) => {
          const channel = optInt(it, 'channel');
          return RadioStick({
            title: optText(it, 'title'),
            valueText: optText(it, 'valueText'),
            fraction: optDouble(it, 'fraction', 0.0),
            mapped: optBoolean(it, 'mapped'),
            reversed: optBoolean(it, 'reversed'),
            channel: channel > 0 ? channel : null,
          });
        }),
        channels: each(view, 'channels', (it) => ({
          label: optText(it, 'label'),
          valueText: optText(it, 'valueText'),
          fraction: optDouble(it, 'fraction', 0.0),
          live: optBoolean(it, 'live'),
        })),
      };

export const RADIO_CAL = 'radioCal';

export const radioCalAction = (action: string): string => `${RADIO_CAL}.${action}`;

export const calibrationStep = (statusText: string): string => {
  const lines = statusText.trim().split(/\r\n|\n|\r/);
  const kept = lines.findLastIndex((it) => !(it.trim() === '' || it.trim().startsWith('Click ')));
  return lines.slice(0, kept + 1).join('\n').trim();
};

export type RadioPrompt = { title: string; body: string; action: string; choices: readonly string[] };

export const RADIO_PROMPTS: readonly RadioPrompt[] = [
  {
    title: 'Spektrum bind',
    body: 'Click Ok to place your Spektrum receiver in the bind mode.\n\nSelect the specific receiver type below:',
    action: 'spektrumBindMode',
    choices: ['DSM2 Mode', 'DSMX (7 channels or less)', 'DSMX (8 channels or more)'],
  },
  {
    title: 'CRSF bind',
    body: 'Click Ok to place your CRSF receiver in the bind mode.',
    action: 'crsfBindMode',
    choices: [],
  },
  {
    title: 'Copy trims',
    body: 'Center your sticks and move throttle all the way down, then press Ok to copy trims. After pressing Ok, reset the trims on your radio back to zero.',
    action: 'copyTrims',
    choices: [],
  },
];

export const THROTTLE_REVERSED_TITLE = 'Throttle channel reversed';
export const THROTTLE_REVERSED_TEXT = 'Calibration failed. The throttle channel on your transmitter is reversed. You must correct this on your transmitter in order to complete calibration.';
