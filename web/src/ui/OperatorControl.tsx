import { type Json, isNull, optBoolean, optInt } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { settingControl } from '@/bridge/QgcState';
import { format } from '@/kotlin/Std';

export const OPERATOR_CONTROL_VIEW = 'view.operatorControl';
export const REQUEST_CONTROL = 'vehicle.requestOperatorControl';
export const ALLOW_TAKEOVER_PATH = 'settings.flyViewSettings.requestControlAllowTakeover';
export const REQUEST_TIMEOUT_PATH = 'settings.flyViewSettings.requestControlTimeout';
export const GCS_SYSTEM_ID_PATH = 'settings.mavlinkSettings.gcsMavlinkSystemID';
export const ALLOW_TAKEOVER_SETTING = settingControl(ALLOW_TAKEOVER_PATH);
export const REQUEST_TIMEOUT_SETTING = settingControl(REQUEST_TIMEOUT_PATH);

export type ControlStation = {
  known: boolean;
  inControl: boolean | null;
  holderSystemId: number | null;
  takeoverAllowed: boolean | null;
  requestAllowed: boolean;
  reason: string;
};

export type ControlAsk = { refusal: string | null; timeoutSeconds: number };

export const controlStation = (_view: Json | null): ControlStation | null => null;

export const controlIsElsewhere = (station: ControlStation | null): boolean => station?.inControl === false;

export const controlLine = (station: ControlStation | null): string | null =>
  station === null || !station.known || station.inControl === true || station.reason.trim() === ''
    ? null
    : station.holderSystemId === null
      ? station.reason
      : `${station.reason} (system ${station.holderSystemId})`;

export const holderLine = (station: ControlStation | null): string | null =>
  station === null || !station.known || station.inControl !== false
    ? null
    : station.holderSystemId !== null
      ? `System in control: ${station.holderSystemId}`
      : station.reason.trim() === ''
        ? null
        : station.reason;

export const acquireLabel = (station: ControlStation | null): string | null =>
  station === null || !station.known || station.inControl !== false ? null : station.takeoverAllowed === true ? 'Acquire control' : 'Send request';

export const acquireEnabled = (station: ControlStation, counting: boolean): boolean => station.requestAllowed && !counting;

export const controlWaitLine = (station: ControlStation | null): string | null =>
  station === null || !station.known || station.inControl !== false || station.requestAllowed ? null : 'Waiting for the other station to answer';

export const inControlLine = (station: ControlStation | null): string | null =>
  station === null || !station.known || station.inControl !== true
    ? null
    : station.holderSystemId === null
      ? 'System in control: This GCS'
      : `System in control: This GCS (${station.holderSystemId})`;

export const takeoverLine = (station: ControlStation | null): string | null =>
  station === null || !station.known || station.takeoverAllowed === null ? null : station.takeoverAllowed ? 'Takeover allowed' : 'Takeover NOT allowed';

export const takeoverChangeable = (station: ControlStation | null, allowTakeover: boolean | null): boolean =>
  station?.inControl === true && allowTakeover !== null && station.takeoverAllowed !== allowTakeover;

export const requestSentLabel = (remainingMs: number): string => `Request sent: ${format('%.1f', Math.max(remainingMs, 0) / 1000.0)}`;

export const allowTakeoverEditable = (station: ControlStation | null): boolean => station?.inControl === true || station?.takeoverAllowed === true;

export const controlSectionTitle = (station: ControlStation): string | null =>
  station.inControl === true ? 'Change takeover condition' : station.inControl === false ? 'Send control request' : null;

export const requestTimeoutEditable = (station: ControlStation): boolean => station.inControl === false && station.takeoverAllowed !== true;

export const requestTimeoutSeconds = (station: ControlStation, setting: number): number => (station.takeoverAllowed === true ? 0 : setting);

export const allowTakeoverSetting = async (): Promise<boolean | null> => {
  const read = await Qgc.get(ALLOW_TAKEOVER_SETTING);
  return isNull(read, 'value') ? null : optBoolean(read, 'value');
};

export const saveAllowTakeover = async (allow: boolean): Promise<string | null> =>
  (await Qgc.set(`${ALLOW_TAKEOVER_PATH}.rawValue`, allow)) ? null : 'This station could not save whether it allows a takeover.';

export const changeTakeover = async (allow: boolean): Promise<string | null> => ((await Qgc.invoke(REQUEST_CONTROL, allow, 0)) ? null : 'The vehicle did not take the change.');

export const askForControl = async (station: ControlStation): Promise<ControlAsk> => {
  const allowTakeover = await allowTakeoverSetting();
  if (allowTakeover === null) return { refusal: 'This station cannot tell whether it would allow a takeover, so it did not ask.', timeoutSeconds: 0 };
  const read = await Qgc.get(REQUEST_TIMEOUT_SETTING);
  const timeout = isNull(read, 'value') ? null : optInt(read, 'value');
  if (timeout === null) return { refusal: 'This station cannot tell how long it would wait, so it did not ask.', timeoutSeconds: 0 };
  const seconds = requestTimeoutSeconds(station, timeout);
  return (await Qgc.invoke(REQUEST_CONTROL, allowTakeover, seconds)) ? { refusal: null, timeoutSeconds: seconds } : { refusal: 'The vehicle did not take the request.', timeoutSeconds: 0 };
};
