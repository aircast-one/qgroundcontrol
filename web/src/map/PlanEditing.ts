import type { MutableState } from '@/kotlin/State';
import { delay } from '@/kotlin/Std';
import { PLAN_UNDO, invokeOk } from './BridgeCalls';
import { FenceBridge, rallyAfterRemove } from './FenceBridge';
import type { MapHit } from './MissionEditing';
import { HOME_ITEM, removeMissionItem } from './MissionKinds';
import { type MissionItem, PlanBridge } from './PlanBridge';
import { selectionAfterRemove } from './PlanItems';
import { usePlanAction } from './PlanShortcuts';
import { useRemembered } from './Remembered';

export const FAILURE_MESSAGE_MS = 2500;

export type BridgeOptions = { label?: string | null; done?: string | null; then?: (() => void) | null };

export type OnBridge = (work: () => Promise<boolean>, options?: BridgeOptions) => void;

export type OnRefusal = (work: () => Promise<string | null>, label?: string | null) => void;

export type Say = (message: string) => void;

export type PlanFeedback = {
  readonly busy: MutableState<string | null>;
  readonly edits: MutableState<number>;
  readonly say: Say;
  readonly onBridge: OnBridge;
  readonly onRefusal: OnRefusal;
};

export const usePlanFeedback = (): PlanFeedback => {
  const busy = useRemembered<string | null>(null);
  const edits = useRemembered(0);

  const say: Say = (message) => {
    busy.value = message;
    void (async () => {
      await delay(FAILURE_MESSAGE_MS);
      busy.value = null;
    })();
  };

  const onRefusal: OnRefusal = (work, label = null) => {
    busy.value = label;
    void (async () => {
      const refusal = await work();
      busy.value = refusal;
      if (refusal !== null) await delay(FAILURE_MESSAGE_MS);
      busy.value = null;
    })();
  };

  const onBridge: OnBridge = (work, { label = null, done = null, then = null } = {}) => {
    busy.value = label;
    void (async () => {
      const ok = await work();
      if (!ok) {
        busy.value = `${label ?? 'That'} did not work`;
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
        return;
      }
      if (then !== null) {
        edits.value += 1;
        then();
      }
      busy.value = done;
      if (done !== null) {
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
      }
    })();
  };

  return { busy, edits, say, onBridge, onRefusal };
};

export type PlanEditing = {
  readonly removals: number;
  readonly deleteSelected: () => void;
  readonly moveItem: (index: number, later: boolean) => void;
  readonly undo: () => void;
};

export const usePlanEditing = ({
  feedback,
  selected,
  allItems,
  rallyCount,
  canUndo,
}: {
  feedback: PlanFeedback;
  selected: MutableState<MapHit | null>;
  allItems: readonly MissionItem[];
  rallyCount: number;
  canUndo: boolean;
}): PlanEditing => {
  const { busy, edits, onBridge, onRefusal } = feedback;
  const removals = useRemembered(0);
  const removeItem = (item: MissionItem): void => {
    void (async () => {
      busy.value = `Removing #${item.sequence}`;
      const outcome = await removeMissionItem(item.index);
      if (outcome.ok) removals.value += 1;
      selected.value = outcome.ok ? selectionAfterRemove(item.index, allItems.length) : null;
      busy.value = outcome.ok ? null : outcome.reason;
      if (!outcome.ok) {
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
      }
    })();
  };
  const deleteSelected = (): void => {
    const hit = selected.value;
    const item = hit?.kind === 'Waypoint' ? allItems.find((it) => it.index === hit.index && it.index > HOME_ITEM) : undefined;
    if (item !== undefined) removeItem(item);
    else if (hit?.kind === 'Rally') {
      onBridge(() => FenceBridge.removeRallyPoint(hit.index), { label: 'Removing the rally point', then: () => (selected.value = rallyAfterRemove(hit.index, rallyCount)) });
    } else if (hit?.kind === 'Circle' || hit?.kind === 'CircleCentre' || hit?.kind === 'CircleRadius') {
      onBridge(() => FenceBridge.deleteCircle(hit.index), { label: 'Removing the circular fence', then: () => (selected.value = null) });
    }
  };
  const moveItem = (index: number, later: boolean): void => {
    onRefusal(async () => {
      const refusal = await PlanBridge.reorder(index, later);
      if (refusal === null) {
        edits.value += 1;
        selected.value = { kind: 'Waypoint', index: later ? index + 1 : index - 1 };
      }
      return refusal;
    });
  };
  const undo = (): void => onBridge(() => invokeOk(PLAN_UNDO));
  usePlanAction('deleteSelected', deleteSelected);
  usePlanAction('undo', () => {
    if (canUndo) undo();
  });
  return { removals: removals.value, deleteSelected, moveItem, undo };
};
