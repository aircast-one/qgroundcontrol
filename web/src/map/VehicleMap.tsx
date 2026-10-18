import { useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { Feature, FeatureCollection, GeoJsonProperties, LineString, Point as GeoPoint } from 'geojson';
import { AttributionControl, type GeoJSONSource, type GeoJSONSourceSpecification, type LngLat, LogoControl, Map as MapLibreMap, type MapTouchEvent } from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import { useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { coerceAtLeast, coerceIn, delay } from '@/kotlin/Std';
import { useValue } from '@/kotlin/State';
import { SYNTHETIC_VIEW_PATH, cameraBeam, installCameraBeamLayer, renderCameraBeam } from './CameraBeam';
import { type FenceCircle, type FencePolygon, type FirmwareFence, type RallyPoint, circlesAsPolygons } from './FenceBridge';
import { LocalFlyMapEdits } from './FlyMapEdits';
import { type GimbalAzimuth, installGimbalLayer, renderGimbals } from './GimbalWedge';
import { type GotoLocation, attachGotoRadiusDrag, editedGoto, gotoPath, installClickMarker, installGotoLayer, previewGoto, renderClickMarker, renderGoto, renderGotoPath } from './GotoMarker';
import { type LandingPattern, landingAreaFeatures, landingLabelFeatures, landingLoiterFeatures, landingPathFeatures } from './LandingPattern';
import { FLY_STATE_VIEW, MapBridge, useMapPath, useMapViewFlag } from './MapBridge';
import { type MapHit, attachMissionEditing, hitTest } from './MissionEditing';
import {
  EDGE_LABEL_SOURCE,
  GCS_SOURCE,
  LANDING_AREA_SOURCE,
  LANDING_LABEL_SOURCE,
  LANDING_LABEL_TEXT,
  LANDING_LOITER_SOURCE,
  LANDING_PATH_SOURCE,
  type OtherMission,
  SHOT_SOURCE,
  installFenceHandleLayer,
  installFenceLayers,
  installLandingLayers,
  installMidpointLayer,
  installMissionLayers,
  installShotLayer,
  installSurveyLayers,
  legArrows,
  loiterHandleFeatures,
  loiterRotationArrows,
  operatorFeatures,
  renderCollisionLegs,
  renderFences,
  renderMidpoints,
  renderMission,
  renderSurveys,
  renderVertexHandles,
  shotFeatures,
} from './MissionLayers';
import { OBSTACLE_VIEW, Canvas, TRUE_SCALE_PROBE, drawMapObstacleOverlay, mapOverlayShape, obstacleOverlay } from './ObstacleOverlay';
import { type OrbitCircle, installOrbitLayer, renderOrbit } from './OrbitMarker';
import { edgeLabels } from './EditableShape';
import { clearWindow, fitPoints, planBounds, planPoints } from './PlanBounds';
import type { MissionItem } from './PlanBridge';
import { fencePath, installTraceLayer, renderTrace, surveyPath } from './PolygonTools';
import { installProximityRadarLayer, placedRadars, renderProximityRadars } from './ProximityRadarMap';
import { QgcTileInterceptor } from './QgcTileSource';
import { ROI_LAYER, installRoiLayer, renderRoi } from './RoiMarker';
import { lastSeenText, useSilentSeconds } from './Silence';
import type { Survey } from './SurveyBridge';
import { TRACK_TAIL_VIEW, TRACK_VIEW, type TrackReading, mergedTrack, plottedTrack, trackDraws, trackReading } from './TrackView';
import { TRAFFIC_LAYER, TRAFFIC_SOURCE, type TrafficMark, installTrafficLayer, trafficFeatures } from './TrafficLayer';
import { installTransectMarks, renderGimbalWedges, renderTransectMarks } from './TransectMarks';
import { type Glide, UNPLACED, glidedAt, toward } from './VehicleGlide';
import { VEHICLES_VIEW, type VehicleChoice, vehicleChoices } from './VehiclePicker';

const VEHICLE_SOURCE = 'aircast-vehicle';
const TAP_SLOP_DP = 24;

export const NO_FADES = { duration: 0, delay: 0 };
const VEHICLE_LAYER = 'aircast-vehicle-layer';
const VEHICLE_HEADING_LAYER = 'aircast-vehicle-heading-layer';
const OTHER_VEHICLE_COLOUR = '#90A4AE';
const VEHICLE_ARROW_IMAGE = 'aircast-vehicle-arrow';

export const HEADING_PROPERTY = 'heading';
export const ACTIVE_PROPERTY = 'active';
export const STALE_PROPERTY = 'stale';
export const VEHICLE_LABEL_PROPERTY = 'vehicleLabel';
export const VEHICLE_LABEL_LAYER = 'aircast-vehicle-label';
const TRAIL_SOURCE = 'aircast-trail';
const HOME_SOURCE = 'aircast-home';
const HOME_LAYER = 'aircast-home-layer';
const HOME_LABEL_LAYER = 'aircast-home-label-layer';
const TRAIL_LAYER = 'aircast-trail-layer';

const DEFAULT_ZOOM = 17.0;

const MIN_FIT_SPAN_DEGREES = 1e-5;
const FIT_PADDING_PIXELS = 80;
const LOGO_EDGE_MARGIN_PX = 16;
const STALE_COLOUR = '#9E9E9E';
const LONG_PRESS_TIMEOUT_MS = 400;
const LONG_PRESS_ECHO_MS = 1000;

export const DEMO_STYLE_URL = 'https://demotiles.maplibre.org/style.json';

export const OSM_TILE_URL = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png';

export const OSM_RASTER_STYLE = `
{
  "version": 8,
  "sources": {
    "osm": {
      "type": "raster",
      "tiles": ["${OSM_TILE_URL}"],
      "tileSize": 256,
      "attribution": "(c) OpenStreetMap contributors"
    }
  },
  "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
  "layers": [ { "id": "osm", "type": "raster", "source": "osm" } ]
}
`;

export type TrackPoint = { readonly latitude: number; readonly longitude: number };

export const TrackPoint: { (fields: TrackPoint): TrackPoint; (latitude: number, longitude: number): TrackPoint } = (first: TrackPoint | number, longitude?: number): TrackPoint =>
  typeof first === 'number' ? { latitude: first, longitude: longitude ?? NaN } : { latitude: first.latitude, longitude: first.longitude };

export const FlightMapPosition: { latest: TrackPoint | null; operatorCentred: boolean } = {
  latest: null,
  operatorCentred: false,
};

export const isPlottable = (latitude: number, longitude: number): boolean =>
  !Number.isNaN(latitude) && !Number.isNaN(longitude) && !(latitude === 0.0 && longitude === 0.0) && latitude >= -90.0 && latitude <= 90.0 && longitude >= -180.0 && longitude <= 180.0;

export const featureCollection = (features: readonly Feature[]): FeatureCollection => ({ type: 'FeatureCollection', features: [...features] });

export const pointFeature = (longitude: number, latitude: number, properties: GeoJsonProperties = {}): Feature<GeoPoint> => ({
  type: 'Feature',
  geometry: { type: 'Point', coordinates: [longitude, latitude] },
  properties,
});

export const lineFeature = (points: readonly TrackPoint[], properties: GeoJsonProperties = {}): Feature<LineString> => ({
  type: 'Feature',
  geometry: { type: 'LineString', coordinates: points.map((it) => [it.longitude, it.latitude]) },
  properties,
});

export const geoJsonSource = (): GeoJSONSourceSpecification => ({ type: 'geojson', data: featureCollection([]) });

export const setGeoJson = (style: MapLibreMap, source: string, data: FeatureCollection): void => {
  void style.getSource<GeoJSONSource>(source)?.setData(data);
};

export const drawnImage = (width: number, height: number, draw: (canvas: CanvasRenderingContext2D) => void): ImageData => {
  const canvas = Object.assign(document.createElement('canvas'), { width, height });
  const context = canvas.getContext('2d');
  if (context === null) return new ImageData(width, height);
  draw(context);
  return context.getImageData(0, 0, width, height);
};

export const addBitmap = (style: MapLibreMap, name: string, image: ImageData): void => {
  if (style.hasImage(name)) style.updateImage(name, image);
  else style.addImage(name, image, { pixelRatio: window.devicePixelRatio || 1 });
};

export const setAllGesturesEnabled = (map: MapLibreMap, enabled: boolean): void => {
  [map.dragPan, map.scrollZoom, map.boxZoom, map.dragRotate, map.keyboard, map.doubleClickZoom, map.touchZoomRotate, map.touchPitch].forEach((gesture) => (enabled ? gesture.enable() : gesture.disable()));
};

export const vehicleFeatures = (latitude: number, longitude: number, heading: number, stale = false, lastSeen: string | null = null): FeatureCollection =>
  featureCollection(isPlottable(latitude, longitude) ? [vehicleFeature(latitude, longitude, heading, stale, true, lastSeen)] : []);

export const fleetFeatures = (fleet: readonly VehicleChoice[], lastSeen: string | null = null): FeatureCollection =>
  featureCollection(
    fleet
      .filter((it) => isPlottable(it.latitude, it.longitude))
      .map((flown) => {
        const label = [fleet.length > 1 ? `Vehicle ${flown.id}` : null, flown.active ? lastSeen : null].filter((it) => it !== null).join('\n');
        return vehicleFeature(flown.latitude, flown.longitude, flown.heading, flown.contactLost, flown.active, label === '' ? null : label);
      }),
  );

export const vehicleFeature = (latitude: number, longitude: number, heading: number, stale = false, active = true, label: string | null = null): Feature<GeoPoint> =>
  pointFeature(longitude, latitude, {
    [STALE_PROPERTY]: stale,
    [ACTIVE_PROPERTY]: active,
    ...(label === null ? {} : { [VEHICLE_LABEL_PROPERTY]: label }),
    ...(Number.isNaN(heading) ? {} : { [HEADING_PROPERTY]: ((heading % 360) + 360) % 360 }),
  });

type LoadedStyle = { readonly map: MapLibreMap };

const elapsedRealtime = (): number => performance.now();

const nanoTime = (): number => Math.round(performance.now() * 1_000_000);

const nextFrameNanos = (): Promise<number> => new Promise((resolve) => requestAnimationFrame(() => resolve(nanoTime())));

const pointOf = (at: LngLat): TrackPoint => ({ latitude: at.lat, longitude: at.lng });

const rendered = (map: MapLibreMap, x: number, y: number, slop: number, layer: string): boolean =>
  map.getLayer(layer) !== undefined && map.queryRenderedFeatures([[x - slop, y - slop], [x + slop, y + slop]], { layers: [layer] }).length > 0;

export type VehicleMapProps = {
  className?: string;
  mapStyle?: string;
  follow?: boolean;
  keepCentered?: boolean;
  missionItems?: readonly MissionItem[];
  linkStartToHome?: boolean;
  fencePolygons?: readonly FencePolygon[];
  fenceCircles?: readonly FenceCircle[];
  rallyPoints?: readonly RallyPoint[];
  operator?: TrackPoint | null;
  operatorHeading?: number;
  surveys?: readonly Survey[];
  shots?: readonly TrackPoint[];
  landings?: readonly LandingPattern[];
  editable?: boolean;
  selectedWaypoint?: number | null;
  circledShapes?: ReadonlySet<string>;
  firmwareFence?: FirmwareFence | null;
  onAdd?: (latitude: number, longitude: number) => void;
  onBlankTap?: ((latitude: number, longitude: number) => void) | null;
  onMove?: (hit: MapHit, latitude: number, longitude: number) => void;
  onWaypointSelected?: (hit: MapHit | null) => void;
  onMoved?: (hit: MapHit, latitude: number, longitude: number) => void;
  canDrag?: (hit: MapHit) => boolean;
  onCentreChanged?: (at: TrackPoint, zoom: number) => void;
  onViewChanged?: (corners: readonly TrackPoint[]) => void;
  bottomInsetPx?: number;
  topInsetPx?: number;
  leftInsetPx?: number;
  rightInsetPx?: number;
  logoEndInsetPx?: number | null;
  cameraBottomPx?: number;
  pip?: boolean;
  fitRequest?: number;
  fitOnly?: readonly TrackPoint[] | null;
  fitBottomShare?: number;
  onFitFailed?: () => void;
  centreRequest?: number;
  centreOn?: TrackPoint | null;
  centreZoom?: number | null;
  centreMinZoom?: number | null;
  gestures?: boolean;
  onMapClick?: ((latitude: number, longitude: number) => void) | null;
  onMissionItemClick?: ((sequence: number) => void) | null;
  traffic?: readonly TrafficMark[];
  onTrafficClick?: (() => void) | null;
  gimbals?: readonly GimbalAzimuth[];
  showsCameraBeam?: boolean;
  breachReturn?: TrackPoint | null;
  proximityRadar?: boolean;
  obstacleOverlay?: boolean;
  tracePoints?: readonly TrackPoint[];
  traceLine?: boolean;
  roi?: TrackPoint | null;
  onRoiClick?: ((at: TrackPoint) => void) | null;
  goto?: GotoLocation | null;
  clickMarker?: TrackPoint | null;
  collisionLegs?: readonly (readonly [TrackPoint, TrackPoint])[];
  orbit?: OrbitCircle | null;
  otherMissions?: readonly OtherMission[];
};

const NONE: readonly never[] = [];
const NO_SHAPES: ReadonlySet<string> = new Set();
const ignore = () => undefined;

export const VehicleMap = ({
  className,
  mapStyle = OSM_RASTER_STYLE,
  follow = true,
  keepCentered = true,
  missionItems = NONE,
  linkStartToHome = false,
  fencePolygons = NONE,
  fenceCircles = NONE,
  rallyPoints = NONE,
  operator = null,
  operatorHeading = NaN,
  surveys = NONE,
  shots = NONE,
  landings = NONE,
  editable = false,
  selectedWaypoint = null,
  circledShapes = NO_SHAPES,
  firmwareFence = null,
  onAdd = ignore,
  onBlankTap = null,
  onMove = ignore,
  onWaypointSelected = ignore,
  onMoved = ignore,
  canDrag = () => true,
  onCentreChanged = ignore,
  onViewChanged = ignore,
  bottomInsetPx = 0,
  topInsetPx = 0,
  leftInsetPx = 0,
  rightInsetPx = 0,
  logoEndInsetPx = null,
  cameraBottomPx = 0,
  pip = false,
  fitRequest = 0,
  fitOnly = null,
  fitBottomShare = 0,
  onFitFailed = ignore,
  centreRequest = 0,
  centreOn = null,
  centreZoom = null,
  centreMinZoom = null,
  gestures = true,
  onMapClick = null,
  onMissionItemClick = null,
  traffic = NONE,
  onTrafficClick = null,
  gimbals = NONE,
  showsCameraBeam = false,
  breachReturn = null,
  proximityRadar = false,
  obstacleOverlay: showsObstacleOverlay = false,
  tracePoints = NONE,
  traceLine = false,
  roi = null,
  onRoiClick = null,
  goto = null,
  clickMarker = null,
  collisionLegs = NONE,
  orbit = null,
  otherMissions = NONE,
}: VehicleMapProps) => {
  const mapEdits = useContext(LocalFlyMapEdits);
  const gotoPreview = useValue(mapEdits.gotoPreview);
  const gotoLoiter = useValue(mapEdits.gotoLoiter);
  const orbitPreview = useValue(mapEdits.orbit);
  const latestMapClick = useUpdatedRef(onMapClick);
  const latestItemClick = useUpdatedRef(onMissionItemClick);
  const latestRoi = useUpdatedRef(roi);
  const latestRoiClick = useUpdatedRef(onRoiClick);
  const latestTrafficClick = useUpdatedRef(onTrafficClick);
  const shownGoto = useMemo(() => editedGoto(previewGoto(goto, gotoPreview), gotoLoiter), [goto, gotoPreview, gotoLoiter]);
  const gotoEditing = gotoLoiter !== null && shownGoto?.loiterRadiusMetres != null;
  const latestGoto = useUpdatedRef(shownGoto);
  const latestItems = useUpdatedRef(missionItems);
  const latestOnAdd = useUpdatedRef(onAdd);
  const latestOnBlankTap = useUpdatedRef(onBlankTap);
  const latestOnMove = useUpdatedRef(onMove);
  const latestOnSelected = useUpdatedRef(onWaypointSelected);
  const latestOnMoved = useUpdatedRef(onMoved);
  const latestCanDrag = useUpdatedRef(canDrag);
  const linkLost = useMapViewFlag(FLY_STATE_VIEW, 'contactLost');
  const silent = useSilentSeconds(linkLost);
  const lastSeen = silent === null ? null : lastSeenText(silent);
  const fleetJson = useMapPath(VEHICLES_VIEW);
  const fleet = useMemo(() => vehicleChoices(fleetJson).choices, [fleetJson]);
  const flown = fleet.find((it) => it.active) ?? null;
  const latitude = flown?.latitude ?? NaN;
  const longitude = flown?.longitude ?? NaN;
  const heading = flown?.heading ?? NaN;
  const home = flown?.home ?? null;
  const glides = useRef<ReadonlyMap<number, Glide>>(new Map());

  const container = useRef<HTMLDivElement>(null);
  const [map, setMap] = useState<MapLibreMap | null>(null);
  const [style, setStyle] = useState<LoadedStyle | null>(null);
  const [draggingVertex, setDraggingVertex] = useState<MapHit | null>(null);
  const panning = useRef(false);
  const [trackingResumesAtMs, setTrackingResumesAtMs] = useState(0);
  const trackJson = useMapPath(TRACK_TAIL_VIEW);
  const [track, setTrack] = useState<TrackReading>(() => trackReading(null));
  const latestTrack = useUpdatedRef(track);
  useLaunchedEffect([trackJson], async (scope) => {
    const tail = trackReading(trackJson);
    const merged = mergedTrack(latestTrack.current, tail);
    const whole = merged === null ? await MapBridge.read(TRACK_VIEW) : null;
    const next = merged ?? (whole === null ? null : trackReading(whole)) ?? { ...tail, points: [], from: 0 };
    if (scope.isActive()) setTrack(next);
  });

  const latestCentreChanged = useUpdatedRef(onCentreChanged);
  const latestViewChanged = useUpdatedRef(onViewChanged);
  const latestLeftInset = useUpdatedRef(leftInsetPx);
  const latestBottomInset = useUpdatedRef(bottomInsetPx);
  const latestTopInset = useUpdatedRef(topInsetPx);
  const latestRightInset = useUpdatedRef(rightInsetPx);
  const createdWith = useRef({ editable, clickable: onMapClick !== null });
  useEffect(() => {
    const view = container.current;
    if (view === null) return undefined;
    const loaded = new MapLibreMap({ container: view, attributionControl: false, maplibreLogo: false, fadeDuration: 0, transformRequest: QgcTileInterceptor.transformRequest, center: [0, 0], zoom: 0 });
    const reportCentre = () => {
      latestCentreChanged.current(pointOf(loaded.getCenter()), loaded.getZoom());
      const right = coerceAtLeast(view.clientWidth - latestRightInset.current, 0);
      const left = coerceIn(latestLeftInset.current, 0, right);
      const bottom = coerceAtLeast(view.clientHeight - latestBottomInset.current, 0);
      const top = coerceIn(latestTopInset.current, 0, bottom);
      const seen = ([[left, top], [right, top], [right, bottom], [left, bottom]] as const).map(([x, y]) => pointOf(loaded.unproject([x, y])));
      latestViewChanged.current(clearWindow(seen));
    };
    reportCentre();
    loaded.on('moveend', () => {
      reportCentre();
      if (panning.current) {
        panning.current = false;
        setTrackingResumesAtMs(elapsedRealtime() + PAN_RECENTER_DELAY_MS);
      }
    });
    loaded.on('movestart', (event) => {
      if (event.originalEvent !== undefined) panning.current = true;
    });
    const attachFlyClicks = (): (() => void) => {
      const detachGoto = attachGotoRadiusDrag(loaded.getCanvasContainer(), loaded, mapEdits, () => latestGoto.current);
      loaded.on('click', (event) => {
        if (event.defaultPrevented) return;
        const { x, y } = event.point;
        const trafficClick = latestTrafficClick.current;
        if (trafficClick !== null && rendered(loaded, x, y, TAP_SLOP_DP, TRAFFIC_LAYER)) {
          trafficClick();
          return;
        }
        const roiTapped = latestRoi.current !== null && rendered(loaded, x, y, TAP_SLOP_DP, ROI_LAYER) ? latestRoi.current : null;
        const roiClick = latestRoiClick.current;
        if (roiTapped !== null && roiClick !== null) {
          roiClick(roiTapped);
          return;
        }
        const hit = hitTest(loaded, x, y);
        const item = hit?.kind === 'Waypoint' ? (latestItems.current.find((it) => it.index === hit.index) ?? null) : null;
        const itemClick = latestItemClick.current;
        if (item !== null && itemClick !== null) itemClick(item.sequence);
      });
      const press = { timer: null as ReturnType<typeof setTimeout> | null, firedAt: -LONG_PRESS_ECHO_MS };
      const cancelPress = () => {
        if (press.timer !== null) clearTimeout(press.timer);
        press.timer = null;
      };
      const longClick = (at: LngLat) => latestMapClick.current?.(at.lat, at.lng);
      loaded.on('touchstart', (event: MapTouchEvent) => {
        cancelPress();
        if (event.defaultPrevented || event.points.length !== 1) return;
        const at = event.lngLat;
        press.timer = setTimeout(() => {
          press.timer = null;
          press.firedAt = elapsedRealtime();
          longClick(at);
        }, LONG_PRESS_TIMEOUT_MS);
      });
      loaded.on('touchend', cancelPress);
      loaded.on('touchcancel', cancelPress);
      loaded.on('movestart', (event) => {
        if (event.originalEvent !== undefined) cancelPress();
      });
      loaded.on('contextmenu', (event) => {
        event.originalEvent.preventDefault();
        if (elapsedRealtime() - press.firedAt < LONG_PRESS_ECHO_MS) return;
        longClick(event.lngLat);
      });
      return () => {
        detachGoto();
        cancelPress();
      };
    };
    const detachClicks = !createdWith.current.editable && createdWith.current.clickable ? attachFlyClicks() : ignore;
    if (createdWith.current.editable) {
      attachMissionEditing(
        loaded.getCanvasContainer(),
        loaded,
        (latitude: number, longitude: number) => latestOnAdd.current(latitude, longitude),
        (hit: MapHit, latitude: number, longitude: number) => latestOnMove.current(hit, latitude, longitude),
        (hit: MapHit | null) => latestOnSelected.current(hit),
        (hit: MapHit, latitude: number, longitude: number) => latestOnMoved.current(hit, latitude, longitude),
        (hit: MapHit | null) => setDraggingVertex(hit),
        (hit: MapHit) => latestCanDrag.current(hit),
        (latitude: number, longitude: number) => {
          const blank = latestOnBlankTap.current;
          if (blank !== null) blank(latitude, longitude);
          else latestOnSelected.current(null);
        },
      );
    }
    setMap(loaded);
    return () => {
      detachClicks();
      setMap(null);
      setStyle(null);
      loaded.remove();
    };
  }, []);

  useEffect(() => {
    if (map === null) return undefined;
    setStyle(null);
    const installed = () => {
      installLayers(map);
      installSurveyLayers(map);
      installTransectMarks(map);
      installLandingLayers(map);
      installMidpointLayer(map);
      installShotLayer(map);
      installFenceLayers(map);
      installMissionLayers(map);
      installTraceLayer(map);
      installFenceHandleLayer(map);
      installTrafficLayer(map);
      installRoiLayer(map);
      installGotoLayer(map);
      installClickMarker(map);
      installOrbitLayer(map);
      installVehicleLayer(map);
      setDraggingVertex(null);
      setStyle({ map });
    };
    map.once('style.load', installed);
    map.setStyle(mapStyle.trimStart().startsWith('{') ? (JSON.parse(mapStyle) as Exclude<Parameters<MapLibreMap['setStyle']>[0], string | null>) : mapStyle, {
      diff: false,
      transformStyle: (_previous, next) => ({ ...next, transition: NO_FADES }),
    });
    return () => {
      map.off('style.load', installed);
    };
  }, [map, mapStyle]);

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    setGeoJson(loaded, EDGE_LABEL_SOURCE, featureCollection(edgeLabels(draggingVertex, fencePolygons, surveys).map((label: { readonly at: TrackPoint; readonly text: string }) => pointFeature(label.at.longitude, label.at.latitude, { [LANDING_LABEL_TEXT]: label.text }))));
  }, [style, draggingVertex, fencePolygons, surveys]);

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    setGeoJson(loaded, TRAFFIC_SOURCE, trafficFeatures(traffic));
  }, [style, traffic]);

  const radars = useMemo(() => (proximityRadar ? placedRadars(fleet, { latitude, longitude }, heading) : NONE), [proximityRadar, fleet, latitude, longitude, heading]);
  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderProximityRadars(loaded, radars);
  }, [style, radars]);

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderOrbit(loaded, orbit, shownGoto !== null, orbitPreview);
  }, [style, orbit, shownGoto, orbitPreview]);
  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderCollisionLegs(loaded, collisionLegs);
  }, [style, collisionLegs]);
  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderClickMarker(loaded, orbitPreview === null ? clickMarker : null);
  }, [style, clickMarker, orbitPreview]);
  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderGoto(loaded, shownGoto, gotoEditing);
    renderGotoPath(loaded, gotoPath({ latitude, longitude }, gotoPreview));
  }, [style, shownGoto, gotoEditing, gotoPreview, latitude, longitude]);
  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderRoi(loaded, roi);
  }, [style, roi]);

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderTrace(loaded, tracePoints, traceLine);
  }, [style, tracePoints, traceLine]);

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderGimbals(loaded, latitude, longitude, gimbals);
  }, [style, latitude, longitude, gimbals]);

  const syntheticJson = useMapPath(SYNTHETIC_VIEW_PATH);
  const beam = useMemo(() => (showsCameraBeam ? cameraBeam(syntheticJson) : NONE), [syntheticJson, showsCameraBeam]);
  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    renderCameraBeam(loaded, beam);
  }, [style, beam]);

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    setGeoJson(loaded, SHOT_SOURCE, shotFeatures(shots));
  }, [style, shots]);

  const recenterOnVehicle = () => {
    const glide = flown === null ? undefined : glides.current.get(flown.id);
    const drawn = glide === undefined ? UNPLACED : glide.at(nanoTime());
    const tracking = follow && isPlottable(drawn.latitude, drawn.longitude) && !panning.current && elapsedRealtime() >= trackingResumesAtMs;
    const shown = map;
    const view = container.current;
    if (!tracking || shown === null || view === null) return;
    const at: [number, number] = [drawn.longitude, drawn.latitude];
    const zoomed = shown.getZoom() > 1.0;
    const point = shown.project(at);
    const height = view.clientHeight;
    const lift = coerceIn(clearAreaLift(topInsetPx, bottomInsetPx), -height / 4, height / 4);
    const centred = zoomed && lift !== 0 ? shown.unproject([point.x, point.y - lift]) : at;
    if (keepCentered || !zoomed) {
      shown.jumpTo({ center: centred, zoom: zoomed ? shown.getZoom() : DEFAULT_ZOOM });
      return;
    }
    if (outsideCentreInset(point.x, point.y, view.clientWidth, height, topInsetPx, bottomInsetPx + cameraBottomPx)) {
      shown.easeTo({ center: centred, duration: RECENTER_ANIMATION_MS });
    }
  };
  const recenterNow = useUpdatedRef(recenterOnVehicle);

  useLaunchedEffect([style, latitude, longitude, heading, home, linkLost, fleet, lastSeen], async (scope) => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    const segments = toward(glides.current, fleet, nanoTime());
    glides.current = segments;
    setGeoJson(loaded, HOME_SOURCE, featureCollection((fleet.length === 0 ? [home] : fleet.map((it) => it.home)).flatMap((it) => (it === null ? [] : [pointFeature(it.longitude, it.latitude)]))));
    recenterNow.current();
    const frame = async (): Promise<void> => {
      const now = await nextFrameNanos();
      if (!scope.isActive()) return;
      setGeoJson(loaded, VEHICLE_SOURCE, fleet.length === 0 ? vehicleFeatures(latitude, longitude, heading, linkLost, lastSeen) : fleetFeatures(glidedAt(fleet, segments, now), lastSeen));
      if (keepCentered) recenterNow.current();
      if ([...segments.values()].some((it) => now < it.endNs)) return frame();
      return undefined;
    };
    await frame();
  });

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    setGeoJson(loaded, TRAIL_SOURCE, featureCollection(trackDraws(track) ? [lineFeature(plottedTrack(track))] : []));
  }, [style, track]);

  useLaunchedEffect([trackingResumesAtMs], async (scope) => {
    if (trackingResumesAtMs === 0) return;
    await delay(coerceAtLeast(trackingResumesAtMs - elapsedRealtime(), 0));
    if (scope.isActive()) recenterNow.current();
  });

  useEffect(() => {
    const loaded = style?.map;
    if (loaded !== undefined) applyPip(loaded, pip);
  }, [style, pip]);

  const logoAtEnd = logoEndInsetPx !== null;
  useEffect(() => {
    if (map === null || pip) return undefined;
    const logo = new LogoControl({ compact: false });
    const attribution = new AttributionControl({ compact: true });
    if (logoAtEnd) {
      map.addControl(logo, 'bottom-right');
      map.addControl(attribution, 'bottom-right');
    } else {
      map.addControl(attribution, 'bottom-left');
      map.addControl(logo, 'bottom-left');
    }
    return () => {
      if (map.hasControl(attribution)) map.removeControl(attribution);
      if (map.hasControl(logo)) map.removeControl(logo);
    };
  }, [map, pip, logoAtEnd]);

  useEffect(() => {
    map?.setPadding({ top: 0, left: 0, right: 0, bottom: cameraBottomPx });
  }, [map, cameraBottomPx]);

  useEffect(() => {
    if (map !== null) placeMapChrome(map, bottomInsetPx, leftInsetPx, logoEndInsetPx);
  }, [map, bottomInsetPx, leftInsetPx, logoEndInsetPx, pip]);

  const centred = useRef(0);
  useEffect(() => {
    const map = style?.map;
    if (centreRequest === 0 || centreRequest === centred.current || map === undefined) return;
    centred.current = centreRequest;
    const at = centreOn !== null && isPlottable(centreOn.latitude, centreOn.longitude) ? centreOn : null;
    if (at === null) return;
    const target: [number, number] = [at.longitude, at.latitude];
    if (centreZoom !== null) {
      map.jumpTo({ center: target, zoom: centreZoom });
    } else if (centreMinZoom !== null && map.getZoom() < centreMinZoom) {
      map.easeTo({ center: target, zoom: centreMinZoom });
    } else if (map.getZoom() > 1.0) {
      const point = map.project(target);
      map.easeTo({ center: map.unproject([point.x - leftInsetPx / 2, point.y - clearAreaLift(topInsetPx, bottomInsetPx)]) });
    } else {
      map.easeTo({ center: target, zoom: DEFAULT_ZOOM });
    }
  }, [centreRequest, style]);

  useEffect(() => {
    if (map !== null) setAllGesturesEnabled(map, gestures);
  }, [map, gestures]);

  const fitted = useRef(0);
  useEffect(() => {
    const map = style?.map;
    if (fitRequest === 0 || fitRequest === fitted.current || map === undefined) return;
    fitted.current = fitRequest;
    const bounds = planBounds(fitPoints(fitOnly ?? planPoints(missionItems, fencePolygons, fenceCircles, rallyPoints, surveys), latitude, longitude));
    if (bounds === null) {
      onFitFailed();
      return;
    }
    if (bounds.spanDegrees < MIN_FIT_SPAN_DEGREES) {
      map.easeTo({ center: [bounds.centre.longitude, bounds.centre.latitude], zoom: DEFAULT_ZOOM });
      return;
    }
    map.fitBounds([[bounds.west, bounds.south], [bounds.unwrappedEast, bounds.north]], {
      padding: {
        left: FIT_PADDING_PIXELS + leftInsetPx,
        top: FIT_PADDING_PIXELS + topInsetPx,
        right: FIT_PADDING_PIXELS,
        bottom: FIT_PADDING_PIXELS + bottomInsetPx + Math.trunc((container.current?.clientHeight ?? 0) * fitBottomShare),
      },
    });
  }, [fitRequest, style]);

  useEffect(() => {
    const loaded = style?.map;
    if (loaded === undefined) return;
    const editedItems = editable ? missionItems : NONE;
    renderSurveys(loaded, surveys);
    renderTransectMarks(loaded, surveys, selectedWaypoint, [...legArrows(missionItems, linkStartToHome), ...loiterRotationArrows(editedItems)]);
    renderGimbalWedges(loaded, missionItems);
    renderLandings(loaded, landings, editedItems, editable ? selectedWaypoint : null);
    renderMidpoints(
      loaded,
      fencePolygons.map((it) => (circledShapes.has(fencePath(it.index)) ? { ...it, editable: null } : it)),
      surveys.map((it) => (circledShapes.has(surveyPath(it)) ? { ...it, editable: null } : it)),
      editedItems,
      selectedWaypoint,
    );
    renderFences(loaded, fencePolygons, rallyPoints, circlesAsPolygons(fenceCircles), firmwareFence, breachReturn !== null && isPlottable(breachReturn.latitude, breachReturn.longitude) ? breachReturn : null);
    setGeoJson(loaded, GCS_SOURCE, operatorFeatures(operator, operatorHeading));
    renderVertexHandles(loaded, fencePolygons, surveys, fenceCircles, landings, circledShapes, loiterHandleFeatures(editedItems, selectedWaypoint));
    renderMission(loaded, missionItems, linkStartToHome, selectedWaypoint, otherMissions, landings, editable);
  }, [style, missionItems, fencePolygons, fenceCircles, rallyPoints, surveys, landings, firmwareFence, selectedWaypoint, linkStartToHome, operator, operatorHeading, breachReturn, otherMissions, circledShapes]);

  return (
    <div className={`relative ${className ?? ''}`}>
      <div ref={container} className="absolute! inset-0 [&_.maplibregl-ctrl]:m-0! [&_.maplibregl-ctrl]:clear-none!" />
      {showsObstacleOverlay ? <ObstacleMapOverlay map={map} latitude={latitude} longitude={longitude} heading={heading} showText={!pip} /> : null}
    </div>
  );
};

