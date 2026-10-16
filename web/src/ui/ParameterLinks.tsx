import type { Fact } from '@/bridge/Qgc';
import type { Typography } from '@/m3';

export const SENSOR_SETTINGS_DIALOG = 'sensorSettings';
export const SETUP_DIALOG_OPENED = 'setup.dialogOpened';
export const VALUE_DETAILS_TITLE = 'Value Details';

export const ValueDetailsSheet = (_props: { fact: Fact; title?: string; onWrite: () => void; onDismiss: () => void }) => null;

export const EDIT_PARAMETER_TITLE = 'Edit parameter';

export const LinkedText = (_props: { html: string; style: Typography; color: string; onParameter: (name: string) => void; className?: string }) => null;

export const ParameterEditDialog = (_props: { name: string; title?: string; onDismiss: () => void }) => null;
