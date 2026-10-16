import { type HTMLAttributes, useContext, useEffect, useMemo, useState } from 'react';
import { type Json, jsonObjects, optInt, optJSONArray } from '@/bridge/Json';
import { useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { getSharedPreferences } from '@/kotlin/Platform';
import { delay, same } from '@/kotlin/Std';
import { useValue } from '@/kotlin/State';
import { Surface } from '@/m3';
import { type FenceCircle, type FencePolygon, type FirmwareFence, type RallyPoint, FenceBridge, breachReturn, fenceCircles, fencePolygons, firmwareFence, rallyPoints } from './FenceBridge';
import { LocalFlyMapEdits } from './FlyMapEdits';
import { GimbalBridge, type GimbalAzimuth } from './GimbalWedge';
import { GotoBridge, type GotoLocation } from './GotoMarker';
import { VideoBridge, shotPoints } from './ItemCamera';
import { MapBridge, useMapBool, useMapPath } from './MapBridge';
import type { OtherMission } from './MissionLayers';
import { OperatorBridge, operatorHeading, operatorPoint } from './OperatorPosition';
import { OrbitBridge, type OrbitCircle } from './OrbitMarker';
import { planMapStyle } from './PlanMap';
import { type MissionItem, linksStartToHome, missionItems } from './PlanBridge';
import { RoiBridge } from './RoiMarker';
import { ScaleBarView } from './ScaleBarView';
import { type Survey, SurveyBridge } from './SurveyBridge';
import { TrafficBridge, type TrafficMark } from './TrafficLayer';
import { VEHICLES_VIEW, vehicleChoices } from './VehiclePicker';
import { FlightMapPosition, type TrackPoint, VehicleMap, isPlottable } from './VehicleMap';

const FLY_POLL_MS = 2000;
const CAMERA_STORE = 'fly-map-camera';
const LONG_PRESS_TIMEOUT_MS = 400;
const TOUCH_SLOP_PX = 8;

export type SavedCamera = { readonly centre: TrackPoint; readonly zoom: number };

const toDoubleOrNull = (text: string): number | null => {
  const parsed = Number(text);
  return text.trim() === '' || Number.isNaN(parsed) ? null : parsed;
};

export const readCamera = (): SavedCamera | null => {
  const parts = (getSharedPreferences(CAMERA_STORE).getString('camera', null) ?? '').split(',').flatMap((it) => {
    const parsed = toDoubleOrNull(it);
    return parsed === null ? [] : [parsed];
  });
  const [latitude, longitude, zoom] = parts;
  if (parts.length !== 3 || latitude === undefined || longitude === undefined || zoom === undefined) return null;
  return isPlottable(latitude, longitude) && zoom > 1.0 ? { centre: { latitude, longitude }, zoom } : null;
};

const writeCamera = (camera: SavedCamera): void => {
  getSharedPreferences(CAMERA_STORE).putString('camera', `${camera.centre.latitude},${camera.centre.longitude},${camera.zoom}`);
};

export const centresOnOperator = (alreadyCentred: boolean, operator: TrackPoint | null, vehiclePlaced: boolean): boolean => !alreadyCentred && operator !== null && !vehiclePlaced;

type FlownPlan = {
  readonly items: readonly MissionItem[];
  readonly linkStartToHome: boolean;
  readonly fences: readonly FencePolygon[];
  readonly circles: readonly FenceCircle[];
  readonly firmwareFence: FirmwareFence | null;
  readonly breachReturn: TrackPoint | null;
  readonly rally: readonly RallyPoint[];
  readonly surveys: readonly Survey[];
  readonly operator: TrackPoint | null;
  readonly operatorHeading: number;
  readonly shots: readonly TrackPoint[];
  readonly traffic: readonly TrafficMark[];
  readonly gimbals: readonly GimbalAzimuth[];
  readonly roi: TrackPoint | null;
  readonly goto: GotoLocation | null;
  readonly orbit: OrbitCircle | null;
  readonly current: number | null;
  readonly others: readonly OtherMission[];
  readonly vehiclePlaced: boolean;
};

const EMPTY_PLAN: FlownPlan = {
  items: [],
  linkStartToHome: false,
  fences: [],
  circles: [],
  firmwareFence: null,
  breachReturn: null,
  rally: [],
  surveys: [],
  operator: null,
  operatorHeading: NaN,
  shots: [],
  traffic: [],
  gimbals: [],
  roi: null,
  goto: null,
  orbit: null,
  current: null,
  others: [],
  vehiclePlaced: false,
};

const FLY_MISSION_ITEMS = 'view.flyMissionItems(geometry)';

export const otherMissions = (view: Json | null): readonly OtherMission[] =>
  jsonObjects(optJSONArray(view, 'others')).map((it) => ({ items: missionItems(it), linkStartToHome: linksStartToHome(it) }));

export const missionArrived = <T, K>(before: readonly T[], after: readonly T[], key: (item: T) => K): boolean => after.length > 0 && !same(after.map(key), before.map(key));

const shape = (item: MissionItem) => [item.sequence, item.latitude, item.longitude, item.command];

const PREVIEW_SHEET_SHARE = 0.75;

const keptPlan = (held: FlownPlan, next: FlownPlan): FlownPlan =>
  (Object.keys(next) as (keyof FlownPlan)[]).reduce<FlownPlan>((plan, field) => (same(held[field], next[field]) ? { ...plan, [field]: held[field] } : plan), next);

const readFlownPlan = async (): Promise<FlownPlan> => {
  const [raw, fences, gcs] = await Promise.all([MapBridge.read(FLY_MISSION_ITEMS), FenceBridge.readFlown(), OperatorBridge.read()]);
  if (raw !== null) MapBridge.markReachable();
  const [video, traffic, gimbals, roi, goto, orbit, vehicles] = await Promise.all([
    VideoBridge.read(),
    TrafficBridge.read(),
    GimbalBridge.read(),
    RoiBridge.read(),
    GotoBridge.read(),
    OrbitBridge.read(),
    MapBridge.read(VEHICLES_VIEW),
  ]);
  const selected = raw === null ? -1 : optInt(raw, 'selected', -1);
  return {
    items: missionItems(raw),
    linkStartToHome: linksStartToHome(raw),
    fences: fencePolygons(fences),
    circles: fenceCircles(fences),
    firmwareFence: firmwareFence(fences),
    breachReturn: breachReturn(fences)?.point ?? null,
    rally: rallyPoints(fences),
    surveys: SurveyBridge.surveysFrom(raw),
    operator: operatorPoint(gcs),
    operatorHeading: operatorHeading(gcs),
    shots: shotPoints(video),
    traffic,
    gimbals,
    roi,
    goto,
    orbit,
    current: selected > 0 ? selected : null,
    others: otherMissions(raw),
    vehiclePlaced: vehicleChoices(vehicles).choices.some((it) => isPlottable(it.latitude, it.longitude)),
  };
};

const useDocumentVisible = (): boolean => {
  const [visible, setVisible] = useState(() => typeof document === 'undefined' || document.visibilityState !== 'hidden');
  useEffect(() => {
    const changed = () => setVisible(document.visibilityState !== 'hidden');
    document.addEventListener('visibilitychange', changed);
    return () => document.removeEventListener('visibilitychange', changed);
  }, []);
  return visible;
};

export type FlyMapProps = {
  className?: string;
  cameraBottomPx?: number;
  topInsetPx?: number;
  bottomInsetPx?: number;
  logoEndInsetPx?: number | null;
  pip?: boolean;
  onMapClick?: ((latitude: number, longitude: number) => void) | null;
  onPlainTap?: (() => void) | null;
  onMissionItemClick?: ((sequence: number) => void) | null;
  onRoiClick?: ((at: TrackPoint) => void) | null;
  onTrafficClick?: (() => void) | null;
  clickMarker?: TrackPoint | null;
};

export const FlyMap = ({
  className,
  cameraBottomPx = 0,
  topInsetPx = 0,
  bottomInsetPx = 0,
  logoEndInsetPx = null,
  pip = false,
  onMapClick = null,
  onPlainTap = null,
  onMissionItemClick = null,
  onRoiClick = null,
  onTrafficClick = null,
  clickMarker = null,
}: FlyMapProps) => {
  const [style, setStyle] = useState<string | null>(null);
  const [plan, setPlan] = useState<FlownPlan>(EMPTY_PLAN);
  const [centre, setCentre] = useState<TrackPoint | null>(null);
  const [zoom, setZoom] = useState(0.0);
  const [fitRequest, setFitRequest] = useState(0);
  const gotoPreview = useValue(useContext(LocalFlyMapEdits).gotoPreview);
  const fleetJson = useMapPath(VEHICLES_VIEW);
  const aircraft = useMemo(() => {
    const active = vehicleChoices(fleetJson).active;
    return active === null ? null : { latitude: active.latitude, longitude: active.longitude };
  }, [fleetJson]);
  useEffect(() => {
    if (gotoPreview !== null) setFitRequest((request) => request + 1);
  }, [gotoPreview]);
  const saved = useMemo(() => readCamera(), []);
  const [centreRequest, setCentreRequest] = useState(saved !== null ? 1 : 0);
  const [centreOn, setCentreOn] = useState<TrackPoint | null>(saved?.centre ?? null);
  const [centreZoom, setCentreZoom] = useState<number | null>(saved?.zoom ?? null);
  const keepCentered = useMapBool('view.control(settings.flyViewSettings.keepMapCenteredOnVehicle)');

  const [mainZoom, setMainZoom] = useState(saved?.zoom ?? 0.0);
  const [zoomedForPip, setZoomedForPip] = useState(false);
  useEffect(() => {
    const at = centre;
    if (at === null || pip === zoomedForPip) return;
    setZoomedForPip(pip);
    const level = pipZoom(mainZoom, pip);
    if (level === null) return;
    setCentreOn(at);
    setCentreZoom(level);
    setCentreRequest((request) => request + 1);
  }, [pip, centre !== null]);

  useLaunchedEffect([], async (scope) => {
    const first = await planMapStyle();
    if (scope.isActive()) setStyle(first);
  });

  const latestPlan = useUpdatedRef(plan);
  const visible = useDocumentVisible();
  useLaunchedEffect([visible], async (scope) => {
    if (!visible) return;
    const poll = async (): Promise<void> => {
      const next = await readFlownPlan();
      const latestStyle = await planMapStyle();
      if (!scope.isActive()) return;
      setStyle(latestStyle);
      if (missionArrived(latestPlan.current.items, next.items, shape)) setFitRequest((request) => request + 1);
      if (centresOnOperator(FlightMapPosition.operatorCentred, next.operator, next.vehiclePlaced)) {
        FlightMapPosition.operatorCentred = true;
        setCentreOn(next.operator);
        setCentreZoom(null);
        setCentreRequest((request) => request + 1);
      }
      setPlan((held) => keptPlan(held, next));
      await delay(FLY_POLL_MS);
      if (scope.isActive()) return poll();
      return undefined;
    };
    await poll();
  });

  const onCentreChanged = (at: TrackPoint, level: number) => {
    setCentre(at);
    setZoom(level);
    FlightMapPosition.latest = at;
    if (keepsZoom(level, pip, zoomedForPip)) {
      setMainZoom(level);
      writeCamera({ centre: at, zoom: level });
    }
  };

  return (
    <Surface className={className}>
      <div className="relative size-full">
        {style === null ? null : (
          <div className="absolute inset-0" {...plainTaps(onPlainTap)}>
            <VehicleMap
              className="size-full"
              mapStyle={style}
              follow
              keepCentered={keepCentered || pip}
              pip={pip}
              gimbals={plan.gimbals}
              showsCameraBeam
              proximityRadar
              obstacleOverlay
              cameraBottomPx={cameraBottomPx}
              topInsetPx={topInsetPx}
              bottomInsetPx={bottomInsetPx}
              logoEndInsetPx={logoEndInsetPx}
              missionItems={plan.items}
              linkStartToHome={plan.linkStartToHome}
              fencePolygons={plan.fences}
              fenceCircles={plan.circles}
              firmwareFence={plan.firmwareFence}
              breachReturn={plan.breachReturn}
              rallyPoints={plan.rally}
              operator={plan.operator}
              operatorHeading={plan.operatorHeading}
              surveys={plan.surveys}
              shots={plan.shots}
              traffic={plan.traffic}
              onTrafficClick={onTrafficClick}
              editable={false}
              onMapClick={onMapClick}
              onMissionItemClick={onMissionItemClick}
              roi={plan.roi}
              onRoiClick={onRoiClick}
              goto={plan.goto}
              clickMarker={clickMarker}
              orbit={plan.orbit}
              selectedWaypoint={plan.current}
              otherMissions={plan.others}
              fitRequest={fitRequest}
              fitOnly={gotoPreview === null ? null : [aircraft, gotoPreview].filter((it) => it !== null)}
              fitBottomShare={gotoPreview !== null ? PREVIEW_SHEET_SHARE : 0}
              centreRequest={centreRequest}
              centreOn={centreOn}
              centreZoom={centreZoom}
              onCentreChanged={onCentreChanged}
            />
          </div>
        )}
        {centre !== null && zoom > 0.0 && !pip ? (
          <ScaleBarView
            latitude={centre.latitude}
            zoom={zoom}
            className={logoEndInsetPx === null ? 'absolute bottom-0 left-0 pb-1 pl-1' : 'absolute bottom-0 right-0'}
            style={logoEndInsetPx === null ? undefined : { paddingRight: logoEndInsetPx, paddingBottom: bottomInsetPx + SCALE_ABOVE_LOGO }}
          />
        ) : null}
      </div>
    </Surface>
  );
};

const SCALE_ABOVE_LOGO = 40;
const SCALE_BAR_HEIGHT = 32;

export const MAP_SCALE_CLEARANCE: number = SCALE_ABOVE_LOGO + SCALE_BAR_HEIGHT;

export const keepsZoom = (level: number, pip: boolean, zoomedOutForPip: boolean): boolean => level > 1.0 && !pip && !zoomedOutForPip;

export const pipZoom = (mainZoom: number, pip: boolean): number | null => {
  if (mainZoom <= 0.0) return null;
  if (!pip) return mainZoom;
  if (mainZoom > 3) return mainZoom - 3;
  return null;
};

type TapStart = { readonly pointer: number; readonly at: number; readonly x: number; readonly y: number };

const tapStarts = new WeakMap<EventTarget, TapStart>();

export const plainTaps = (onTap: (() => void) | null): Pick<HTMLAttributes<HTMLElement>, 'onPointerDownCapture' | 'onPointerUpCapture' | 'onPointerCancelCapture'> =>
  onTap === null
    ? {}
    : {
        onPointerDownCapture: (event) => {
          if (event.isPrimary) tapStarts.set(event.currentTarget, { pointer: event.pointerId, at: event.timeStamp, x: event.clientX, y: event.clientY });
        },
        onPointerUpCapture: (event) => {
          const down = tapStarts.get(event.currentTarget);
          if (down === undefined || down.pointer !== event.pointerId) return;
          tapStarts.delete(event.currentTarget);
          const quick = event.timeStamp - down.at < LONG_PRESS_TIMEOUT_MS;
          const still = Math.hypot(event.clientX - down.x, event.clientY - down.y) < TOUCH_SLOP_PX;
          if (quick && still) onTap();
        },
        onPointerCancelCapture: (event) => {
          tapStarts.delete(event.currentTarget);
        },
      };