const ObstacleMapOverlay = ({ map, latitude, longitude, heading, showText }: { map: MapLibreMap | null; latitude: number; longitude: number; heading: number; showText: boolean }) => {
  const json = useMapPath(OBSTACLE_VIEW);
  const [, setCameraMoves] = useState(0);
  useEffect(() => {
    if (map === null) return undefined;
    const moved = () => setCameraMoves((moves) => moves + 1);
    map.on('move', moved);
    return () => {
      map.off('move', moved);
    };
  }, [map]);
  const overlay = useMemo(() => obstacleOverlay(json), [json]);
  if (overlay === null) return null;
  return (
    <Canvas
      className="absolute inset-0 size-full"
      onDraw={(scope) => {
        if (map === null || !isPlottable(latitude, longitude) || Number.isNaN(heading)) return;
        const vehicle = map.project([longitude, latitude]);
        const probe = TRUE_SCALE_PROBE;
        const metresInProbe = map.unproject([0, 0]).distanceTo(map.unproject([probe, 0]));
        const centre = { x: vehicle.x, y: vehicle.y };
        const shape = mapOverlayShape(overlay, centre, scope.height, metresInProbe, probe, heading, map.getBearing());
        drawMapObstacleOverlay(scope, overlay, shape, centre, showText);
      }}
    />
  );
};

