import { type Json, jsonObjects, jsonStrings, optBoolean, optJSONArray, optText } from '@/bridge/Json';

export const REBOOT_VEHICLE = 'vehicle.rebootVehicle';
export const CALIBRATION = 'view.calibration';

type FastCompass = { invocation: string; help: string; vehicleHasPosition: boolean; gcsLatitude: number | null; gcsLongitude: number | null };

export type CalibrationSide = { key: string; title: string; visible: boolean; stage: string; rotate: boolean };

export type CompassResult = { compass: number; green: number; yellow: number; range: number; position: number };

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

export const calibrationState = (_view: Json | null): CalibrationState | null => null;

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
