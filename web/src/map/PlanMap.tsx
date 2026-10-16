import { setWorkerUrl } from 'maplibre-gl';
import mapLibreWorker from 'maplibre-gl/dist/maplibre-gl-worker.mjs?url';
import { MapBridge } from './MapBridge';
import { currentMapType, installQgcTileSource, qgcRasterStyle } from './QgcTileSource';

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