const placeMapChrome = (map: MapLibreMap, bottomInsetPx: number, leftInsetPx: number, logoEndInsetPx: number | null): void => {
  const corner = (side: 'left' | 'right') => map.getContainer().querySelector<HTMLElement>(`.maplibregl-ctrl-bottom-${side}`);
  const bottom = `${bottomInsetPx + LOGO_EDGE_MARGIN_PX}px`;
  const start = corner('left');
  const end = corner('right');
  if (start !== null) Object.assign(start.style, logoEndInsetPx === null ? { paddingLeft: `${leftInsetPx + LOGO_EDGE_MARGIN_PX}px`, paddingBottom: bottom } : { paddingLeft: '', paddingBottom: '' });
  if (end !== null) Object.assign(end.style, logoEndInsetPx === null ? { paddingRight: '', paddingBottom: '' } : { paddingRight: `${logoEndInsetPx}px`, paddingBottom: bottom });
};

const installLayers = (style: MapLibreMap): void => {
  if (style.getSource(TRAIL_SOURCE) === undefined) {
    style.addSource(TRAIL_SOURCE, geoJsonSource());
    style.addLayer({ id: TRAIL_LAYER, type: 'line', source: TRAIL_SOURCE, paint: { 'line-color': '#FF0000', 'line-width': 3 } });
  }
  if (style.getSource(HOME_SOURCE) === undefined) {
    style.addSource(HOME_SOURCE, geoJsonSource());
    style.addLayer({
      id: HOME_LAYER,
      type: 'circle',
      source: HOME_SOURCE,
      paint: { 'circle-color': '#43A047', 'circle-radius': 12, 'circle-stroke-color': '#FFFFFF', 'circle-stroke-width': 2 },
    });
    style.addLayer({
      id: HOME_LABEL_LAYER,
      type: 'symbol',
      source: HOME_SOURCE,
      layout: { 'text-field': 'H', 'text-font': ['Noto Sans Regular'], 'text-size': 14, 'text-allow-overlap': true, 'text-ignore-placement': true },
      paint: { 'text-color': '#FFFFFF' },
    });
  }
};

