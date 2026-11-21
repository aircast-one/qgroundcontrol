import {
  type CSSProperties,
  type HTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  type Ref,
  type RefCallback,
  type RefObject,
  type SyntheticEvent,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from 'react';
import { type Json, has, optBoolean } from '@/bridge/Json';
import { useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import { useUpdatedRef } from '@/kotlin/Compose';
import { getSharedPreferences } from '@/kotlin/Platform';
import { useValue } from '@/kotlin/State';
import { lostVehicle } from '@/map/LostVehicle';
import { FOLLOW_VEHICLE, mapFollowing, mapWindow, vehicleOffScreen } from '@/map/MapFollow';
import { type TrackPoint, isPlottable } from '@/map/VehicleMap';
import { VEHICLES_VIEW, vehicleChoices } from '@/map/VehiclePicker';
import { coerceAtLeast, same } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { Map3d, useMap3dShown } from '@/map/Map3d';
import { MapLayersSheet } from '@/map/MapLayersSheet';
import { Viewer3DPane } from '@/map/Viewer3D';
import { Drawable, MaterialIcon } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { AircastSpace, Color, aircast, md, withAlpha } from '@/theme/Colors';
import { FleetPanel } from '@/ui/ActiveVehicle';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { CAMERA_VIEW, cameraReading } from '@/ui/CameraControl';
import { railAboveShutter } from '@/ui/CameraControlLayer';
import { HudLadders } from '@/ui/HudLadders';
import { useRememberDragAims } from '@/ui/CameraSwitch';
import { FLY_MENU, FlyDeckLayout } from '@/ui/FlyDeck';
import { LocalFlyScreenState, MENU_SHEET } from '@/ui/FlyScreenState';
import { useArmed, useHasVehicle } from '@/ui/FlyState';
import { ApproximateAimNote, gimbalSwitchShown } from '@/ui/GimbalIndicator';
import { OfflineGuideCard } from '@/ui/OfflineStatusSheet';
import { LocalCompactHud, LocalFlyOsd, OSD_CONTROL_SIZE, OSD_SCRIM, compactHud, osdShadow, useOsdBackdrop, useOsdTint } from '@/ui/Osd';
import { PayloadRailStrip } from '@/ui/PayloadRail';
import { map3dToggleLabel, map3dToggleText, useMap3dDrawn } from '@/ui/Map3dView';
import { BATTERY_BAR_KEY, LayoutEditingNotice, LayoutWidget, MINIMAP_KEY, MINIMAP_NAME, useHoldThroughToEditLayout, useJiggleAngle } from '@/ui/OverlayLayout';
import { HeadingTape } from '@/ui/HeadingTape';
import { BatteryReturnBar, LocalCompactStatus } from '@/ui/StatusStrip';
import { nativeChosen } from '@/ui/SyntheticSurface';
import { SYNTHETIC_VIEW, VIDEO_CORNER_ROOM, useRememberSyntheticAims, useRememberSyntheticAvailable, useRememberSyntheticChosen } from '@/ui/SyntheticView';
import { UpdateBanner } from '@/ui/AppUpdates';
import { TrafficBanner } from '@/ui/TrafficBanner';
import { HudReadings, TelemetryRow } from '@/ui/VehicleUi';
import {
  VIDEO_SWIPE_DISTANCE,
  type VideoGestureHandlers,
  VideoSwipe,
  cameraStep,
  sideways,
  useRememberCameraStepper,
  useRememberVideoFrame,
  verticalSwipe,
  videoFrame,
  videoGestures,
  videoSwipe,
} from '@/ui/VideoGestures';
import { FlyNoVideoMessage, NO_VIDEO_PILL_HEIGHT, NO_VIDEO_PILL_WIDTH, centredRegion, looksForAircraft, messageRegion } from '@/ui/VideoSurface';
import { VIDEO_VIEW, videoReading } from '@/ui/VideoView';
import { useDesktopLayout } from '@/ui/DesktopLayout';
import { VirtualJoystick, useShownStickPad } from '@/ui/VirtualJoystick';

const FLY_STORE = 'fly';
const FLY_VIEW_KEY = 'view';
export const STATUS_ROW_HEIGHT = 32;
const PAYLOAD_STRIP_CLEARANCE = 64;
const ACTION_RAIL_CLEARANCE = 72;
const STOP_CLEARANCE = 56;
const RAIL_CLEARANCE = STOP_CLEARANCE + AircastSpace.s3 * 2;
const TOP_SCRIM_HEIGHT = 96;
const TOP_SCRIM_ALPHA = 0.6;
const MINIMAP_THUMB = 56;
const FULL_SCREEN_LAYER = 10;
const STICK_LAYER = FULL_SCREEN_LAYER + 1;
const DECISION_LAYER = 4;
const PIP_LAYER = 0.5;
const MAP_SCALE_CLEARANCE = 88;
const CORNER_STICK_MIN_HEIGHT = 560;
const STICK_FLOOR = MINIMAP_THUMB + AircastSpace.s3 * 2;
const RAIL_SHUTTER_ROOM = 56;
const BOTTOM_LEAD = ACTION_RAIL_CLEARANCE - AircastSpace.s3;

export type Offset = { x: number; y: number };
export const Offset = (x: number, y: number): Offset => ({ x, y });
export type Size = { width: number; height: number };
export const Size = (width: number, height: number): Size => ({ width, height });
export type Rect = { left: number; top: number; right: number; bottom: number; width: number; height: number };
export const Rect = (left: number, top: number, right: number, bottom: number): Rect => ({ left, top, right, bottom, width: right - left, height: bottom - top });
export const rectAt = (topLeft: Offset, size: Size): Rect => Rect(topLeft.x, topLeft.y, topLeft.x + size.width, topLeft.y + size.height);
export const rectCenter = (rect: Rect): Offset => Offset((rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2);
export const rectTopLeft = (rect: Rect): Offset => Offset(rect.left, rect.top);
export const translated = (rect: Rect, dx: number, dy: number): Rect => Rect(rect.left + dx, rect.top + dy, rect.right + dx, rect.bottom + dy);
export const plus = (a: Offset, b: Offset): Offset => Offset(a.x + b.x, a.y + b.y);
export const minus = (a: Offset, b: Offset): Offset => Offset(a.x - b.x, a.y - b.y);

export type ModifierProps = HTMLAttributes<HTMLDivElement> & { ref?: Ref<HTMLDivElement> };

export const PASS_THROUGH =
  'pointer-events-none [&_button]:pointer-events-auto [&_a]:pointer-events-auto [&_input]:pointer-events-auto [&_textarea]:pointer-events-auto [&_[role=button]]:pointer-events-auto [&_[role=slider]]:pointer-events-auto [&_[role=switch]]:pointer-events-auto [&_[role=tab]]:pointer-events-auto [&_[data-gesture]]:pointer-events-auto';

const layer = (z: number): number => Math.round(z * 2);

const stopPropagation = (event: SyntheticEvent) => event.stopPropagation();

const stopActivation = (event: KeyboardEvent) => {
  if (event.key === 'Enter' || event.key === ' ') event.stopPropagation();
};

export const activate = (action: () => void) => (event: KeyboardEvent) => {
  if (event.key !== 'Enter' && event.key !== ' ') return;
  event.preventDefault();
  action();
};

export const StopPropagation = ({ className, style, children }: { className?: string; style?: CSSProperties; children: ReactNode }) => (
  <div className={className} style={style} onPointerDown={stopPropagation} onClick={stopPropagation} onKeyDown={stopActivation}>
    {children}
  </div>
);

export const A11yActions = ({ actions }: { actions: readonly (readonly [string, () => void])[] }) => (
  <>
    {actions.map(([label, run]) => (
      <button key={label} type="button" className="sr-only" onPointerDown={stopPropagation} onKeyDown={stopActivation} onClick={(event) => { event.stopPropagation(); run(); }}>
        {label}
      </button>
    ))}
  </>
);

const EXIT_MS = 300;

export const AnimatedVisibility = ({ visible, from = null, className, style, children }: { visible: boolean; from?: 'top' | 'bottom' | null; className?: string; style?: CSSProperties; children: ReactNode }) => {
  const [mounted, setMounted] = useState(visible);
  useEffect(() => {
    if (visible) {
      setMounted(true);
      return undefined;
    }
    const timer = setTimeout(() => setMounted(false), EXIT_MS);
    return () => clearTimeout(timer);
  }, [visible]);
  if (!visible && !mounted) return null;
  const slide = from === 'bottom' ? 'translate-y-full' : from === 'top' ? '-translate-y-full' : '';
  const entering = from === 'bottom' ? 'starting:translate-y-full' : from === 'top' ? 'starting:-translate-y-full' : '';
  const shown = visible ? `translate-y-0 opacity-100 starting:opacity-0 ${entering}` : `opacity-0 ${slide}`;
  return (
    <div className={`transition-[opacity,translate] duration-300 motion-reduce:transition-none ${shown} ${className ?? ''}`} style={style}>
      {children}
    </div>
  );
};

const ZERO_SIZE = Size(0, 0);

const MinimapWidget = ({ editing, children }: { editing: boolean; children: ReactNode }) =>
  editing ? (
    <LayoutWidget widgetKey={MINIMAP_KEY} movable={false} name={MINIMAP_NAME}>
      {children}
    </LayoutWidget>
  ) : (
    children
  );

export const useElementSize = (): { ref: RefCallback<HTMLElement>; size: Size; node: RefObject<HTMLElement | null> } => {
  const [size, setSize] = useState<Size>(ZERO_SIZE);
  const node = useRef<HTMLElement | null>(null);
  const ref = useCallback((element: HTMLElement | null) => {
    node.current = element;
    if (element === null) return undefined;
    const read = () => setSize((was) => (was.width === element.offsetWidth && was.height === element.offsetHeight ? was : Size(element.offsetWidth, element.offsetHeight)));
    read();
    const observer = new ResizeObserver(read);
    observer.observe(element);
    return () => {
      observer.disconnect();
      node.current = null;
    };
  }, []);
  return { ref, size, node };
};

const subscribeResize = (listener: () => void) => {
  window.addEventListener('resize', listener);
  return () => window.removeEventListener('resize', listener);
};

const TopScrim = () => (
  <div className="pointer-events-none absolute inset-x-0 top-0" style={{ height: TOP_SCRIM_HEIGHT, background: `linear-gradient(to bottom, ${withAlpha(Color.Black, TOP_SCRIM_ALPHA)}, transparent)` }} />
);

const FlyTopBar = ({ status, statusRef, children }: { status: ReactNode; statusRef: RefCallback<HTMLElement> | undefined; children: ReactNode }) => (
  <div className="flex flex-col gap-2 px-3 pb-2" style={{ paddingTop: `calc(env(safe-area-inset-top, 0px) + ${AircastSpace.s2}px)` }}>
    <div ref={statusRef} className="flex flex-col gap-2">
      <LocalCompactStatus.Provider value>
        <div data-status-row className="flex w-full items-center gap-2" style={{ minHeight: STATUS_ROW_HEIGHT }}>
          {status}
        </div>
      </LocalCompactStatus.Provider>
      <LayoutEditingNotice />
    </div>
    {children}
  </div>
);

export const useScreenSize = (): Size => {
  const width = useSyncExternalStore(subscribeResize, () => window.innerWidth, () => 0);
  const height = useSyncExternalStore(subscribeResize, () => window.innerHeight, () => 0);
  return useMemo(() => Size(width, height), [width, height]);
};

export type Insets = { left: number; top: number; right: number; bottom: number };

export const NO_INSETS: Insets = { left: 0, top: 0, right: 0, bottom: 0 };

export const useSafeAreaInsets = (): Insets => {
  const [insets, setInsets] = useState<Insets>(NO_INSETS);
  useEffect(() => {
    const probe = document.createElement('div');
    probe.style.cssText = 'position:fixed;visibility:hidden;pointer-events:none;padding:env(safe-area-inset-top,0px) env(safe-area-inset-right,0px) env(safe-area-inset-bottom,0px) env(safe-area-inset-left,0px)';
    document.body.appendChild(probe);
    const read = () => {
      const style = getComputedStyle(probe);
      const next = { left: parseFloat(style.paddingLeft) || 0, top: parseFloat(style.paddingTop) || 0, right: parseFloat(style.paddingRight) || 0, bottom: parseFloat(style.paddingBottom) || 0 };
      setInsets((was) => (same(was, next) ? was : next));
    };
    read();
    window.addEventListener('resize', read);
    return () => {
      window.removeEventListener('resize', read);
      probe.remove();
    };
  }, []);
  return insets;
};

export const useStatusRowInsets = (): RefCallback<HTMLElement> => {
  const flyScreen = useContext(LocalFlyScreenState);
  const safe = useSafeAreaInsets();
  const row = useElementSize();
  useLayoutEffect(() => {
    const below = row.node.current?.getBoundingClientRect().bottom ?? safe.top;
    flyScreen.widgetInsets.value = { ...safe, top: Math.max(safe.top, below) };
    return () => {
      flyScreen.widgetInsets.value = NO_INSETS;
    };
  }, [safe, row.size]);
  return row.ref;
};

export const CUTOUT_PADDING: CSSProperties = { paddingLeft: 'env(safe-area-inset-left, 0px)', paddingRight: 'env(safe-area-inset-right, 0px)' };

export const useSwipeInsteadOfClick = (onSwipe: ((swipe: VideoSwipe) => void) | null): HTMLAttributes<HTMLElement> => {
  const latest = useUpdatedRef(onSwipe);
  const swiped = useRef(false);
  const swipes = useMemo(
    () =>
      verticalSwipe(VIDEO_SWIPE_DISTANCE, (swipe: VideoSwipe) => {
        swiped.current = true;
        latest.current?.(swipe);
      }),
    [],
  );
  return {
    ...(onSwipe === null ? {} : swipes),
    onClickCapture: (event) => {
      if (!swiped.current) return;
      swiped.current = false;
      event.stopPropagation();
      event.preventDefault();
    },
  };
};

export type FlyView = { readonly name: 'Video' | 'Map' | 'ThreeD'; readonly ordinal: number; readonly label: string; readonly icon: DrawableName };
export const FlyView = {
  Video: { name: 'Video', ordinal: 0, label: 'Video', icon: 'ic_videocam' },
  Map: { name: 'Map', ordinal: 1, label: 'Map', icon: 'ic_map' },
  ThreeD: { name: 'ThreeD', ordinal: 2, label: '3D', icon: 'ic_explore' },
} as const satisfies Record<string, FlyView>;
export const FlyViewEntries: readonly FlyView[] = [FlyView.Video, FlyView.Map, FlyView.ThreeD];

export const flyViewNamed = (name: string | null): FlyView => FlyViewEntries.find((it) => it.name === name) ?? FlyView.Video;

export const VIEWER3D_VIEW = 'view.viewer3d';

export const flyViewsOffered = (viewer3dEnabled: boolean | null, current: FlyView): readonly FlyView[] =>
  FlyViewEntries.filter((it) => it !== FlyView.ThreeD || viewer3dEnabled === true || (viewer3dEnabled === null && current === FlyView.ThreeD));

export const flyViewAllowed = (viewer3dEnabled: boolean | null, current: FlyView): FlyView =>
  current === FlyView.ThreeD && viewer3dEnabled === false ? FlyView.Map : current;

export const viewer3dEnabled = (view: Json | null): boolean | null => (view !== null && has(view, 'enabled') ? optBoolean(view, 'enabled') : null);

export const flyViewShown = (chosen: FlyView, armed: boolean, noVideoSource: boolean, nothingToShow = false): FlyView =>
  chosen === FlyView.Video && ((armed && noVideoSource) || nothingToShow) ? FlyView.Map : chosen;

export const flyViewSwapped = (view: FlyView): FlyView => (view === FlyView.Map ? FlyView.Video : FlyView.Map);

export const loadFlyView = (): FlyView => flyViewNamed(getSharedPreferences(FLY_STORE).getString(FLY_VIEW_KEY, null));

export const saveFlyView = (view: FlyView): void => getSharedPreferences(FLY_STORE).putString(FLY_VIEW_KEY, view.name);

const portraitQuery = () => window.matchMedia('(orientation: portrait)');

const subscribePortrait = (listener: () => void) => {
  const query = portraitQuery();
  query.addEventListener('change', listener);
  return () => query.removeEventListener('change', listener);
};

export const useFlyIsPortrait = (): boolean => useSyncExternalStore(subscribePortrait, () => portraitQuery().matches, () => false);

const DESKTOP_PIP = Size(320, 180);

export const thumbSize = (desktop: boolean): Size => (desktop ? DESKTOP_PIP : Size(MINIMAP_THUMB, MINIMAP_THUMB));

const thumbPlace = (size: Size, lead: number): CSSProperties => ({
  position: 'absolute',
  left: `calc(env(safe-area-inset-left, 0px) + ${lead + AircastSpace.s3}px)`,
  bottom: AircastSpace.s3,
  width: size.width,
  height: size.height,
});

type FlyScreenProps = {
  view: FlyView;
  onView: (view: FlyView) => void;
  status: ReactNode;
  video: (modifier: ModifierProps, large: boolean) => ReactNode;
  map: (modifier: ModifierProps) => ReactNode;
  keyRow: ReactNode;
  cameraSwitch: (mapIsPip: boolean, modifier: ModifierProps) => ReactNode;
  cameraControls: ReactNode;
  overlays: ReactNode;
  actions: (layout: FlyDeckLayout) => ReactNode;
  fullScreen: boolean;
  onFullScreen: () => void;
  onExitFullScreen: () => void;
};

type ShellBox = { size: Size; letterbox: Letterbox };

const NO_BOX: ShellBox = { size: ZERO_SIZE, letterbox: { start: 0, end: 0 } };

const useShellBox = (): [RefCallback<HTMLElement>, ShellBox] => {
  const [box, setBox] = useState<ShellBox>(NO_BOX);
  const ref = useCallback((element: HTMLElement | null) => {
    if (element === null) return undefined;
    const read = () => {
      const next = { size: Size(element.offsetWidth, element.offsetHeight), letterbox: letterboxOf(element) };
      setBox((was) => (same(was, next) ? was : next));
    };
    read();
    const observer = new ResizeObserver(read);
    observer.observe(element);
    window.addEventListener('resize', read);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', read);
    };
  }, []);
  return [ref, box];
};

export const FlyScreen = ({ view, onView, status, video, map, keyRow, cameraSwitch, cameraControls, overlays, actions, fullScreen, onFullScreen, onExitFullScreen }: FlyScreenProps) => {
  const videoJson = useQgcPath(VIDEO_VIEW);
  const syntheticReady = useRememberSyntheticAvailable();
  const hasPicture = optBoolean(videoJson, 'available') || syntheticReady;
  const map3dDrawn = useMap3dDrawn();
  const pictured = view === FlyView.Video || (view === FlyView.Map && map3dDrawn);
  const [rootRef, box] = useShellBox();
  const screen = useScreenSize();
  const boxHeight = box.size.height > 0 ? box.size.height : screen.height;
  const boxWidth = box.size.width > 0 ? box.size.width : screen.width;
  const stickBand = boxHeight - TOP_SCRIM_HEIGHT - STICK_FLOOR;
  const stickPad = useShownStickPad(boxHeight, stickBand);
  const shellState = useContext(LocalFlyScreenState);
  const statusRow = useStatusRowInsets();
  const layout = shellState.layout;
  const safeArea = useSafeAreaInsets();
  const direction = typeof document === 'undefined' ? 'ltr' : document.dir;
  const cutoutStart = direction === 'rtl' ? safeArea.right : safeArea.left;
  const cutoutEnd = direction === 'rtl' ? safeArea.left : safeArea.right;
  const controls = useElementSize();
  const osd = useElementSize();
  const gimbalSwitch = useValue(gimbalSwitchShown);
  const corner = useElementSize();
  const mapButtons = useElementSize();
  const stickSlot = useElementSize();
  const stickInCorner = stickPad === null && view === FlyView.Video && boxHeight >= CORNER_STICK_MIN_HEIGHT;
  const holdStick = useCallback((element: HTMLElement | null) => {
    shellState.cameraStickSlot.value = element;
    const release = stickSlot.ref(element);
    return () => {
      shellState.cameraStickSlot.value = null;
      release?.();
    };
  }, []);
  const gimbalRoom = view !== FlyView.Map && gimbalSwitch ? OSD_CONTROL_SIZE + AircastSpace.s2 : 0;
  const cornerStack = gimbalRoom + (stickInCorner && stickSlot.size.height > 0 ? corner.size.height + stickSlot.size.height + AircastSpace.s2 * 2 : corner.size.width > 0 ? corner.size.height + AircastSpace.s2 : 0);
  const switchEnd = view === FlyView.Map ? MAP_LAYERS_CLEARANCE : AircastSpace.s3;
  const osdEndRoom = stickPad !== null || view === FlyView.Map ? MAP_LAYERS_CLEARANCE : switchEnd + (corner.size.width > 0 ? corner.size.width + AircastSpace.s2 : 0);
  const insets = stickInsets(stickPad, fullScreen, box.letterbox, cutoutStart, cutoutEnd, controls.size.width);
  const ornamentEnd = insets.ornamentEnd;
  useEffect(() => {
    shellState.mapOrnamentEnd.value = ornamentEnd;
  });
  useEffect(() => () => {
    shellState.mapOrnamentEnd.value = null;
  }, []);
  const editing = useValue(layout.editing);
  const settingsOver = useValue(useContext(LocalAppNavigation).settingsOpen);
  const deciding = useValue(shellState.guidedPanels) > 0;
  const obstacles = useValue(shellState.obstacles);
  const hasVehicle = useHasVehicle();
  const armed = useArmed();
  const lostFlight = useValue(lostVehicle) !== null;
  const offlineSheet = useValue(shellState.offlineSheetOpen);
  const mapAboveOsd = view === FlyView.Map && stickPad === null && hasVehicle && !deciding ? osd.size.height : 0;
  const cornerRoom = stickPad !== null ? MAP_LAYERS_CLEARANCE : mapAboveOsd;
  const mapButtonsBottom = stickPad !== null ? AircastSpace.s3 : MAP_SCALE_CLEARANCE + mapAboveOsd;
  const switchBottom = view === FlyView.Map ? mapButtonsBottom : AircastSpace.s3 + gimbalRoom;
  const syntheticChosen = useRememberSyntheticChosen();
  const overlaysBottom = useValue(shellState.mapInsets).top;
  const cameraJson = useQgcPath(CAMERA_VIEW);
  const aboveShutter = useMemo(() => railAboveShutter(cameraReading(cameraJson)), [cameraJson]);
  const [layers, setLayers] = useState(false);
  useEffect(() => setLayers(false), [view]);

  const mapIsPip = view === FlyView.Video;
  const videoIsPip = view === FlyView.Map;
  const videoShown = view !== FlyView.ThreeD && (view !== FlyView.Map || hasPicture);
  const thumbShown = mapIsPip || (videoIsPip && hasPicture);
  const hiddenWidgets = useValue(layout.hidden);
  const minimapCollapsed = mapIsPip && hiddenWidgets.has(MINIMAP_KEY);
  const pipThumb = thumbSize(useDesktopLayout());
  const syntheticPip = videoIsPip && syntheticReady && videoReading(videoJson)?.decoding !== true;
  const toggleThumb = minimapCollapsed || syntheticPip;
  const thumb = toggleThumb ? Size(OSD_CONTROL_SIZE, OSD_CONTROL_SIZE) : pipThumb;
  const thumbLead = toggleThumb ? 0 : BOTTOM_LEAD;
  const thumbStyle = thumbPlace(thumb, thumbLead);
  const thumbFrame = rectAt(Offset(safeArea.left + AircastSpace.s3 + thumbLead, box.size.height - (AircastSpace.s3 + thumb.height)), thumb);
  const pipNow = videoIsPip && !fullScreen;
  const holdThrough = useHoldThroughToEditLayout();
  const pipJiggle = useJiggleAngle(mapIsPip ? MINIMAP_KEY : 'pip', editing && thumbShown && !fullScreen, thumb);
  const videoTarget = pipNow ? thumbFrame : Rect(0, 0, box.size.width, box.size.height);
  const frame = useRememberVideoFrame(videoTarget, false);
  const cameras = useRememberCameraStepper();
  const dragAims = useRememberDragAims();
  const syntheticAims = useRememberSyntheticAims();
  const latest = useUpdatedRef({ pipNow, onExitFullScreen, onFullScreen, fullScreen, dragAims, syntheticAims, cameras });
  const videoGestureProps = useMemo(() => {
    const handlers: VideoGestureHandlers = {
      owned: () => false,
      claimsSwipe: (moved: Offset) => sideways(moved) && !latest.current.dragAims && !(latest.current.syntheticAims && !latest.current.pipNow),
      onTap: () => undefined,
      onDoubleTap: () => (latest.current.fullScreen ? latest.current.onExitFullScreen() : latest.current.onFullScreen()),
      onSwipe: (moved: Offset) => {
        const step = cameraStep(videoSwipe(moved, VIDEO_SWIPE_DISTANCE));
        if (step !== null) latest.current.cameras.step(step);
      },
      onHold: () => false,
      onHoldDrag: () => undefined,
      onHoldEnd: () => undefined,
    };
    return videoGestures(handlers);
  }, []);
  const videoMessage = videoShown && !videoIsPip && videoReading(videoJson)?.decoding !== true;
  const guideMessage = !hasVehicle && (view === FlyView.Map || (videoMessage && looksForAircraft(hasVehicle, videoReading(videoJson), syntheticChosen)));
  const besideThumb = thumbShown ? thumb.width : 0;
  const showMap = () => onView(FlyView.Map);
  const showVideo = () => onView(FlyView.Video);

  const sight = (
    <>
      <HudLadders shellWidth={boxWidth} shellHeight={boxHeight} />
      <HudReadings
        startInset={ACTION_RAIL_CLEARANCE + AircastSpace.s3}
        startBelow={fullScreen ? 0 : overlaysBottom + AircastSpace.s2}
        style={{
          zIndex: layer(2),
          top: `calc(env(safe-area-inset-top, 0px) + ${STATUS_ROW_HEIGHT + AircastSpace.s6}px)`,
          bottom: `calc(50% + ${RAIL_SHUTTER_ROOM / 2 + aboveShutter + AircastSpace.s4}px)`,
        }}
      />
    </>
  );
  const deckLayer = (
    <LocalFlyOsd.Provider value={pictured}>
      <div
        className={`absolute inset-0 ${PASS_THROUGH}`}
        style={{ zIndex: layer(fullScreen ? FULL_SCREEN_LAYER + 0.5 : deciding ? DECISION_LAYER : 2), opacity: settingsOver ? 0 : 1, ...CUTOUT_PADDING, paddingTop: STATUS_ROW_HEIGHT + AircastSpace.s4 }}
      >
        <div className="relative size-full">
          {actions(FlyDeckLayout.Rail)}
          {hasVehicle && !deciding ? (
            <div
              className="absolute inset-x-0 flex justify-center"
              style={{ bottom: (view === FlyView.Video || fullScreen ? AircastSpace.s3 : Math.max(PAYLOAD_STRIP_CLEARANCE, osd.size.height + AircastSpace.s2)) + (stickPad === null ? 0 : Math.max(stickBand, stickPad) + STICK_FLOOR) }}
            >
              <LayoutWidget widgetKey="payload" movable={false} hideable={false} live={armed}>
                <PayloadRailStrip />
              </LayoutWidget>
            </div>
          ) : null}
        </div>
      </div>
    </LocalFlyOsd.Provider>
  );

  return (
    <div ref={rootRef} className="relative size-full overflow-hidden" {...holdThrough}>
      <LocalCompactHud.Provider value={compactHud(boxHeight)}>
        {fullScreen ? null : (
          <>
            <div className={`absolute inset-0 ${PASS_THROUGH}`} style={{ zIndex: layer(1), opacity: settingsOver ? 0 : 1, ...CUTOUT_PADDING }}>
              <div className="relative size-full">
                <LocalFlyOsd.Provider value={pictured}>
                  <FlyChromeLayout
                    top={
                      <div className="relative">
                        <FlyTopBar status={status} statusRef={statusRow}>
                          <LayoutWidget widgetKey="headingTape" hideable={false} className="self-center">
                            <HeadingTape />
                          </LayoutWidget>
                          {view === FlyView.Video ? <ApproximateAimNote /> : null}
                          <TrafficBanner />
                          <UpdateBanner />
                          <div className="flex items-center gap-2">
                            {keyRow}
                          </div>
                        </FlyTopBar>
                        <div className="pointer-events-none absolute inset-x-0" style={{ top: 'env(safe-area-inset-top, 0px)' }}>
                          <LayoutWidget widgetKey={BATTERY_BAR_KEY} movable={false} live hideable={false}>
                            <BatteryReturnBar />
                          </LayoutWidget>
                        </div>
                      </div>
                    }
                    overlays={
                      <div className="flex w-full flex-col items-start gap-2" style={{ paddingInlineStart: insets.overlaysStart, paddingInlineEnd: insets.overlaysEnd }}>
                        <LocalAvoidedByVideoMessage.Provider value>{overlays}</LocalAvoidedByVideoMessage.Provider>
                      </div>
                    }
                    bottom={<div style={{ height: cornerRoom }} />}
                    overlaysAboveBottom
                    message={
                      lostFlight ? null : guideMessage ? (
                        offlineSheet ? null : (
                          <OfflineGuideCard inVideo={videoMessage} />
                        )
                      ) : videoMessage ? (
                        <div className={`size-full ${osdShadow()}`}>
                          <FlyNoVideoMessage />
                        </div>
                      ) : null
                    }
                    messageAboveBottom={false}
                    obstacles={[...obstacles.values()]}
                    onInsets={(next) => {
                      if (!same(next, shellState.mapInsets.value)) shellState.mapInsets.value = next;
                    }}
                  />
                  {stickPad === null && !deciding ? (
                    <div
                      className="absolute inset-y-0 end-0 flex items-center"
                      style={{
                        zIndex: layer(1),
                        paddingTop: `calc(env(safe-area-inset-top, 0px) + ${RAIL_CLEARANCE + AircastSpace.s3}px)`,
                        paddingBottom: view === FlyView.Map ? railAboveMapButtons(mapButtonsBottom, mapButtons.size.height) : RAIL_CLEARANCE,
                        paddingInlineEnd: AircastSpace.s3,
                      }}
                    >
                      <LocalAvoidedByVideoMessage.Provider value>
                        <div className="flex items-center justify-center" style={{ minWidth: RAIL_SHUTTER_ROOM }}>
                          {cameraControls}
                        </div>
                      </LocalAvoidedByVideoMessage.Provider>
                    </div>
                  ) : null}
                  <div
                    ref={corner.ref}
                    className="absolute end-0 bottom-0 flex items-center justify-end"
                    style={{ zIndex: layer(1), marginInlineEnd: switchEnd, marginBottom: switchBottom, minHeight: MAP_LAYERS_SIZE }}
                  >
                    <LocalAvoidedByVideoMessage.Provider value>{cameraSwitch(mapIsPip, {})}</LocalAvoidedByVideoMessage.Provider>
                  </div>
                  {stickInCorner ? (
                    <div
                      ref={holdStick}
                      className="absolute end-0 bottom-0"
                      style={{ zIndex: layer(1), marginInlineEnd: AircastSpace.s3, marginBottom: switchBottom + corner.size.height + AircastSpace.s2 }}
                    />
                  ) : null}
                  {hasVehicle && !deciding && view === FlyView.Video ? (
                    sight
                  ) : hasVehicle && !deciding ? (
                    <div
                      ref={osd.ref}
                      className={`absolute inset-x-0 bottom-0 flex items-end gap-6 ${osdShadow()} ${PASS_THROUGH}`}
                      style={{ zIndex: layer(2), paddingInlineStart: BOTTOM_LEAD + besideThumb + AircastSpace.s3 * 2, paddingInlineEnd: osdEndRoom, paddingBottom: AircastSpace.s3 }}
                    >
                      <LayoutWidget widgetKey="telemetry" movable={false} hideable={false} className="shrink-0">
                        <LocalFlyOsd.Provider value>
                          <TelemetryRow valuesShown chooser={false} compact stacked className={pictured ? undefined : 'rounded-shape-md bg-black/55 px-3 py-1.5'} />
                        </LocalFlyOsd.Provider>
                      </LayoutWidget>
                    </div>
                  ) : null}
                </LocalFlyOsd.Provider>
                {view === FlyView.Map ? (
                  <LocalFlyOsd.Provider value={pictured}>
                    <div className="absolute end-0 bottom-0" style={{ marginInlineEnd: AircastSpace.s3, marginBottom: mapButtonsBottom }}>
                      <LayoutWidget widgetKey="mapLayers" hideable={false}>
                        <div ref={mapButtons.ref} className="flex flex-col gap-2">
                          <FollowButton />
                          <Map3dToggle />
                          <MapFab color={md('surfaceContainerHigh')} contentColor={md('onSurface')} label="Map layers" onClick={() => setLayers(true)}>
                            <Drawable name="ic_layers" />
                          </MapFab>
                        </div>
                      </LayoutWidget>
                    </div>
                  </LocalFlyOsd.Provider>
                ) : null}
                {view === FlyView.Map && layers ? <MapLayersSheet onDismiss={() => setLayers(false)} /> : null}
              </div>
            </div>
          </>
        )}
        {fullScreen && hasVehicle && view === FlyView.Video ? (
          <LocalFlyOsd.Provider value>
            <div className={`absolute inset-0 ${PASS_THROUGH}`} style={{ zIndex: layer(FULL_SCREEN_LAYER + 0.5), ...CUTOUT_PADDING }}>
              <TopScrim />
              <div className="relative size-full">
                <FlyTopBar status={status} statusRef={undefined}>
                  <HeadingTape className="self-center" />
                </FlyTopBar>
                {sight}
              </div>
            </div>
          </LocalFlyOsd.Provider>
        ) : null}
        {deckLayer}
        {stickPad !== null ? (
          <LocalFlyOsd.Provider value={pictured}>
            <LocalAvoidedByVideoMessage.Provider value>
              <div
                className={`absolute inset-x-0 bottom-0 flex justify-center ${PASS_THROUGH}`}
                style={{
                  zIndex: layer(deciding ? DECISION_LAYER - 0.5 : STICK_LAYER),
                  opacity: settingsOver ? 0 : 1,
                  paddingInlineStart: insets.rowStart,
                  paddingInlineEnd: insets.rowEnd,
                  paddingBottom: STICK_FLOOR,
                  height: Math.max(stickBand, stickPad) + STICK_FLOOR,
                }}
              >
                <VirtualJoystick
                  pad={stickPad}
                  className="h-full"
                  trailing={
                    fullScreen ? null : (
                      <div className="flex h-full">
                        <div style={{ width: insets.trailingGap }} />
                        <div ref={controls.ref} className="flex h-full items-center justify-center" style={{ minWidth: RAIL_SHUTTER_ROOM }}>
                          {cameraControls}
                        </div>
                      </div>
                    )
                  }
                />
              </div>
            </LocalAvoidedByVideoMessage.Provider>
          </LocalFlyOsd.Provider>
        ) : null}
        {view === FlyView.ThreeD ? <Viewer3DPane className="absolute inset-0" /> : null}
        {view !== FlyView.ThreeD && !minimapCollapsed
          ? map(
              mapIsPip
                ? {
                    className: 'absolute overflow-hidden rounded-shape-md',
                    style: { ...thumbStyle, zIndex: layer(PIP_LAYER), border: `2px solid ${md('onSurface')}`, ...pipJiggle },
                    'aria-hidden': fullScreen || undefined,
                  }
                : { className: 'absolute inset-0', 'aria-hidden': fullScreen || undefined },
            )
          : null}
        <AnimatedVisibility visible={videoShown} from="bottom" className={`absolute inset-0 ${PASS_THROUGH}`} style={{ zIndex: fullScreen ? layer(FULL_SCREEN_LAYER) : videoIsPip ? layer(PIP_LAYER) : 0 }}>
          {video(
            {
              ...videoGestureProps,
              className: `pointer-events-auto overflow-hidden bg-black ${pipNow ? 'rounded-shape-md' : ''}`,
              style: { ...videoFrame(videoTarget, () => frame.value, cameras.nudge), touchAction: 'none', ...(pipNow ? pipJiggle : {}), ...(pipNow || fullScreen || cornerStack === 0 ? {} : ({ [VIDEO_CORNER_ROOM]: `${cornerStack}px` } as CSSProperties)) },
            },
            !pipNow,
          )}
        </AnimatedVisibility>
        {fullScreen ? null : (
          <>
            <TopScrim />
            {minimapCollapsed ? (
              <div className="absolute" style={{ ...thumbStyle, zIndex: layer(2) }}>
                <MinimapWidget editing={editing}>
                  <div inert={editing}>
                    <Surface
                      onClick={showMap}
                      shape="full"
                      color={Color.Transparent}
                      contentColor={aircast('outdoorForeground')}
                      label="Map"
                      className="grid place-items-center"
                      style={{ width: thumb.width, height: thumb.height }}
                    >
                      <Drawable name="ic_map" className={osdShadow()} />
                    </Surface>
                  </div>
                </MinimapWidget>
              </div>
            ) : mapIsPip ? (
              <div className="absolute" style={{ ...thumbStyle, zIndex: layer(2) }}>
                <LayoutWidget widgetKey={MINIMAP_KEY} movable={false} name={MINIMAP_NAME}>
                  <div
                    role="button"
                    tabIndex={0}
                    aria-label="Map"
                    aria-description="Show the map"
                    inert={editing}
                    onClick={showMap}
                    onKeyDown={activate(showMap)}
                    className="cursor-pointer overflow-hidden rounded-shape-md"
                    style={{ width: thumb.width, height: thumb.height }}
                  />
                </LayoutWidget>
              </div>
            ) : null}
            {videoIsPip && hasPicture ? (
              <div
                role="button"
                tabIndex={0}
                aria-label="Video"
                aria-description="Show the video"
                inert={editing}
                onClick={showVideo}
                onKeyDown={activate(showVideo)}
                className="absolute cursor-pointer overflow-hidden rounded-shape-md"
                style={{ ...thumbStyle, zIndex: layer(3), border: `2px solid ${md('onSurface')}`, ...pipJiggle }}
              />
            ) : null}
          </>
        )}
      </LocalCompactHud.Provider>
    </div>
  );
};

export const FleetCard = () => {
  const flyScreen = useContext(LocalFlyScreenState);
  const deciding = useValue(flyScreen.guidedPanels) > 0;
  const screen = useScreenSize();
  const backdrop = useOsdBackdrop(md('surfaceContainer'));
  if (Math.min(screen.width, screen.height) < FLEET_CARD_MIN_SCREEN_DP || deciding) return null;
  return (
    <Surface shape="large" color={backdrop} contentColor={md('onSurface')} className="overflow-hidden" style={{ maxWidth: FLEET_CARD_MAX_WIDTH, maxHeight: FLEET_CARD_MAX_HEIGHT }}>
      <FleetPanel className="max-h-[320px] overflow-y-auto py-2" />
    </Surface>
  );
};

const MAP_LAYERS_SIZE = 48;
export const MAP_LAYERS_CLEARANCE = MAP_LAYERS_SIZE + AircastSpace.s3 + AircastSpace.s2;

export const railAboveMapButtons = (buttonsBottom: number, buttonsHeight: number): number => buttonsBottom + buttonsHeight + AircastSpace.s2 + AircastSpace.s3;
const FLEET_CARD_MAX_WIDTH = 440;
const FLEET_CARD_MAX_HEIGHT = 320;
const FLEET_CARD_MIN_SCREEN_DP = 600;

const activeAt = (json: Json | null): TrackPoint | null => {
  const active = vehicleChoices(json).active;
  return active === null || !isPlottable(active.latitude, active.longitude) ? null : { latitude: active.latitude, longitude: active.longitude };
};

const FollowButton = () => {
  const following = useValue(mapFollowing);
  const shown = useValue(mapWindow);
  const vehicle = useQgcDerived(VEHICLES_VIEW, activeAt);
  if (following || vehicle === null) return null;
  const away = vehicleOffScreen(shown, vehicle);
  const label = away === null ? FOLLOW_VEHICLE : `${FOLLOW_VEHICLE}. ${away.text}`;
  return (
    <MapFab
      color={md('primaryContainer')}
      contentColor={md('onPrimaryContainer')}
      label={label}
      onClick={() => {
        mapFollowing.value = true;
      }}
    >
      {away === null ? <Drawable name="ic_my_location" /> : <Drawable name="ic_navigation" style={{ transform: `rotate(${away.bearing}deg)` }} />}
    </MapFab>
  );
};

const MapFab = ({ color, contentColor, label, onClick, children }: { color: string; contentColor: string; label: string; onClick: () => void; children: ReactNode }) => (
  <Surface
    onClick={onClick}
    shape="full"
    color={useOsdBackdrop(color)}
    contentColor={useOsdTint(contentColor, aircast('outdoorForeground'))}
    label={label}
    title={label}
    className={`grid place-items-center ${useContext(LocalFlyOsd) ? osdShadow() : ''}`}
    style={{ width: MAP_LAYERS_SIZE, height: MAP_LAYERS_SIZE }}
  >
    {children}
  </Surface>
);

const Map3dToggle = () => {
  const shown = useMap3dShown();
  const offered = useQgcDerived(SYNTHETIC_VIEW, nativeChosen);
  return offered ? (
    <MapFab color={md('surfaceContainerHigh')} contentColor={md('onSurface')} label={map3dToggleLabel(shown)} onClick={() => Map3d.show(!shown)}>
      <Text typography="labelLarge">{map3dToggleText(shown)}</Text>
    </MapFab>
  ) : null;
};

export type MapInsets = { top: number; bottom: number; ornaments: number };
export const MapInsets = (fields: { top: number; bottom: number; ornaments?: number }): MapInsets => ({ ornaments: fields.bottom, ...fields });

export type Letterbox = { start: number; end: number };
export const Letterbox = (fields: Letterbox): Letterbox => ({ ...fields });

export type StickInsets = { rowStart: number; rowEnd: number; trailingGap: number; overlaysStart: number; overlaysEnd: number; ornamentEnd: number | null };

export const stickInsets = (pad: number | null, fullScreen: boolean, letterbox: Letterbox, cutoutStart: number, cutoutEnd: number, controlsWidth: number): StickInsets => {
  const startGap = letterbox.start + cutoutStart;
  const endGap = letterbox.end + cutoutEnd;
  const edge = cutoutEnd + AircastSpace.s3;
  const inset = fullScreen ? Math.max(startGap, endGap) + AircastSpace.s3 : Math.max(startGap + ACTION_RAIL_CLEARANCE, endGap + AircastSpace.s3 + controlsWidth + AircastSpace.s2);
  return {
    rowStart: inset - letterbox.start,
    rowEnd: fullScreen ? inset - letterbox.end : edge,
    trailingGap: coerceAtLeast(inset - letterbox.end - edge - controlsWidth, AircastSpace.s2),
    overlaysStart: pad === null ? ACTION_RAIL_CLEARANCE : inset - startGap + pad + AircastSpace.s3,
    overlaysEnd: pad === null ? AircastSpace.s3 : inset - endGap + pad + AircastSpace.s3,
    ornamentEnd: pad === null || fullScreen ? null : inset - letterbox.end + pad + AircastSpace.s2,
  };
};

const letterboxOf = (element: HTMLElement): Letterbox => {
  const rect = element.getBoundingClientRect();
  const left = rect.left;
  const right = window.innerWidth - rect.right;
  const ltr = getComputedStyle(element).direction !== 'rtl';
  return Letterbox({ start: coerceAtLeast(ltr ? left : right, 0), end: coerceAtLeast(ltr ? right : left, 0) });
};

export const LocalAvoidedByVideoMessage = createContext(false);

type FlyChromeLayoutProps = {
  top: ReactNode;
  overlays: ReactNode;
  bottom: ReactNode;
  overlaysAboveBottom: boolean;
  message: ReactNode | null;
  messageAboveBottom: boolean;
  obstacles: readonly Rect[];
  onInsets: (insets: MapInsets) => void;
};

const FlyChromeLayout = ({ top, overlays, bottom, overlaysAboveBottom, message, messageAboveBottom, obstacles, onInsets }: FlyChromeLayoutProps) => {
  const root = useElementSize();
  const topSlot = useElementSize();
  const overlaySlot = useElementSize();
  const bottomSlot = useElementSize();
  const width = root.size.width;
  const height = root.size.height;
  const topHeight = topSlot.size.height;
  const bottomHeight = bottomSlot.size.height;
  const overlayHeight = overlaySlot.size.height;
  const overlayRoom = coerceAtLeast(height - topHeight - (overlaysAboveBottom ? bottomHeight : 0), 0);
  const origin = root.node.current?.getBoundingClientRect() ?? null;
  const free = Rect(AircastSpace.s3, topHeight, width - AircastSpace.s3, height - AircastSpace.s3 - (messageAboveBottom ? bottomHeight : 0));
  const room = message === null ? null : messageRegion(free, obstacles.map((it) => translated(it, -(origin?.left ?? 0), -(origin?.top ?? 0))), AircastSpace.s3, NO_VIDEO_PILL_WIDTH, NO_VIDEO_PILL_HEIGHT);
  const region = room === null ? null : centredRegion(room, width / 2, NO_VIDEO_PILL_WIDTH);
  const latestInsets = useUpdatedRef(onInsets);
  useEffect(() => {
    latestInsets.current(MapInsets({ top: topHeight + overlayHeight, bottom: bottomHeight }));
  }, [topHeight, overlayHeight, bottomHeight]);
  return (
    <div ref={root.ref} className="relative size-full">
      {region !== null ? (
        <div className="absolute" style={{ left: region.left, top: region.top, width: region.width, height: region.height }}>
          {message}
        </div>
      ) : null}
      <div ref={topSlot.ref} className="absolute inset-x-0 top-0">
        {top}
      </div>
      <div ref={overlaySlot.ref} className="absolute inset-x-0" style={{ top: topHeight, maxHeight: overlayRoom }}>
        {overlays}
      </div>
      <div ref={bottomSlot.ref} className="absolute end-0 bottom-0">
        {bottom}
      </div>
    </div>
  );
};

export const useAvoidedByVideoMessage = (key: string): RefCallback<HTMLElement> => {
  const flyScreen = useContext(LocalFlyScreenState);
  const node = useRef<HTMLElement | null>(null);
  const measure = useCallback(() => {
    const element = node.current;
    if (element === null) return;
    const box = element.getBoundingClientRect();
    const next = Rect(box.left, box.top, box.right, box.bottom);
    const was = flyScreen.obstacles.get(key);
    if (was === undefined || !same(was, next)) flyScreen.obstacles.set(key, next);
  }, [key]);
  useEffect(() => () => flyScreen.obstacles.remove(key), [key]);
  useLayoutEffect(measure);
  return useCallback(
    (element: HTMLElement | null) => {
      node.current = element;
      if (element === null) return undefined;
      measure();
      const observer = new ResizeObserver(measure);
      observer.observe(element);
      window.addEventListener('resize', measure);
      return () => {
        observer.disconnect();
        window.removeEventListener('resize', measure);
        node.current = null;
      };
    },
    [measure],
  );
};

export const FlyMenuButton = () => {
  const flyScreen = useContext(LocalFlyScreenState);
  const backdrop = useOsdBackdrop(OSD_SCRIM);
  return (
    <Surface
      onClick={() => {
        flyScreen.requestedSheet.value = MENU_SHEET;
      }}
      shape="full"
      color={backdrop}
      contentColor={aircast('outdoorForeground')}
      label={FLY_MENU}
      className={`grid place-items-center ${osdShadow()}`}
      style={{ width: STATUS_ROW_HEIGHT + AircastSpace.s2, height: STATUS_ROW_HEIGHT + AircastSpace.s2 }}
    >
      <MaterialIcon name="more_vert" filled size={24} />
    </Surface>
  );
};
