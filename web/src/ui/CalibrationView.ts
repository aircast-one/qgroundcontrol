import { type Json, type JsonArray, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type FastCompass, fastCompass } from '@/ui/FastCompass';

export const CALIBRATION = 'view.calibration';

export type CalibrationSide = { key: string; title: string; visible: boolean; stage: string; rotate: boolean };

export type CalibrationRoutine = {
  id: string;
  title: string;
  invocation: string;
  arguments: readonly boolean[];
  blocked: boolean;
  enabled: boolean;
  description: string;
  dialogHelp: string;
  status: string;
  warning: string;
  spinsPropeller: boolean;
  dialogTitle: string;
};

export type CalibrationState = {
  connected: boolean;
  inProgress: boolean;
  showsSides: boolean;
  nextEnabled: boolean;
  cancelEnabled: boolean;
  progress: number;
  waitingForCancel: boolean;
  helpText: string;
  statusText: string;
  completed: string;
  accelNeeded: boolean;
  compassNeeded: boolean;
  needsAttention: string;
  px4: boolean;
  settingsTitle: string;
  settingsDialogTitle: string;
  fastCompass: FastCompass | null;
  sides: readonly CalibrationSide[];
  routines: readonly CalibrationRoutine[];
  compassResults: readonly CompassResult[];
};

export type CompassResult = { compass: number; green: number; yellow: number; range: number; position: number };

const list = <T,>(json: Json, key: string, item: (json: Json) => T): T[] => jsonObjects(optJSONArray(json, key)).map(item);

const booleans = (array: JsonArray | null): boolean[] =>
  (array ?? []).map((value) => value === true || (typeof value === 'string' && value.toLowerCase() === 'true'));

export const calibrationState = (view: Json | null): CalibrationState | null =>
  view === null || optText(view, 'class') !== 'Calibration'
    ? null
    : {
        connected: optBoolean(view, 'connected'),
        inProgress: optBoolean(view, 'inProgress'),
        showsSides: optBoolean(view, 'showsSides'),
        nextEnabled: optBoolean(view, 'nextEnabled'),
        cancelEnabled: optBoolean(view, 'cancelEnabled'),
        progress: optDouble(view, 'progress', 0.0),
        waitingForCancel: optBoolean(view, 'waitingForCancel'),
        helpText: optText(view, 'helpText'),
        statusText: optText(view, 'statusText'),
        completed: optText(view, 'completed'),
        accelNeeded: optBoolean(view, 'accelNeeded'),
        compassNeeded: optBoolean(view, 'compassNeeded'),
        needsAttention: optText(view, 'needsAttention'),
        px4: optBoolean(view, 'px4'),
        settingsTitle: optText(view, 'settingsTitle'),
        settingsDialogTitle: optText(view, 'settingsDialogTitle'),
        fastCompass: fastCompass(optJSONObject(view, 'fastCompass')),
        compassResults: list(view, 'compassResults', (it) => ({
          compass: optInt(it, 'compass'),
          green: optDouble(it, 'green'),
          yellow: optDouble(it, 'yellow'),
          range: optDouble(it, 'range'),
          position: optDouble(it, 'position'),
        })),
        sides: list(view, 'sides', (it) => ({
          key: optText(it, 'key'),
          title: optText(it, 'title'),
          visible: optBoolean(it, 'visible'),
          stage: optText(it, 'stage'),
          rotate: optBoolean(it, 'rotate'),
        })),
        routines: list(view, 'routines', (it) => ({
          id: optText(it, 'id'),
          title: optText(it, 'title'),
          invocation: optText(it, 'invocation'),
          arguments: booleans(optJSONArray(it, 'arguments')),
          blocked: optBoolean(it, 'blocked'),
          enabled: optBoolean(it, 'enabled'),
          description: optText(it, 'description'),
          dialogHelp: optText(it, 'dialogHelp'),
          status: optText(it, 'status'),
          warning: optText(it, 'warning'),
          spinsPropeller: optBoolean(it, 'spinsPropeller'),
          dialogTitle: optText(it, 'dialogTitle'),
        })),
      };

export const CANCEL_WAIT_TITLE = 'Cancelling calibration';
export const CANCEL_WAIT_TEXT = 'Waiting for Vehicle to response to Cancel. This may take a few seconds.';

export const COMPASS_ROUTINE = 'compass';

export const REBOOT_VEHICLE = 'vehicle.rebootVehicle';

export const runningTitle = (name: string): string => (name.trim() === '' ? 'Calibration in progress' : `Calibrating ${name.trim()}`);

export const SENSOR_HEALTH = 'view.sensors';

export type SensorHealth = { name: string; state: string; label: string };

export type SensorHealthReading = { available: boolean; sensors: readonly SensorHealth[]; failing: readonly string[]; status: string };

export const sensorHealth = (view: Json | null): SensorHealthReading | null =>
  view === null || optText(view, 'class') !== 'SensorHealth'
    ? null
    : {
        available: optBoolean(view, 'available'),
        sensors: jsonObjects(optJSONArray(view, 'sensors')).flatMap((item) => {
          const name = optText(item, 'name');
          return name.trim() === '' ? [] : [{ name, state: optText(item, 'state'), label: optText(item, 'label') }];
        }),
        failing: jsonStrings(optJSONArray(view, 'failing')).filter((it) => it.trim() !== ''),
        status: optText(view, 'status'),
      };

export const healthSummary = (reading: SensorHealthReading | null): string =>
  reading === null || !reading.available
    ? ''
    : reading.failing.length === 1
      ? `${reading.failing[0]} is reporting a fault.`
      : reading.failing.length > 1
        ? `${reading.failing.length} sensors are reporting faults.`
        : '';