export const PIP_ICON_SCALE = 1 / 3;
export const PIP_TRAFFIC_SCALE = 1 / 2.5;

const applyPip = (style: MapLibreMap, pip: boolean): void => {
  if (style.getLayer(TRAIL_LAYER) !== undefined) style.setLayoutProperty(TRAIL_LAYER, 'visibility', pip ? 'none' : 'visible');
  const scale = pip ? PIP_ICON_SCALE : 1;
  if (style.getLayer(VEHICLE_LAYER) !== undefined) {
    style.setPaintProperty(VEHICLE_LAYER, 'circle-radius', ['case', ['get', ACTIVE_PROPERTY], 9 * scale, 6 * scale]);
    style.setPaintProperty(VEHICLE_LAYER, 'circle-stroke-width', 2 * scale);
  }
  if (style.getLayer(VEHICLE_HEADING_LAYER) !== undefined) style.setLayoutProperty(VEHICLE_HEADING_LAYER, 'icon-size', scale);
  if (style.getLayer(TRAFFIC_LAYER) !== undefined) {
    style.setLayoutProperty(TRAFFIC_LAYER, 'icon-size', pip ? PIP_TRAFFIC_SCALE : 1);
    style.setLayoutProperty(TRAFFIC_LAYER, 'text-size', pip ? 11 * PIP_TRAFFIC_SCALE : 11);
  }
};

