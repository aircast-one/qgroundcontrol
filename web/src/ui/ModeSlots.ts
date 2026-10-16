import { type Json, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';

export const MODE_SLOTS = 'view.modeSlots';

export type ModeSlot = { slot: number; mode: string; live: boolean };

export type ModeSlotsView = {
  channel: number;
  slots: readonly ModeSlot[];
  reason: string;
  activeSwitches: readonly string[];
  optionChannelsOn: readonly number[];
  channelMonitor: boolean;
  activeParams: ReadonlySet<string>;
};

export const ModeSlotsView = (fields: { channel: number; slots: readonly ModeSlot[]; reason: string; activeSwitches?: readonly string[]; optionChannelsOn?: readonly number[]; channelMonitor?: boolean; activeParams?: ReadonlySet<string> }): ModeSlotsView => ({
  activeSwitches: [],
  optionChannelsOn: [],
  channelMonitor: false,
  activeParams: new Set(),
  ...fields,
});

export const modeSlotsView = (view: Json | null): ModeSlotsView | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : ModeSlotsView({
        activeSwitches: jsonStrings(optJSONArray(view, 'activeSwitches')),
        optionChannelsOn: jsonObjects(optJSONArray(view, 'channelOptions'))
          .filter((it) => optBoolean(it, 'enabled'))
          .map((it) => optInt(it, 'channel')),
        channelMonitor: optBoolean(view, 'channelMonitor'),
        activeParams: new Set(jsonStrings(optJSONArray(view, 'activeParams'))),
        channel: optInt(view, 'channel'),
        slots: jsonObjects(optJSONArray(view, 'slots')).map((it) => ({ slot: optInt(it, 'slot'), mode: optText(it, 'mode'), live: optBoolean(it, 'live') })),
        reason: optText(view, 'reason'),
      });

export const liveSlotText = (view: ModeSlotsView | null): string | null => {
  if (view === null || view.slots.length === 0) return null;
  const live = view.slots.find((it) => it.live) ?? null;
  if (live === null) return view.reason.trim() === '' ? `The switch on channel ${view.channel} is not on any mode slot.` : view.reason;
  return `The switch on channel ${view.channel} is on slot ${live.slot}, ${live.mode}.`;
};

export const liveSwitchesText = (view: ModeSlotsView | null): string | null => {
  if (view === null) return null;
  if (view.activeSwitches.length > 0) return `Switches on: ${view.activeSwitches.join(', ')}`;
  if (view.optionChannelsOn.length > 0) return `Channel options on: ${view.optionChannelsOn.map((it) => `channel ${it}`).join(', ')}`;
  return null;
};
