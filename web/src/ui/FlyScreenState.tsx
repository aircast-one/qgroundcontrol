import { createContext, useContext, useEffect } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';
import { type MutableState, mutableStateListOf, mutableStateMapOf, mutableStateOf, useValue } from '@/kotlin/State';
import { FlyMapEdits } from '@/map/FlyMapEdits';
import { type Insets, MapInsets, NO_INSETS, type Rect } from '@/ui/FlyShell';
import type { OverlayLayoutState } from '@/ui/OverlayLayout';
import { PipCorner } from '@/ui/VideoGestures';
import type { OpenGuidedValue } from './GuidedValueFlow';
import type { GuidedAction } from './VehicleUi';

export type VehicleScoped<T> = { readonly vehicle: number | null; readonly value: T };

export class FlyScreenState {
  readonly layout: OverlayLayoutState;
  readonly refusal = mutableStateOf<string | null>(null);
  readonly deckRequest = mutableStateOf<string | null>(null);
  readonly requestedSheet = mutableStateOf<string | null>(null);
  readonly offlineSheetOpen = mutableStateOf(false);
  readonly offeredSheets = mutableStateListOf<string>();
  readonly choosingReadings = mutableStateOf(false);
  readonly pendingMode = mutableStateOf<string | null>(null);
  readonly instrumentEdits = mutableStateOf(0);
  readonly guidedPanels = mutableStateOf(0);
  readonly confirming = mutableStateOf<VehicleScoped<GuidedAction> | null>(null);
  readonly choosingValue = mutableStateOf<VehicleScoped<OpenGuidedValue> | null>(null);
  readonly controlRequestDeadlines = mutableStateMapOf<number, number>();
  readonly mapEdits = new FlyMapEdits();
  readonly obstacles = mutableStateMapOf<string, Rect>();
  readonly mapInsets = mutableStateOf<MapInsets>(MapInsets({ top: 0, bottom: 0 }));
  readonly mapOrnamentEnd = mutableStateOf<number | null>(null);
  readonly cameraStickSlot = mutableStateOf<HTMLElement | null>(null);
  readonly widgetInsets = mutableStateOf<Insets>(NO_INSETS);
  readonly videoTucked = mutableStateOf(false);
  readonly pipCorner = mutableStateOf<PipCorner>(PipCorner.TopEnd);

  constructor(layout: OverlayLayoutState) {
    this.layout = layout;
  }

  get guidedPanelOpen(): boolean {
    return this.guidedPanels.value > 0;
  }
}

export const useVehicleScoped = <T,>(state: MutableState<VehicleScoped<T> | null>, vehicle: number | null): readonly [T | null, (next: T | null) => void] => {
  const held = useValue(state);
  const set = (next: T | null) => {
    state.value = next === null ? null : { vehicle, value: next };
  };
  return [held !== null && held.vehicle === vehicle ? held.value : null, set];
};

const unprovided = new Proxy({} as FlyScreenState, {
  get: () => {
    throw new Error('FlyScreenState is provided by AircastShell');
  },
});

export const LocalFlyScreenState = createContext<FlyScreenState>(unprovided);

export const TRAFFIC_SHEET = 'traffic';
export const MENU_SHEET = 'menu';
export const STATUS_ALL_SHEET = 'status-all';

export const REQUESTABLE_SHEETS: ReadonlySet<string> = new Set([MENU_SHEET, 'readings', 'camera', 'gimbal', 'status', STATUS_ALL_SHEET, 'modes', TRAFFIC_SHEET]);

export const OpenOnRequest = ({ name, open }: { name: string; open: () => void }) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const latestOpen = useUpdatedRef(open);
  const requested = useValue(flyScreen.requestedSheet);
  useEffect(() => {
    flyScreen.offeredSheets.add(name);
    return () => flyScreen.offeredSheets.remove(name);
  }, [name]);
  useEffect(() => {
    if (requested !== name) return;
    flyScreen.requestedSheet.value = null;
    latestOpen.current();
  }, [requested]);
  return null;
};