const installVehicleLayer = (style: MapLibreMap): void => {
  installCameraBeamLayer(style);
  installProximityRadarLayer(style);
  installGimbalLayer(style);
  if (style.getSource(VEHICLE_SOURCE) !== undefined) return;
  style.addSource(VEHICLE_SOURCE, geoJsonSource());
  style.addLayer({
    id: VEHICLE_LAYER,
    type: 'circle',
    source: VEHICLE_SOURCE,
    paint: {
      'circle-color': ['case', ['get', STALE_PROPERTY], STALE_COLOUR, ['!', ['get', ACTIVE_PROPERTY]], OTHER_VEHICLE_COLOUR, '#E53935'],
      'circle-radius': ['case', ['get', ACTIVE_PROPERTY], 9, 6],
      'circle-stroke-color': '#FFFFFF',
      'circle-stroke-width': 2,
    },
  });
  addBitmap(style, VEHICLE_ARROW_IMAGE, headingArrow());
  style.addLayer({
    id: VEHICLE_HEADING_LAYER,
    type: 'symbol',
    source: VEHICLE_SOURCE,
    filter: ['has', HEADING_PROPERTY],
    layout: {
      'icon-image': VEHICLE_ARROW_IMAGE,
      'icon-rotate': ['get', HEADING_PROPERTY],
      'icon-rotation-alignment': 'map',
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    },
    paint: { 'icon-opacity': ['case', ['get', STALE_PROPERTY], 0.4, 1.0] },
  });
  style.addLayer({
    id: VEHICLE_LABEL_LAYER,
    type: 'symbol',
    source: VEHICLE_SOURCE,
    filter: ['has', VEHICLE_LABEL_PROPERTY],
    layout: {
      'text-field': ['get', VEHICLE_LABEL_PROPERTY],
      'text-size': 11,
      'text-offset': [0, 1.6],
      'text-anchor': 'top',
      'text-allow-overlap': true,
      'text-ignore-placement': false,
    },
    paint: { 'text-color': '#FFFFFF', 'text-halo-color': '#000000', 'text-halo-width': 1 },
  });
};

