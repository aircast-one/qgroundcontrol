import { type Json, opt, optBoolean, optInt, optJSONObject, optText } from '@/bridge/Json';
import { mutableStateOf } from '@/kotlin/State';
import { MapBridge } from './MapBridge';

export type VehicleSync = { readonly name: 'Offline' | 'Busy' | 'Ready'; readonly ordinal: number };

export const VehicleSync = {
  Offline: { name: 'Offline', ordinal: 0 },
  Busy: { name: 'Busy', ordinal: 1 },
  Ready: { name: 'Ready', ordinal: 2 },
} as const satisfies Record<string, VehicleSync>;

export const VehicleSyncEntries: readonly VehicleSync[] = [VehicleSync.Offline, VehicleSync.Busy, VehicleSync.Ready];

export const vehicleSyncState = (offline: boolean, syncing: boolean): VehicleSync => (offline ? VehicleSync.Offline : syncing ? VehicleSync.Busy : VehicleSync.Ready);

export const syncRefusal = (state: VehicleSync, action: string): string | null => {
  switch (state.name) {
    case 'Offline': return `No vehicle to ${action}`;
    case 'Busy': return 'Already syncing, wait for it to finish';
    case 'Ready': return null;
  }
};

export type UploadGate = {
  readonly canSend: boolean;
  readonly canProceed: boolean;
  readonly pausesFirst: boolean;
  readonly heading: string;
  readonly refusal: string;
  readonly proceedTitle: string;
};

export const notReadyToSend = (view: Json | null): string | null => {
  const readiness = optJSONObject(view, 'readiness');
  if (readiness === null || optBoolean(readiness, 'ready')) return null;
  const reason = optText(readiness, 'reason');
  return reason.trim() === '' ? 'The plan is not ready to send.' : reason;
};

export const NEWEST_PATTERN = -1;

export const nextNotReady = (view: Json | null): number | null => {
  const readiness = optJSONObject(view, 'readiness');
  if (readiness === null || optBoolean(readiness, 'ready')) return null;
  const next = optInt(readiness, 'next', -1);
  return next > 0 ? next : null;
};

export const PlanFocus = {
  requests: mutableStateOf<number | null>(null),
  newPattern: mutableStateOf<number | null>(null),

  notReady(view: Json | null): void {
    const next = nextNotReady(view);
    if (next !== null) PlanFocus.requests.value = next;
  },
};

export const uploadGate = (view: Json | null): UploadGate | null => {
  const upload = optJSONObject(view, 'upload');
  return upload === null
    ? null
    : {
        canSend: optBoolean(upload, 'canSend'),
        canProceed: optBoolean(upload, 'canProceed'),
        pausesFirst: optBoolean(upload, 'pausesFirst'),
        heading: optText(upload, 'heading'),
        refusal: optText(upload, 'refusal'),
        proceedTitle: optText(upload, 'proceedTitle'),
      };
};

export type UploadStep =
  | { readonly kind: 'Send' }
  | { readonly kind: 'Confirm'; readonly gate: UploadGate }
  | { readonly kind: 'Refuse'; readonly reason: string };

export const UploadStep = {
  Send: { kind: 'Send' } as UploadStep,
  Confirm: (gate: UploadGate): UploadStep => ({ kind: 'Confirm', gate }),
  Refuse: (reason: string): UploadStep => ({ kind: 'Refuse', reason }),
};

export const uploadStep = (gate: UploadGate | null, notReady: string | null = null): UploadStep => {
  if (notReady !== null) return UploadStep.Refuse(notReady);
  if (gate === null) return UploadStep.Refuse('The plan could not be checked against the vehicle.');
  if (gate.canSend) return UploadStep.Send;
  if (gate.canProceed) return UploadStep.Confirm(gate);
  return UploadStep.Refuse(gate.refusal.trim() === '' ? 'This plan cannot be uploaded.' : gate.refusal);
};

export type PlanSupport = { readonly fence: boolean; readonly rally: boolean; readonly fenceRefused: boolean; readonly rallyRefused: boolean };

export const PlanSupport = (fields: { fence: boolean; rally: boolean; fenceRefused?: boolean; rallyRefused?: boolean }): PlanSupport => ({ fenceRefused: false, rallyRefused: false, ...fields });

export const planSupport = (view: Json | null): PlanSupport => {
  const actions = optJSONObject(view, 'actions');
  return PlanSupport({
    fence: optBoolean(actions, 'addFence'),
    rally: optBoolean(actions, 'addRally'),
    fenceRefused: opt(view, 'fenceSupported') === false,
    rallyRefused: opt(view, 'rallySupported') === false,
  });
};

export const freshPlanView = (): Promise<Json | null> => MapBridge.read('view.plan');

export const UPLOADED = 'Uploaded';

export const uploadLabel = (offline: boolean, syncing: boolean, dirty: boolean, hasItems: boolean): string => {
  if (syncing) return 'Uploading…';
  if (!dirty && hasItems && !offline) return UPLOADED;
  return 'Upload';
};
