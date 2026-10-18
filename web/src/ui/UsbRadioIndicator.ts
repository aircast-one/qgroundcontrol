import { type Json, optJSONObject, optText } from '@/bridge/Json';

export const USB_RADIO_VIEW = 'view.usbRadio';

export type UsbRadioState = 'live' | 'paused' | 'standby' | 'ready' | 'stopped';

export type UsbRadioBadge = { text: string; state: UsbRadioState };

const STATES: readonly UsbRadioState[] = ['live', 'paused', 'standby', 'ready', 'stopped'];

export const usbRadioBadge = (view: Json | null): UsbRadioBadge | null => {
  const it = optJSONObject(view, 'indicator');
  const state = STATES.find((known) => known === optText(it, 'state'));
  return it === null || state === undefined ? null : { text: optText(it, 'text'), state };
};

export const usbRadioTone = (state: UsbRadioState): 'error' | 'warning' | null => (state === 'stopped' ? 'error' : state === 'paused' ? 'warning' : null);
