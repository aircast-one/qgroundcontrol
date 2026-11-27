import { PlanCommands } from '@/bridge/BridgeCommands';
import type { MutableState } from '@/kotlin/State';
import { delay, replaceFirstCharLower } from '@/kotlin/Std';
import { PLAN_UNDO, invokeOk } from './BridgeCalls';
import { FenceBridge, rallyAfterRemove } from './FenceBridge';
import type { MapHit } from './MissionEditing';
import { HOME_ITEM, removeMissionItem } from './MissionKinds';
import { KIND_LAND, KIND_TAKEOFF, MAV_CMD_NAV_RETURN_TO_LAUNCH, type MissionItem, PlanBridge } from './PlanBridge';
import { itemTitle, selectionAfterRemove } from './PlanItems';
import { useRemembered } from './Remembered';

export const FAILURE_MESSAGE_MS = 2500;

export type BridgeOptions = { label?: string | null; done?: string | null; edited?: boolean; then?: (() => void | Promise<void>) | null };

export type OnBridge = (work: () => Promise<boolean>, options?: BridgeOptions) => Promise<boolean>;

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

  const showFor = async (message: string | null): Promise<void> => {
    busy.value = message;
    if (message === null) return;
    await delay(FAILURE_MESSAGE_MS);
    busy.value = null;
  };

  const say: Say = (message) => void showFor(message);

  const onRefusal: OnRefusal = (work, label = null) => {
    busy.value = label;
    void work().then(showFor);
  };

  const onBridge: OnBridge = (work, { label = null, done = null, edited = false, then = null } = {}) => {
    busy.value = label;
    const settled = (async () => {
      const ok = await work();
      if (ok && edited) edits.value += 1;
      if (ok) await then?.();
      return ok;
    })();
    void settled.then((ok) => showFor(ok ? done : `${label ?? 'That'} did not work`));
    return settled;
  };

  return { busy, edits, say, onBridge, onRefusal };
};

export type Deletion = { readonly label: string; readonly run: () => void };

export type Removals = { readonly item: (item: MissionItem) => void; readonly rally: (index: number) => void; readonly circle: (index: number) => void };

export const deletableSelection = (selected: MapHit | null, items: readonly MissionItem[], remove: Removals): Deletion | null => {
  switch (selected?.kind) {
    case 'Waypoint': {
      const item = items.find((it) => it.index === selected.index && it.index !== HOME_ITEM);
      return item === undefined ? null : { label: `Delete ${replaceFirstCharLower(itemTitle(item))}`, run: () => remove.item(item) };
    }
    case 'Rally':
      return { label: 'Delete rally point', run: () => remove.rally(selected.index) };
    case 'Circle':
    case 'CircleCentre':
    case 'CircleRadius':
      return { label: 'Delete circular fence', run: () => remove.circle(selected.index) };
    default:
      return null;
  }
};

export type ItemMoves = { readonly earlier: boolean; readonly later: boolean };

const STAYS: ItemMoves = { earlier: false, later: false };

export const pinsOrder = (item: MissionItem): boolean => item.kind === KIND_TAKEOFF || item.kind === KIND_LAND || item.commandId === MAV_CMD_NAV_RETURN_TO_LAUNCH;

export const itemMoves = (items: readonly MissionItem[], index: number): ItemMoves => {
  const movable = (at: number): boolean => {
    const item = items.find((it) => it.index === at);
    return item !== undefined && item.index !== HOME_ITEM && !pinsOrder(item);
  };
  return movable(index) ? { earlier: movable(index - 1), later: movable(index + 1) } : STAYS;
};

export type PlanEditing = {
  readonly removals: number;
  readonly removing: boolean;
  readonly deletion: Deletion | null;
  readonly moveItem: (index: number, later: boolean) => void;
  readonly undo: () => void;
  readonly redo: () => void;
};

export const usePlanEditing = ({
  feedback,
  selected,
  allItems,
  rallyCount,
}: {
  feedback: PlanFeedback;
  selected: MutableState<MapHit | null>;
  allItems: readonly MissionItem[];
  rallyCount: number;
}): PlanEditing => {
  const { busy, edits, onBridge, onRefusal } = feedback;
  const removals = useRemembered(0);
  const removing = useRemembered(false);

  const removal = (work: () => Promise<unknown>): void => {
    if (removing.value) return;
    removing.value = true;
    void work().finally(() => {
      removing.value = false;
    });
  };

  const removeItem = async (item: MissionItem): Promise<void> => {
    busy.value = `Removing ${replaceFirstCharLower(itemTitle(item))}`;
    const outcome = await removeMissionItem(item.index);
    if (outcome.ok) removals.value += 1;
    selected.value = outcome.ok ? selectionAfterRemove(item.index, allItems.length) : null;
    busy.value = outcome.ok ? null : outcome.reason;
    if (!outcome.ok) {
      await delay(FAILURE_MESSAGE_MS);
      busy.value = null;
    }
  };

  const deletion = deletableSelection(selected.value, allItems, {
    item: (item) => removal(() => removeItem(item)),
    rally: (index) =>
      removal(() =>
        onBridge(() => FenceBridge.removeRallyPoint(index), {
          label: 'Removing the rally point',
          edited: true,
          then: () => {
            selected.value = rallyAfterRemove(index, rallyCount);
          },
        }),
      ),
    circle: (index) =>
      removal(() =>
        onBridge(() => FenceBridge.deleteCircle(index), {
          label: 'Removing the circular fence',
          edited: true,
          then: () => {
            selected.value = null;
          },
        }),
      ),
  });

  const moveItem = (index: number, later: boolean): void => {
    onRefusal(async () => {
      const moved = await PlanBridge.reorder(index, later);
      if (moved.refusal === null) {
        edits.value += 1;
        selected.value = { kind: 'Waypoint', index: moved.index };
      }
      return moved.refusal;
    });
  };

  return {
    removals: removals.value,
    removing: removing.value,
    deletion,
    moveItem,
    undo: () => void onBridge(() => invokeOk(PLAN_UNDO)),
    redo: () => void onBridge(() => PlanCommands.redo()),
  };
};
