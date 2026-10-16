import type { Json } from '@/bridge/Json';

export const RADIO_VIEW = 'view.radio';

export type RadioStick = { title: string; valueText: string; fraction: number; mapped: boolean; reversed: boolean; channel: number | null };

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

export const radioView = (_view: Json | null): RadioView | null => null;

export const RADIO_CAL = 'radioCal';

export const radioCalAction = (action: string): string => `${RADIO_CAL}.${action}`;
