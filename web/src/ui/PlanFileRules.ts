import { type Json, type JsonArray, jsonObjects, optBoolean, optDouble, optJSONObject, optText } from '@/bridge/Json';
import { coerceIn } from '@/kotlin/Std';

export const DEFAULT_PLAN_NAME = 'mission.plan';
export const DEFAULT_KML_NAME = 'mission.kml';
export const DEFAULT_BOUNDARY_EXT = 'kml';
export const PLAN_EXTENSION = 'plan';
export const KML_EXTENSION = 'kml';

export const withExtension = (name: string, extension: string): string =>
  name.toLowerCase().endsWith(`.${extension}`.toLowerCase()) ? name : `${name}.${extension}`;

const substringAfterLast = (text: string, delimiter: string, missing: string): string => {
  const at = text.lastIndexOf(delimiter);
  return at < 0 ? missing : text.slice(at + delimiter.length);
};

export const boundaryCacheName = (displayName: string | null): string => {
  const ext = displayName === null ? null : substringAfterLast(displayName, '.', '').toLowerCase();
  return `boundary.${ext !== null && ext.trim() !== '' ? ext : DEFAULT_BOUNDARY_EXT}`;
};

export const mainBoundaryName = (names: readonly string[]): string | null =>
  names.find((it) => it.toLowerCase().endsWith('.shp')) ?? names.find((it) => it.toLowerCase().endsWith('.kml')) ?? names[0] ?? null;

export const importedNothing = (distance: number | null): boolean => distance === null || distance <= 0.0;

export type PlanActions = {
  open: boolean;
  save: boolean;
  exportKml: boolean;
  newPlan: boolean;
  clearFromVehicle: boolean;
  download: boolean;
};

export const planActions = (view: Json | null): PlanActions => {
  const actions = optJSONObject(view, 'actions');
  const allowed = (name: string) => actions !== null && optBoolean(actions, name);
  return {
    open: allowed('open'),
    save: allowed('save'),
    exportKml: allowed('exportKml'),
    newPlan: allowed('newPlan'),
    clearFromVehicle: allowed('clearMission'),
    download: allowed('download'),
  };
};

export type PlanHistory = { canUndo: boolean; canRedo: boolean };

export const planHistory = (view: Json | null): PlanHistory => ({
  canUndo: optBoolean(view, 'canUndo'),
  canRedo: optBoolean(view, 'canRedo'),
});

export const planIsDirty = (view: Json | null): boolean => optBoolean(view, 'dirty');

export const planIsSyncing = (view: Json | null): boolean => optText(optJSONObject(view, 'sync'), 'state') === 'busy';

export const planSyncProgress = (view: Json | null): number => {
  const sync = optJSONObject(view, 'sync');
  return sync === null ? 0 : coerceIn(optDouble(sync, 'progress', 0.0), 0, 1);
};

export const planContainsItems = (view: Json | null): boolean => optBoolean(view, 'containsItems');

const UNCHECKED_SAVE = 'The plan could not be checked for saving.';

export const saveBlockedReason = (view: Json | null): string | null => {
  const readiness = optJSONObject(view, 'readiness');
  if (readiness === null) return UNCHECKED_SAVE;
  if (optBoolean(readiness, 'ready')) return null;
  const reason = optText(readiness, 'reason');
  return reason.trim() === '' ? UNCHECKED_SAVE : reason;
};

export type PlanConfirm = { readonly name: 'Open' | 'NewPlan' | 'ClearMission' | 'Download'; readonly ordinal: number };

export const PlanConfirm = {
  Open: { name: 'Open', ordinal: 0 },
  NewPlan: { name: 'NewPlan', ordinal: 1 },
  ClearMission: { name: 'ClearMission', ordinal: 2 },
  Download: { name: 'Download', ordinal: 3 },
} as const satisfies Record<string, PlanConfirm>;

export const PlanConfirmEntries: readonly PlanConfirm[] = [PlanConfirm.Open, PlanConfirm.NewPlan, PlanConfirm.ClearMission, PlanConfirm.Download];

export type ConfirmCopy = { title: string; body: string; confirm: string; destructive: boolean };

export const ConfirmCopy = (fields: { title: string; body: string; confirm: string; destructive?: boolean }): ConfirmCopy => ({ destructive: false, ...fields });

export const confirmCopy = (kind: PlanConfirm): ConfirmCopy => {
  switch (kind.name) {
    case 'Open':
      return ConfirmCopy({
        title: 'Plan overwrite',
        body: 'You have unsaved/unsent changes. Loading from a file will lose these changes. Are you sure you want to load from a file?',
        confirm: 'Load from file',
      });
    case 'NewPlan':
      return ConfirmCopy({
        title: 'Create Plan',
        body: 'Are you sure you want to remove current plan and create a new plan?',
        confirm: 'Create plan',
      });
    case 'Download':
      return ConfirmCopy({
        title: 'Plan overwrite',
        body: 'You have unsaved/unsent changes. Loading from the Vehicle will lose these changes. Are you sure you want to load from the Vehicle?',
        confirm: 'Load from vehicle',
      });
    case 'ClearMission':
      return ConfirmCopy({
        title: 'Clear the mission from the vehicle?',
        body: 'This removes the mission from the aircraft as well as from this plan. It cannot be undone.',
        confirm: 'Clear mission',
        destructive: true,
      });
  }
};

export const planTitle = (documentName: string | null): string => {
  if (documentName === null || documentName.trim() === '') return 'Untitled Plan';
  const dot = documentName.lastIndexOf('.');
  return dot > 0 ? documentName.slice(0, dot) : documentName;
};

export const planStatusText = (view: Json | null): string => optText(view, 'status');

export const DRAWN_KINDS: ReadonlySet<string> = new Set(['settings', 'takeoff', 'land', 'waypoint', 'command', 'altitude', 'roi', 'survey', 'corridor', 'structure']);

export const undrawnItemNames = (items: JsonArray): readonly string[] => [
  ...new Set(
    jsonObjects(items)
      .filter((it) => !DRAWN_KINDS.has(optText(it, 'kind')))
      .map((it) => optText(it, 'name'))
      .filter((it) => it.trim() !== ''),
  ),
];

export const undrawnItemsWarning = (names: readonly string[]): string | null =>
  names.length === 0 ? null : `The map cannot draw ${names.join(', ')}. Those items are still in the plan and will still be flown.`;
