import { useContext, useState } from 'react';
import { type Json, type JsonArray, arrayJSONObject, jsonObjects, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { stageFile, stagedFile, unstageAll } from '@/core/files';
import { useUpdatedRef } from '@/kotlin/Compose';
import { createDocument, getSharedPreferences, openDocument, openDocuments } from '@/kotlin/Platform';
import { useValue } from '@/kotlin/State';
import { delay } from '@/kotlin/Std';
import { PlanFocus, freshPlanView } from '@/map/VehicleSync';
import { LocalOpenPlan } from '@/ui/AppNavigation';
import {
  DEFAULT_KML_NAME,
  DEFAULT_PLAN_NAME,
  KML_EXTENSION,
  PLAN_EXTENSION,
  boundaryCacheName,
  importedNothing,
  mainBoundaryName,
  planIsSyncing,
  saveBlockedReason,
  withExtension,
} from '@/ui/PlanFileRules';

const PLAN_ROOT = 'plan';
const OPEN_FOLDER = 'plan-open';
const OPEN_FALLBACK = 'opened.plan';
const SAVE_CACHE = 'saving.plan';
const KML_CACHE = 'export.kml';
const MISSION_ROOT = 'plan.missionController';
const PLAN_VIEW = 'view.plan';
const MISSION_ITEMS_VIEW = 'view.missionItems';

export const PLAN_MIME = '*/*';

export const PLAN_OPEN_TYPES: readonly string[] = [PLAN_MIME];

export type PatternChoice = {
  options: () => readonly string[];
  pick: (name: string) => void;
  cancel: () => void;
};

export type PlanFileActions = {
  open: () => void;
  saveAs: () => void;
  save: () => void;
  exportKml: () => void;
  importBoundary: () => void;
  patternChoice: PatternChoice;
  newPlan: () => void;
  clearMission: () => void;
  download: () => void;
  documentName: () => string | null;
  opened: () => number;
};

export const planLoad = async (path: string): Promise<boolean | null> => {
  const loaded = await Qgc.invokeResult(`${PLAN_ROOT}.loadFromFile`, path);
  return typeof loaded === 'boolean' ? loaded : null;
};

export const loadFailureMessage = (loaded: boolean | null): string | null => (loaded === null ? 'The plan could not be loaded. The current plan is unchanged.' : null);

export const planSave = async (path: string): Promise<string | null> => {
  if ((await Qgc.invokeResult(`${PLAN_ROOT}.saveToFile`, path)) !== true) return null;
  return currentPlanPath(await Qgc.get(PLAN_VIEW));
};

const cachePath = (...names: string[]): string => `/${names.join('/')}`;

const leafOf = (path: string): string => path.slice(path.lastIndexOf('/') + 1);

const copyOut = (source: File, name: string): boolean => {
  try {
    createDocument(name, PLAN_MIME, source);
    return true;
  } catch {
    return false;
  }
};

export const patternNames = (view: Json | null): readonly string[] =>
  jsonObjects(optJSONArray(view, 'patterns'))
    .map((it) => optText(it, 'name'))
    .filter((it) => it.trim() !== '');

export const currentPlanPath = (view: Json | null): string | null => {
  const path = optText(view, 'filePath');
  return path.trim() === '' ? null : path;
};

export const visualItems = async (): Promise<JsonArray> => optJSONArray(await Qgc.get(MISSION_ITEMS_VIEW), 'items') ?? [];

const lastDistance = (items: JsonArray): number | null => {
  const distance = arrayJSONObject(items, items.length - 1)?.patternDistance;
  return typeof distance === 'number' ? distance : null;
};

const TEXT_MISSION_SUFFIXES: ReadonlySet<string> = new Set(['waypoints', 'txt']);

export const openedFileName = (shown: string | null): string => {
  const name = shown === null ? null : shown.slice(shown.lastIndexOf('/') + 1);
  return name !== null && name.trim() !== '' && name !== '.' && name !== '..' ? name : OPEN_FALLBACK;
};

export const isTextMission = (name: string): boolean => {
  const dot = name.lastIndexOf('.');
  return TEXT_MISSION_SUFFIXES.has(dot < 0 ? '' : name.slice(dot + 1));
};

const FILES_STORE = 'plan-files';
const LAST_DOCUMENT_KEY = 'lastDocument';

const rememberDocument = (document: File): void => getSharedPreferences(FILES_STORE).putString(LAST_DOCUMENT_KEY, document.name);

const DOWNLOAD_POLL_MS = 200;
const DOWNLOAD_POLLS = 300;

const downloadSettled = async (left: number = DOWNLOAD_POLLS): Promise<boolean> => {
  if (left === 0) return false;
  if (!planIsSyncing(await Qgc.get(PLAN_VIEW))) return true;
  await delay(DOWNLOAD_POLL_MS);
  return downloadSettled(left - 1);
};

const importMessage = async (documents: readonly File[], pattern: string): Promise<string | null> => {
  const labelled = documents.map((it) => [it, it.name] as const);
  const main = mainBoundaryName(labelled.map(([, name]) => name));
  const label = labelled.find(([, name]) => name === main)?.[1] ?? null;
  const staged = cachePath(boundaryCacheName(label));
  await unstageAll(cachePath('boundary.'));
  const copied = await labelled.reduce<Promise<boolean>>(async (ok, [document, name]) => (await ok) && stageFile(cachePath(boundaryCacheName(name)), document), Promise.resolve(true));
  if (!copied) return 'That file could not be read.';
  const before = (await visualItems()).length;
  await Qgc.invoke(`${MISSION_ROOT}.insertComplexMissionItemFromKMLOrSHP`, pattern, staged, before, true);
  const after = await visualItems();
  if (after.length <= before) return `${label ?? 'That file'} added nothing to the plan.`;
  if (!importedNothing(lastDistance(after))) return null;
  await Qgc.invoke(`${MISSION_ROOT}.removeVisualItem`, after.length - 1);
  return `${label ?? 'That file'} holds no area for a ${pattern}.`;
};

export const useRememberPlanFileActions = (onResult: (message: string) => void = () => undefined): PlanFileActions => {
  const result = useUpdatedRef(onResult);
  const openPlan = useContext(LocalOpenPlan);
  const documentName = useValue(openPlan.name);
  const [opened, setOpened] = useState(0);
  const [pendingImport, setPendingImport] = useState<readonly File[] | null>(null);
  const [patterns, setPatterns] = useState<readonly string[]>([]);

  const adopt = (document: File) => {
    openPlan.document.value = document;
    rememberDocument(document);
    openPlan.name.value = document.name;
  };

  const forget = () => {
    openPlan.document.value = null;
    openPlan.name.value = null;
  };

  const discard = (method: string, success: string, failure: string, then: () => Promise<void> = async () => undefined) =>
    void (async () => {
      const ok = await Qgc.invoke(`${PLAN_ROOT}.${method}`);
      if (ok) {
        forget();
        result.current(success);
        await then();
      } else {
        result.current(failure);
      }
    })();

  const importFrom = (documents: readonly File[], pattern: string) => void importMessage(documents, pattern).then((message) => result.current(message ?? 'Boundary imported.'));

  const guarded = (action: () => void) =>
    void (async () => {
      const view = await freshPlanView();
      const blocked = saveBlockedReason(view);
      if (blocked === null) {
        action();
      } else {
        PlanFocus.notReady(view);
        result.current(blocked);
      }
    })();

  const writeTo = (target: string) =>
    void (async () => {
      const named = withExtension(target, PLAN_EXTENSION);
      const staged = cachePath(SAVE_CACHE);
      await unstageAll(staged);
      const written = await planSave(staged);
      if (written === null) return result.current('The plan could not be saved.');
      const saved = await stagedFile(written);
      if (saved === null || !copyOut(saved, named)) return result.current('The plan was written but could not be copied out.');
      adopt(new File([saved], named));
      result.current('Plan saved.');
    })();

  const open = () =>
    void (async () => {
      const chosen = await openDocument([...PLAN_OPEN_TYPES]);
      if (chosen === null) return;
      const staged = cachePath(OPEN_FOLDER, openedFileName(chosen.name));
      await unstageAll(cachePath(OPEN_FOLDER));
      const [failure, loaded] = (await stageFile(staged, chosen)) ? await planLoad(staged).then((it) => [loadFailureMessage(it), it === true] as const) : (['That file could not be read.', false] as const);
      if (!loaded) {
        if (failure !== null) result.current(failure);
        return;
      }
      if (isTextMission(leafOf(staged))) forget();
      else adopt(chosen);
      setOpened((it) => it + 1);
      result.current('Plan opened.');
    })();

  const exportKmlTo = (target: string) =>
    void (async () => {
      const staged = cachePath(KML_CACHE);
      await unstageAll(staged);
      await Qgc.invoke(`${PLAN_ROOT}.saveToKml`, staged);
      const exported = await stagedFile(staged);
      result.current(
        exported === null || exported.size === 0
          ? 'The plan could not be exported.'
          : copyOut(exported, withExtension(target, KML_EXTENSION))
            ? 'KML exported.'
            : 'The KML was written but could not be copied out.',
      );
    })();

  const importBoundary = () =>
    void (async () => {
      const chosen = await openDocuments([...PLAN_OPEN_TYPES]);
      if (chosen.length === 0) return;
      const names = patternNames(await Qgc.get(PLAN_VIEW));
      if (names.length === 0) result.current('This vehicle offers no pattern to import into.');
      else if (names.length === 1) importFrom(chosen, names[0] ?? '');
      else {
        setPatterns(names);
        setPendingImport(chosen);
      }
    })();

  return {
    open,
    saveAs: () => guarded(() => writeTo(openPlan.name.value ?? DEFAULT_PLAN_NAME)),
    save: () =>
      guarded(() => {
        const target = openPlan.document.value;
        writeTo(target === null ? DEFAULT_PLAN_NAME : target.name);
      }),
    exportKml: () => guarded(() => exportKmlTo(DEFAULT_KML_NAME)),
    importBoundary,
    patternChoice: {
      options: () => (pendingImport !== null ? patterns : []),
      pick: (name) => {
        const documents = pendingImport;
        setPendingImport(null);
        if (documents !== null) importFrom(documents, name);
      },
      cancel: () => setPendingImport(null),
    },
    newPlan: () => discard('removeAll', 'New plan.', 'The plan could not be cleared.'),
    clearMission: () => discard('removeAllFromVehicle', 'Clear sent to the vehicle.', 'The clear could not be sent to the vehicle.'),
    download: () =>
      discard('loadFromVehicle', 'Loading the plan from the vehicle.', 'The plan could not be loaded from the vehicle.', async () => {
        if (await downloadSettled()) setOpened((it) => it + 1);
      }),
    documentName: () => documentName,
    opened: () => opened,
  };
};
