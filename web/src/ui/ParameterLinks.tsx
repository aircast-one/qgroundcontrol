import type { Fact } from '@/bridge/Qgc';

export const SENSOR_SETTINGS_DIALOG = 'sensorSettings';
export const SETUP_DIALOG_OPENED = 'setup.dialogOpened';
export const VALUE_DETAILS_TITLE = 'Value Details';

export const ValueDetailsSheet = (_props: { fact: Fact; title?: string; onWrite: () => void; onDismiss: () => void }) => null;
