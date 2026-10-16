import type { FeatureCollection } from 'geojson';
import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, jsonObjects, optBoolean, optDouble, optJSONArray } from '@/bridge/Json';
import { MapBridge } from './MapBridge';
import { addBitmap, drawnImage, featureCollection, geoJsonSource, isPlottable, pointFeature, setGeoJson } from './VehicleMap';

export const GIMBAL_AZIMUTH_VIEW = 'view.gimbalAzimuth';
const GIMBAL_SOURCE = 'aircast-gimbal-azimuth';
const GIMBAL_LAYER = 'aircast-gimbal-azimuth-layer';
const GIMBAL_IMAGE = 'aircast-gimbal-wedge';
const YAW_PROPERTY = 'yaw';
const OPACITY_PROPERTY = 'opacity';
const ACTIVE_OPACITY = 1.0;
const OTHER_OPACITY = 0.4;
const WEDGE_SIZE_PX = 48;

export type GimbalAzimuth = { readonly yaw: number; readonly active: boolean };

export const gimbalAzimuths = (view: Json | null): readonly GimbalAzimuth[] =>
  jsonObjects(optJSONArray(view, 'gimbals')).map((it) => ({ yaw: optDouble(it, 'yaw'), active: optBoolean(it, 'active') }));

export const GimbalBridge = {
  read: async (): Promise<readonly GimbalAzimuth[]> => gimbalAzimuths(await MapBridge.read(GIMBAL_AZIMUTH_VIEW)),
};

export const gimbalFeatures = (latitude: number, longitude: number, gimbals: readonly GimbalAzimuth[]): FeatureCollection =>
  featureCollection(
    gimbals
      .filter((it) => isPlottable(latitude, longitude) && !Number.isNaN(it.yaw))
      .map((gimbal) => pointFeature(longitude, latitude, { [YAW_PROPERTY]: gimbal.yaw, [OPACITY_PROPERTY]: gimbal.active ? ACTIVE_OPACITY : OTHER_OPACITY })),
  );

const wedge = (): ImageData =>
  drawnImage(WEDGE_SIZE_PX, WEDGE_SIZE_PX, (canvas) => {
    const size = WEDGE_SIZE_PX;
    const shade = canvas.createLinearGradient(0, 0, 0, size);
    shade.addColorStop(0, 'rgba(255, 255, 255, 0)');
    shade.addColorStop(1, `rgba(255, 255, 255, ${128 / 255})`);
    canvas.fillStyle = shade;
    canvas.beginPath();
    canvas.moveTo(size / 2, size);
    canvas.lineTo(size * 0.0, 0);
    canvas.lineTo(size / 2, size * 0.25);
    canvas.lineTo(size, 0);
    canvas.closePath();
    canvas.fill();
  });

export const installGimbalLayer = (style: MapLibreMap): void => {
  if (style.getSource(GIMBAL_SOURCE) !== undefined) return;
  style.addSource(GIMBAL_SOURCE, geoJsonSource());
  addBitmap(style, GIMBAL_IMAGE, wedge());
  style.addLayer({
    id: GIMBAL_LAYER,
    type: 'symbol',
    source: GIMBAL_SOURCE,
    layout: {
      'icon-image': GIMBAL_IMAGE,
      'icon-anchor': 'bottom',
      'icon-rotate': ['get', YAW_PROPERTY],
      'icon-rotation-alignment': 'map',
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    },
    paint: { 'icon-opacity': ['get', OPACITY_PROPERTY] },
  });
};

export const renderGimbals = (style: MapLibreMap, latitude: number, longitude: number, gimbals: readonly GimbalAzimuth[]): void =>
  setGeoJson(style, GIMBAL_SOURCE, gimbalFeatures(latitude, longitude, gimbals));
