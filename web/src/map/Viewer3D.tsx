import type { Feature, FeatureCollection, Position } from 'geojson';
import { Map as MapLibreMap } from 'maplibre-gl';
import { useEffect, useMemo, useRef, useState } from 'react';
import { type Json, type JsonArray, isJson, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { useUpdatedRef } from '@/kotlin/Compose';
import { coerceAtLeast, coerceIn, same } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { Color, md } from '@/theme/Colors';
import { useMapPath } from './MapBridge';
import { planMapStyle } from './PlanMap';
import { QgcTileInterceptor } from './QgcTileSource';
import { NO_FADES, featureCollection, geoJsonSource, isPlottable, setGeoJson } from './VehicleMap';
import { VEHICLES_VIEW, vehicleChoices } from './VehiclePicker';

export const VIEWER3D_VIEW = 'view.viewer3d';
export const VIEWER3D_PATH = 'view.viewer3dPath';
export const VIEWER3D_VEHICLE = 'view.viewer3dVehicle';
const V3D_FLOATING_SOURCE = 'viewer3d-floating';
const V3D_FLOATING_LAYER = 'viewer3d-floating-layer';
const V3D_VEHICLE_SOURCE = 'viewer3d-vehicle';
const V3D_VEHICLE_LAYER = 'viewer3d-vehicle-layer';
const RIBBON_MAX_PIECES = 64;
const METRES_PER_DEGREE = 111_320.0;
const RIBBON_WIDTH = 1.5;
const RIBBON_THICKNESS = 1.0;
const RIBBON_STEP = 8.0;
const MARKER_SIZE = 3.0;
const LABEL_LIFT = 3.0;
const ARM_LENGTH = 3.0;
const ARM_WIDTH = 0.5;
const ROTOR_SIZE = 1.6;
const HUB_SIZE = 1.4;
const FRONT_ARM_COLOUR = '#E53935';
const REAR_ARM_COLOUR = '#ECEFF1';
const ROTOR_COLOUR = '#37474F';
const HUB_COLOUR = '#90A4AE';
const SELECTED_MARKER_COLOUR = '#FFFF00';
const MARKER_PICK_DP = 24;
const RADIANS = Math.PI / 180;

export type Point3D = { readonly lon: number; readonly lat: number; readonly alt: number };

export type LonLat = readonly [number, number];

export type Slab = { readonly corners: readonly LonLat[]; readonly base: number; readonly top: number; readonly colour: string };

const numberAt = (array: JsonArray, index: number): number => {
  const value = array[index];
  return typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
};

const point3d = (array: JsonArray | null): Point3D | null => (array !== null && array.length === 3 ? { lon: numberAt(array, 0), lat: numberAt(array, 1), alt: numberAt(array, 2) } : null);

const box = (at: Point3D, size: number, colour: string, height: number = size): Slab => {
  const dLat = size / 2 / METRES_PER_DEGREE;
  const dLon = dLat / Math.cos(at.lat * RADIANS);
  return {
    corners: [
      [at.lon - dLon, at.lat - dLat],
      [at.lon + dLon, at.lat - dLat],
      [at.lon + dLon, at.lat + dLat],
      [at.lon - dLon, at.lat + dLat],
    ],
    base: at.alt - height / 2,
    top: at.alt + height / 2,
    colour,
  };
};

export const ribbon = (from: Point3D, to: Point3D, colour: string, width: number = RIBBON_WIDTH, thickness: number = RIBBON_THICKNESS): readonly Slab[] => {
  const scale = Math.cos(((from.lat + to.lat) / 2) * RADIANS);
  const dx = (to.lon - from.lon) * METRES_PER_DEGREE * scale;
  const dy = (to.lat - from.lat) * METRES_PER_DEGREE;
  const length = Math.hypot(dx, dy);
  if (length < width / 2) return [{ ...box(from, width, colour, 0.0), base: Math.min(from.alt, to.alt) - thickness / 2, top: Math.max(from.alt, to.alt) + thickness / 2 }];
  const pieces = coerceIn(Math.ceil(length / RIBBON_STEP), 1, RIBBON_MAX_PIECES);
  const climb = Math.abs(to.alt - from.alt) / pieces;
  const nx = (-dy / length) * (width / 2);
  const ny = (dx / length) * (width / 2);
  const toLonLat = (x: number, y: number): LonLat => [from.lon + x / (METRES_PER_DEGREE * scale), from.lat + y / METRES_PER_DEGREE];
  return Array.from({ length: pieces }, (_, piece) => {
    const t0 = piece / pieces;
    const t1 = (piece + 1) / pieces;
    const alt = from.alt + ((to.alt - from.alt) * (t0 + t1)) / 2;
    const half = Math.max(thickness, climb) / 2;
    const [x0, y0, x1, y1] = [dx * t0, dy * t0, dx * t1, dy * t1];
    return { corners: [toLonLat(x0 + nx, y0 + ny), toLonLat(x1 + nx, y1 + ny), toLonLat(x1 - nx, y1 - ny), toLonLat(x0 - nx, y0 - ny)], base: alt - half, top: alt + half, colour };
  });
};

const offset = (at: Point3D, east: number, north: number): Point3D => ({ ...at, lon: at.lon + east / (METRES_PER_DEGREE * Math.cos(at.lat * RADIANS)), lat: at.lat + north / METRES_PER_DEGREE });

export const quadFrame = (at: Point3D, heading: number): readonly Slab[] => [
  ...[45.0, -45.0, 135.0, -135.0].flatMap((arm) => {
    const bearing = (heading + arm) * RADIANS;
    const motor = offset(at, Math.sin(bearing) * ARM_LENGTH, Math.cos(bearing) * ARM_LENGTH);
    const colour = Math.abs(arm) < 90 ? FRONT_ARM_COLOUR : REAR_ARM_COLOUR;
    return [...ribbon(at, motor, colour, ARM_WIDTH, ARM_WIDTH), box({ ...motor, alt: at.alt + ARM_WIDTH }, ROTOR_SIZE, ROTOR_COLOUR, ARM_WIDTH / 2)];
  }),
  box(at, HUB_SIZE, HUB_COLOUR, HUB_SIZE / 2),
];

export type PathMarker = { readonly mission: number; readonly at: Point3D | null };

const listedObjects = (array: JsonArray | null): readonly (Json | null)[] => (array ?? []).map((it) => (isJson(it) ? it : null));

export const pathMarkers = (view: Json | null): readonly PathMarker[] =>
  listedObjects(optJSONArray(view, 'markers')).map((it) => ({ mission: it === null ? 0 : optInt(it, 'mission', 0), at: point3d(optJSONArray(it, 'at')) }));

export const selectedAfterTap = (selected: ReadonlySet<number>, markers: readonly PathMarker[], hit: number | null): ReadonlySet<number> =>
  hit === null ? new Set() : new Set([...[...selected].filter((it) => markers[it]?.mission !== markers[hit]?.mission), hit]);

const missionMarkers = (markers: readonly PathMarker[], mission: number): readonly (readonly [number, PathMarker])[] =>
  markers.flatMap((marker, index) => (marker.mission === mission ? [[index, marker] as const] : []));

export const selectionKept = (selected: ReadonlySet<number>, before: readonly PathMarker[], after: readonly PathMarker[]): ReadonlySet<number> =>
  new Set(
    [...selected].flatMap((index) => {
      const mission = before[index]?.mission;
      if (mission === undefined) return [];
      const was = missionMarkers(before, mission);
      const now = missionMarkers(after, mission);
      if (!same(now.map(([, marker]) => marker), was.map(([, marker]) => marker))) return [];
      const kept = now[was.findIndex(([at]) => at === index)];
      return kept === undefined ? [] : [kept[0]];
    }),
  );

export const pickedMarker = (tap: readonly [number, number], onScreen: readonly (readonly [number, number] | null)[], radius: number): number | null =>
  onScreen
    .flatMap((at, index) => (at === null ? [] : [[index, Math.hypot(at[0] - tap[0], at[1] - tap[1])] as const]))
    .filter(([, apart]) => apart <= radius)
    .reduce<readonly [number, number] | null>((best, candidate) => (best === null || candidate[1] < best[1] ? candidate : best), null)?.[0] ?? null;

export const pathSlabs = (view: Json | null, selected: ReadonlySet<number> = new Set()): readonly Slab[] => {
  const ribbons = jsonObjects(optJSONArray(view, 'segments')).flatMap((segment) => {
    const from = point3d(optJSONArray(segment, 'from'));
    const to = point3d(optJSONArray(segment, 'to'));
    return from === null || to === null ? [] : ribbon(from, to, optText(segment, 'colour'));
  });
  const boxes = listedObjects(optJSONArray(view, 'markers')).flatMap((marker, index) => {
    const at = marker === null ? null : point3d(optJSONArray(marker, 'at'));
    return at === null ? [] : [box(at, MARKER_SIZE, selected.has(index) ? SELECTED_MARKER_COLOUR : optText(marker, 'colour'))];
  });
  return [...ribbons, ...boxes];
};

export type Label3D = { readonly at: Point3D; readonly text: string };

export const pathLabels = (view: Json | null): readonly Label3D[] =>
  jsonObjects(optJSONArray(view, 'markers')).flatMap((marker) => {
    const at = point3d(optJSONArray(marker, 'at'));
    const text = optText(marker, 'label');
    return at === null || text.trim() === '' ? [] : [{ at: { ...at, alt: at.alt + LABEL_LIFT }, text }];
  });

export const lifted = (ground: readonly [number, number], pixelsPerMetre: number, tilt: number, height: number): readonly [number, number] => [
  ground[0],
  ground[1] - coerceAtLeast(height, 0.0) * pixelsPerMetre * Math.sin(tilt * RADIANS),
];

const onScreen = (map: MapLibreMap, at: Point3D): readonly [number, number] => {
  const across = (map.getBearing() + 90) * RADIANS;
  const ground = map.project([at.lon, at.lat]);
  const side = offset(at, Math.sin(across), Math.cos(across));
  const beside = map.project([side.lon, side.lat]);
  return lifted([ground.x, ground.y], Math.hypot(beside.x - ground.x, beside.y - ground.y), map.getPitch(), at.alt);
};

export const vehicleSlabs = (view: Json | null): readonly Slab[] =>
  jsonObjects(optJSONArray(view, 'vehicles')).flatMap((vehicle) => {
    const at = point3d(optJSONArray(vehicle, 'at'));
    return at === null ? [] : quadFrame(at, optDouble(vehicle, 'heading', 0.0));
  });

const ring = (corners: readonly LonLat[]): Position[] => [...corners, ...corners.slice(0, 1)].map(([lon, lat]) => [lon, lat]);

export const slabFeatures = (slabs: readonly Slab[]): FeatureCollection =>
  featureCollection(
    slabs.map((slab): Feature => ({
      type: 'Feature',
      geometry: { type: 'Polygon', coordinates: [ring(slab.corners)] },
      properties: { base: coerceAtLeast(slab.base, 0.0), top: coerceAtLeast(slab.top, 0.1), colour: slab.colour },
    })),
  );

const V3D_BUILDING_SOURCE = 'viewer3d-buildings';
const V3D_BUILDING_LAYER = 'viewer3d-buildings-layer';
const SCENE_PITCH = 60.0;
const SCENE_ZOOM = 16.0;

export type Building3D = { readonly outer: readonly (readonly LonLat[])[]; readonly inner: readonly (readonly LonLat[])[]; readonly height: number };

export type Scene3D = { readonly available: boolean; readonly reason: string; readonly buildings: readonly Building3D[]; readonly centre: LonLat | null };

const lonLats = (array: JsonArray | null): readonly LonLat[] => (array ?? []).flatMap((it) => (Array.isArray(it) ? [[numberAt(it, 0), numberAt(it, 1)] as const] : []));

const rings = (array: JsonArray | null): readonly (readonly LonLat[])[] =>
  (array ?? []).flatMap((it) => (Array.isArray(it) ? [lonLats(it)] : [])).filter((it) => it.length > 2);

export const signedArea = (corners: readonly LonLat[]): number =>
  corners.reduce((sum, a, index) => {
    const b = corners[(index + 1) % corners.length] ?? a;
    return sum + (a[0] * b[1] - b[0] * a[1]);
  }, 0) / 2.0;

export const wound = (corners: readonly LonLat[], counterClockwise: boolean): readonly LonLat[] => {
  const first = corners[0];
  const last = corners.at(-1);
  const closed = first === undefined || (last !== undefined && same(first, last)) ? corners : [...corners, first];
  return signedArea(closed) > 0 === counterClockwise ? closed : closed.toReversed();
};

export const scene3d = (view: Json | null): Scene3D => {
  const bounds = optJSONObject(view, 'bounds');
  return {
    available: optBoolean(view, 'available'),
    reason: optText(view, 'reason'),
    buildings: jsonObjects(optJSONArray(view, 'buildings'))
      .map((it) => ({ outer: rings(optJSONArray(it, 'outer')), inner: rings(optJSONArray(it, 'inner')), height: optDouble(it, 'height', 0.0) }))
      .filter((it) => it.outer.length > 0 && it.height > 0.0),
    centre: bounds === null ? null : [(optDouble(bounds, 'west') + optDouble(bounds, 'east')) / 2, (optDouble(bounds, 'south') + optDouble(bounds, 'north')) / 2],
  };
};

const positions = (corners: readonly LonLat[]): Position[] => corners.map(([lon, lat]) => [lon, lat]);

export const buildingFeatures = (buildings: readonly Building3D[]): FeatureCollection =>
  featureCollection(
    buildings.map((building): Feature => {
      const outers = building.outer.map((it) => positions(wound(it, true)));
      const holes = building.inner.map((it) => positions(wound(it, false)));
      return {
        type: 'Feature',
        geometry: outers.length === 1 ? { type: 'Polygon', coordinates: [...outers, ...holes] } : { type: 'MultiPolygon', coordinates: outers.map((it) => [it]) },
        properties: { height: building.height },
      };
    }),
  );

const addSlabLayer = (style: MapLibreMap, source: string, layer: string): void => {
  style.addSource(source, geoJsonSource());
  style.addLayer({
    id: layer,
    type: 'fill-extrusion',
    source,
    paint: { 'fill-extrusion-color': ['get', 'colour'], 'fill-extrusion-base': ['get', 'base'], 'fill-extrusion-height': ['get', 'top'] },
  });
};

const installScene = (style: MapLibreMap): void => {
  style.addSource(V3D_BUILDING_SOURCE, geoJsonSource());
  style.addLayer({
    id: V3D_BUILDING_LAYER,
    type: 'fill-extrusion',
    source: V3D_BUILDING_SOURCE,
    paint: { 'fill-extrusion-color': '#B0BEC5', 'fill-extrusion-height': ['get', 'height'], 'fill-extrusion-base': 0, 'fill-extrusion-opacity': 0.85 },
  });
  addSlabLayer(style, V3D_FLOATING_SOURCE, V3D_FLOATING_LAYER);
  addSlabLayer(style, V3D_VEHICLE_SOURCE, V3D_VEHICLE_LAYER);
};

type StyleSpec = Exclude<Parameters<MapLibreMap['setStyle']>[0], string | null>;

export const Viewer3DPane = ({ className }: { className?: string }) => {
  const viewJson = useMapPath(VIEWER3D_VIEW);
  const scene = useMemo(() => scene3d(viewJson), [viewJson]);
  const vehiclesJson = useMapPath(VEHICLES_VIEW);
  const vehicle = useMemo(() => vehicleChoices(vehiclesJson).choices.find((it) => it.active && isPlottable(it.latitude, it.longitude)) ?? null, [vehiclesJson]);
  const pathJson = useMapPath(VIEWER3D_PATH);
  const markerList = useMemo(() => pathMarkers(pathJson), [pathJson]);
  const [selectedMarkers, setSelectedMarkers] = useState<ReadonlySet<number>>(() => new Set());
  const selectedAmong = useRef(markerList);
  useEffect(() => {
    const before = selectedAmong.current;
    setSelectedMarkers((selected) => selectionKept(selected, before, markerList));
    selectedAmong.current = markerList;
  }, [markerList]);
  const slabs = useMemo(() => pathSlabs(pathJson, selectedMarkers), [pathJson, selectedMarkers]);
  const markers = useUpdatedRef(markerList);
  const labels = useMemo(() => pathLabels(pathJson), [pathJson]);
  const [cameraMoves, setCameraMoves] = useState(0);
  const vehicleJson = useMapPath(VIEWER3D_VEHICLE);
  const frame = useMemo(() => vehicleSlabs(vehicleJson), [vehicleJson]);
  const [map, setMap] = useState<MapLibreMap | null>(null);
  const [style, setStyle] = useState<MapLibreMap | null>(null);
  const framedOn = useRef<LonLat | null>(null);
  const container = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const view = container.current;
    if (view === null) return undefined;
    const live: { active: boolean; map: MapLibreMap | null } = { active: true, map: null };
    void planMapStyle().then((mapStyle) => {
      if (!live.active) return;
      const loaded = new MapLibreMap({ container: view, attributionControl: false, maplibreLogo: false, fadeDuration: 0, transformRequest: QgcTileInterceptor.transformRequest, center: [0, 0], zoom: 0 });
      live.map = loaded;
      loaded.on('move', () => setCameraMoves((it) => it + 1));
      loaded.on('moveend', () => setCameraMoves((it) => it + 1));
      loaded.on('click', (event) => {
        const hit = pickedMarker([event.point.x, event.point.y], markers.current.map((marker) => (marker.at === null ? null : onScreen(loaded, marker.at))), MARKER_PICK_DP);
        setSelectedMarkers((selected) => selectedAfterTap(selected, markers.current, hit));
      });
      loaded.dragRotate.enable();
      loaded.touchZoomRotate.enableRotation();
      loaded.touchPitch.enable();
      loaded.once('style.load', () => {
        installScene(loaded);
        setStyle(loaded);
      });
      loaded.setStyle(mapStyle.trimStart().startsWith('{') ? (JSON.parse(mapStyle) as StyleSpec) : mapStyle, { diff: false, transformStyle: (_previous, next) => ({ ...next, transition: NO_FADES }) });
      setMap(loaded);
    });
    return () => {
      live.active = false;
      setMap(null);
      setStyle(null);
      live.map?.remove();
    };
  }, []);

  useEffect(() => {
    if (style !== null) setGeoJson(style, V3D_BUILDING_SOURCE, buildingFeatures(scene.buildings));
  }, [style, scene]);

  useEffect(() => {
    if (style !== null) setGeoJson(style, V3D_FLOATING_SOURCE, slabFeatures(slabs));
  }, [style, slabs]);

  useEffect(() => {
    if (style !== null) setGeoJson(style, V3D_VEHICLE_SOURCE, slabFeatures(frame));
  }, [style, frame]);

  const centreLon = scene.centre?.[0] ?? null;
  const centreLat = scene.centre?.[1] ?? null;
  const latestVehicle = useUpdatedRef(vehicle);
  useEffect(() => {
    if (map === null) return;
    const flown = latestVehicle.current;
    const target: LonLat | null = centreLon !== null && centreLat !== null ? [centreLon, centreLat] : flown === null ? null : [flown.longitude, flown.latitude];
    if (target === null) return;
    const held = framedOn.current;
    if ((held !== null && same(held, target)) || (held !== null && centreLon === null)) return;
    framedOn.current = target;
    map.jumpTo({ center: [target[0], target[1]], zoom: SCENE_ZOOM, pitch: SCENE_PITCH });
  }, [map, centreLon, centreLat, vehicle !== null]);

  const placed = useMemo(() => (map !== null && style !== null ? labels.map((it) => [onScreen(map, it.at), it.text] as const) : []), [map, style, labels, cameraMoves]);

  return (
    <div className={`relative overflow-hidden ${className ?? ''}`}>
      <div ref={container} className="absolute inset-0" />
      {placed.map(([at, text], index) => (
        <Text key={index} typography="labelLarge" color={Color.Black} className="pointer-events-none absolute top-0 left-0" style={{ transform: `translate(${Math.trunc(at[0])}px, ${Math.trunc(at[1])}px)` }}>
          {text}
        </Text>
      ))}
      {!scene.available && scene.reason.trim() !== '' ? (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center p-6">
          <Surface shape="medium" color={md('surfaceContainerHigh')} contentColor={md('onSurface')} className="pointer-events-auto">
            <Text typography="bodyMedium" className="block p-4">
              {scene.reason}
            </Text>
          </Surface>
        </div>
      ) : null}
    </div>
  );
};
