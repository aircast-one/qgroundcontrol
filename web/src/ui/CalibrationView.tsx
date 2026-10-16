import type { Json } from '@/bridge/Json';

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
