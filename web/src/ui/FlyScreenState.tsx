import { createContext, useContext, useEffect } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';
import { mutableStateListOf, mutableStateMapOf, mutableStateOf, useValue } from '@/kotlin/State';
import { FlyMapEdits } from '@/map/FlyMapEdits';
import { MapInsets, type Rect } from '@/ui/FlyShell';
import type { OverlayLayoutState } from '@/ui/OverlayLayout';
import { PipCorner } from '@/ui/VideoGestures';

export class FlyScreenState {
  readonly layout: OverlayLayoutState;
  readonly refusal = mutableStateOf<string | null>(null);
  readonly deckRequest = mutableStateOf<string | null>(null);
  readonly requestedSheet = mutableStateOf<string | null>(null);
  readonly offeredSheets = mutableStateListOf<string>();
  readonly choosingReadings = mutableStateOf(false);
  readonly pendingMode = mutableStateOf<string | null>(null);
  readonly instrumentEdits = mutableStateOf(0);
  readonly guidedPanels = mutableStateOf(0);
  readonly controlRequestDeadlines = mutableStateMapOf<number, number>();
  readonly mapEdits = new FlyMapEdits();
  readonly obstacles = mutableStateMapOf<string, Rect>();
  readonly mapInsets = mutableStateOf<MapInsets>(MapInsets({ top: 0, bottom: 0 }));
  readonly mapOrnamentEnd = mutableStateOf<number | null>(null);
  readonly videoTucked = mutableStateOf(false);
  readonly pipCorner = mutableStateOf<PipCorner>(PipCorner.TopEnd);

  constructor(layout: OverlayLayoutState) {
    this.layout = layout;
  }

  get guidedPanelOpen(): boolean {
    return this.guidedPanels.value > 0;
  }
}

const unprovided = new Proxy({} as FlyScreenState, {
  get: () => {
    throw new Error('FlyScreenState is provided by AircastShell');
  },
});

export const LocalFlyScreenState = createContext<FlyScreenState>(unprovided);

export const TRAFFIC_SHEET = 'traffic';

export const REQUESTABLE_SHEETS: ReadonlySet<string> = new Set(['more', 'readings', 'camera', 'gimbal', 'status', 'status-all', 'modes', TRAFFIC_SHEET]);

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
