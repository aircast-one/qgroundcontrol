import type { Map as MapLibreMap, SymbolLayerSpecification } from 'maplibre-gl';
import { azimuthBetween, pointAt } from './FenceBridge';
import type { MissionItem } from './PlanBridge';
import type { Survey } from './SurveyBridge';
import { metresBetween } from './TerrainProfile';
import { type TrackPoint, addBitmap, drawnImage, featureCollection, geoJsonSource, lineFeature, pointFeature, setGeoJson } from './VehicleMap';

const TRANSECT_ARROW_SOURCE = 'aircast-transect-arrows';
const TRANSECT_ARROW_LAYER = 'aircast-transect-arrow-layer';
const TRANSECT_STUB_SOURCE = 'aircast-transect-stubs';
const TRANSECT_STUB_LAYER = 'aircast-transect-stub-layer';
const TRANSECT_ARROW_IMAGE = 'aircast-transect-arrow';
const ARROW_BEARING = 'bearing';
const ARROW_SIZE_PX = 30;
const ENTRY_QUARTER = 1;
const EXIT_QUARTER = 3;

export type TransectArrow = { at: TrackPoint; bearing: number };

export type TransectMarks = { arrows: readonly TransectArrow[]; stubs: readonly (readonly TrackPoint[])[] };

export const arrowOn = (from: TrackPoint, to: TrackPoint, quarter: number): TransectArrow => {
  const at = pointAt(from, (metresBetween(from, to) / 4) * quarter, azimuthBetween(from, to));
  return { at, bearing: azimuthBetween(at, to) };
};

export const transectMarks = (points: readonly TrackPoint[], turnaround: boolean, current: boolean): TransectMarks => {
  const step = turnaround ? 4 : 2;
  const first = turnaround ? 1 : 0;
  const last = points.length - (turnaround ? 2 : 1);
  const count = Math.trunc(points.length / step);
  const segment = (from: number, quarter: number): TransectArrow | null => {
    const a = points[from];
    const b = points[from + 1];
    return a === undefined || b === undefined ? null : arrowOn(a, b, quarter);
  };
  const arrows =
    !current || points.length < 2
      ? []
      : [segment(first, ENTRY_QUARTER), count > 3 ? segment(first + step, ENTRY_QUARTER) : null, segment(last - 1, EXIT_QUARTER), count > 3 ? segment(last - step - 1, EXIT_QUARTER) : null].filter(
          (it): it is TransectArrow => it !== null,
        );
  const stubs = current || !turnaround || points.length < 2 ? [] : [points.slice(0, 2), points.slice(-2)];
  return { arrows, stubs };
};

const chevron = (): ImageData =>
  drawnImage(ARROW_SIZE_PX, ARROW_SIZE_PX, (canvas) => {
    const half = ARROW_SIZE_PX / 2;
    canvas.strokeStyle = '#FFFFFF';
    canvas.lineWidth = 3;
    canvas.beginPath();
    canvas.moveTo(0, half);
    canvas.lineTo(half, 0);
    canvas.lineTo(ARROW_SIZE_PX, half);
    canvas.stroke();
  });

const GIMBAL_WEDGE_SOURCE = 'aircast-gimbal-wedges';
const GIMBAL_WEDGE_LAYER = 'aircast-gimbal-wedge-layer';
const GIMBAL_WEDGE_IMAGE = 'aircast-mission-gimbal-wedge';
const GIMBAL_WEDGE_PX = 72;
const GIMBAL_WEDGE_SWEEP = 90;

export const gimbalWedges = (items: readonly MissionItem[]): readonly TransectArrow[] =>
  items.filter((it) => Number.isFinite(it.heading) && Number.isFinite(it.gimbalYaw)).map((it) => ({ at: { latitude: it.latitude, longitude: it.longitude }, bearing: it.heading + it.gimbalYaw }));

const wedge = (): ImageData =>
  drawnImage(GIMBAL_WEDGE_PX, GIMBAL_WEDGE_PX, (canvas) => {
    const half = GIMBAL_WEDGE_PX / 2;
    const radians = (degrees: number) => (degrees * Math.PI) / 180;
    canvas.fillStyle = `rgba(255, 255, 255, ${140 / 255})`;
    canvas.beginPath();
    canvas.moveTo(half, half);
    canvas.arc(half, half, half, radians(-90 - GIMBAL_WEDGE_SWEEP / 2), radians(-90 + GIMBAL_WEDGE_SWEEP / 2));
    canvas.closePath();
    canvas.fill();
  });

const arrowFeatures = (arrows: readonly TransectArrow[]) => featureCollection(arrows.map((it) => pointFeature(it.at.longitude, it.at.latitude, { [ARROW_BEARING]: it.bearing })));

export const renderGimbalWedges = (style: MapLibreMap, items: readonly MissionItem[]): void => setGeoJson(style, GIMBAL_WEDGE_SOURCE, arrowFeatures(gimbalWedges(items)));

const rotatedIcon = (id: string, source: string, image: string): SymbolLayerSpecification => ({
    id,
    type: 'symbol',
    source,
    layout: {
      'icon-image': image,
      'icon-rotate': ['get', ARROW_BEARING],
      'icon-rotation-alignment': 'map',
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    },
  });

export const installTransectMarks = (style: MapLibreMap): void => {
  if (style.getSource(GIMBAL_WEDGE_SOURCE) === undefined) {
    style.addSource(GIMBAL_WEDGE_SOURCE, geoJsonSource());
    addBitmap(style, GIMBAL_WEDGE_IMAGE, wedge());
    style.addLayer(rotatedIcon(GIMBAL_WEDGE_LAYER, GIMBAL_WEDGE_SOURCE, GIMBAL_WEDGE_IMAGE));
  }
  if (style.getSource(TRANSECT_ARROW_SOURCE) !== undefined) return;
  style.addSource(TRANSECT_STUB_SOURCE, geoJsonSource());
  style.addLayer({ id: TRANSECT_STUB_LAYER, type: 'line', source: TRANSECT_STUB_SOURCE, paint: { 'line-color': '#FFFFFF', 'line-width': 2 } });
  style.addSource(TRANSECT_ARROW_SOURCE, geoJsonSource());
  addBitmap(style, TRANSECT_ARROW_IMAGE, chevron());
  style.addLayer(rotatedIcon(TRANSECT_ARROW_LAYER, TRANSECT_ARROW_SOURCE, TRANSECT_ARROW_IMAGE));
};

export const renderTransectMarks = (style: MapLibreMap, surveys: readonly Survey[], selected: number | null, legArrows: readonly TransectArrow[] = []): void => {
  const marks = surveys.map((it) => transectMarks(it.transects, it.turnaround, it.index === selected));
  setGeoJson(style, TRANSECT_ARROW_SOURCE, arrowFeatures([...marks.flatMap((it) => it.arrows), ...legArrows]));
  setGeoJson(style, TRANSECT_STUB_SOURCE, featureCollection(marks.flatMap((it) => it.stubs).map((stub) => lineFeature(stub))));
};