const headingArrow = (): ImageData =>
  drawnImage(48, 48, (canvas) => {
    const size = 48;
    const path = new Path2D(`M${size / 2} 2 L${size - 8} ${size - 6} L${size / 2} ${size * 0.72} L8 ${size - 6} Z`);
    canvas.strokeStyle = '#FFFFFF';
    canvas.lineWidth = 6;
    canvas.lineJoin = 'round';
    canvas.stroke(path);
    canvas.fillStyle = '#E53935';
    canvas.fill(path);
  });

const renderLandings = (style: MapLibreMap, landings: readonly LandingPattern[], items: readonly MissionItem[], selected: number | null): void => {
  setGeoJson(style, LANDING_AREA_SOURCE, landingAreaFeatures(landings));
  setGeoJson(style, LANDING_LABEL_SOURCE, landingLabelFeatures(landings, selected, items));
  setGeoJson(style, LANDING_PATH_SOURCE, landingPathFeatures(landings));
  setGeoJson(style, LANDING_LOITER_SOURCE, landingLoiterFeatures(landings, items));
};

const PAN_RECENTER_DELAY_MS = 10_000;
const RECENTER_ANIMATION_MS = 1000;
const CENTRE_INSET_FRACTION = 0.15;

export const clearAreaLift = (topInset: number, bottomInset: number): number => (topInset - bottomInset) / 2;

export const outsideCentreInset = (x: number, y: number, width: number, height: number, topInset: number, bottomInset: number): boolean => {
  const side = width * CENTRE_INSET_FRACTION;
  const top = topInset + height * CENTRE_INSET_FRACTION;
  const bottom = height - bottomInset - height * CENTRE_INSET_FRACTION;
  return width > 0 && height > 0 && (x < side || x > width - side || y < top || y > bottom);
};
