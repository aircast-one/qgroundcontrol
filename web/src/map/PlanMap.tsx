import type { ReactNode } from 'react';
import { setWorkerUrl } from 'maplibre-gl';
import mapLibreWorker from 'maplibre-gl/dist/maplibre-gl-worker.mjs?url';
import { useProduceState } from '@/kotlin/Compose';
import { Surface } from '@/m3';
import { md } from '@/theme/Colors';
import { MapBridge } from './MapBridge';
import { type PlanBar, PlanMapContent } from './PlanMapContent';
import { currentMapType, installQgcTileSource, qgcRasterStyle } from './QgcTileSource';
import type { TrackPoint } from './VehicleMap';

const PlanMapState = { tileSourceInstalled: false };

export const planMapStyle = async (): Promise<string> => {
  if (!PlanMapState.tileSourceInstalled) {
    MapBridge.start();
    setWorkerUrl(mapLibreWorker);
    installQgcTileSource();
    PlanMapState.tileSourceInstalled = true;
  }
  return qgcRasterStyle(await currentMapType());
};

export const PlanMapScreen = ({
  className,
  onCentre = null,
  itemPanel = null,
  header = null,
  routeSettings = null,
  fitKey = 0,
  onTemplates = null,
}: {
  className?: string;
  onCentre?: ((latitude: number, longitude: number) => void) | null;
  itemPanel?: ((index: number, at: TrackPoint | null, leg: string | null, remove: (() => void) | null) => ReactNode) | null;
  header?: ((bar: PlanBar) => ReactNode) | null;
  routeSettings?: (() => ReactNode) | null;
  fitKey?: number;
  onTemplates?: (() => void) | null;
}) => {
  const style = useProduceState<string | null>(null, [], async (set) => set(await planMapStyle()));
  return (
    <Surface className={className} color={md('surface')} contentColor={md('onSurface')}>
      {style === null ? null : (
        <PlanMapContent mapStyle={style} onCentre={onCentre} itemPanel={itemPanel} header={header} routeSettings={routeSettings} fitKey={fitKey} onTemplates={onTemplates} />
      )}
    </Surface>
  );
};
