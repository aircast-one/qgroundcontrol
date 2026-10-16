import { type Json, isNull, optText } from '@/bridge/Json';
import type { SnackbarDuration } from '@/m3/Feedback';

export const WARNINGS = 'view.warnings';

const PREARM_PREFIX = 'PreArm:';

export type SnackbarData = { visuals: { message: string; duration: SnackbarDuration }; dismiss: () => void };

export type ArmingCheck = { message: string; description: string; severity: string };

export const armingBlocker = (view: Json | null): string | null => {
  if (view === null || isNull(view, 'armingBlocker')) return null;
  const blocker = optText(view, 'armingBlocker');
  return blocker.trim() === '' ? null : blocker;
};

export const armingChecks = (_view: Json | null): readonly ArmingCheck[] | null => null;

export const chipBlocker = (blocker: string): string => {
  const trimmed = blocker.trim();
  const rest = (trimmed.startsWith(PREARM_PREFIX) ? trimmed.slice(PREARM_PREFIX.length) : trimmed).trim();
  const sentence = rest.includes('. ') ? rest.slice(0, rest.indexOf('. ')) : rest;
  return sentence.replace(/\.+$/, '');
};

export const AppSnackbar = (_props: { data: SnackbarData }) => null;

export const VehicleMessageBanner = (_props: { className?: string }) => null;

export const VehicleMessagesSheet = (_props: { onDismiss: () => void }) => null;
