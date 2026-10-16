import { type ReactNode, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { settingControl, useQgcPath, useQgcValue } from '@/bridge/QgcState';
import { useUpdatedRef } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { Surface, Text, type Typography } from '@/m3';
import { MAP_SCALE_CLEARANCE } from '@/map/FlyMap';
import { MapLayersSheet } from '@/map/MapLayersSheet';
import { Viewer3DPane } from '@/map/Viewer3D';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { AircastSpace, Color, aircast, withAlpha } from '@/theme/Colors';
import { OsdCompassDial } from '@/ui/AttitudeInstrument';
import { CameraSwitch, useRememberGimbalDrags } from '@/ui/CameraSwitch';
import { FlyDeckLayout } from '@/ui/FlyDeck';
import { LocalFlyScreenState } from '@/ui/FlyScreenState';
import {
  A11yActions,
  AnimatedVisibility,
  FlyView,
  MapInsets,
  type ModifierProps,
  Offset,
  PASS_THROUGH,
  Rect,
  Size,
  VIEWER3D_VIEW,
  flyViewAllowed,
  flyViewsOffered,
  minus,
  plus,
  rectAt,
  rectCenter,
  rectTopLeft,
  useElementSize,
  useSafeAreaInsets,
  useScreenSize,
  useSwipeInsteadOfClick,
  viewer3dEnabled,
} from '@/ui/FlyShell';
import { useHasVehicle } from '@/ui/FlyState';
import { LocalFlyOsd, osdShadow } from '@/ui/Osd';
import { LayoutWidget } from '@/ui/OverlayLayout';
import { BatteryReturnBar } from '@/ui/StatusStrip';
import { useRememberSyntheticAims } from '@/ui/SyntheticView';
import { TrafficBanner } from '@/ui/TrafficBanner';
import { LocalDeckTelemetry, TelemetryRow } from '@/ui/VehicleUi';
import {
  PipCorner,
  VIDEO_SWIPE_DISTANCE,
  type VideoGestureHandlers,
  VideoSwipe,
  cameraStep,
  growingSwipe,
  hidingSwipe,
  sideways,
  useRememberCameraStepper,
  useRememberVideoFrame,
  verticalSwipe,
  videoFrame,
  videoGestures,
  videoSwipe,
} from '@/ui/VideoGestures';
import { FlyNoVideoMessage } from '@/ui/VideoSurface';
import { VIDEO_VIEW, type VideoReading, videoReading } from '@/ui/VideoView';
import { VirtualJoystick, useShownStickPad } from '@/ui/VirtualJoystick';

const VIDEO_ASPECT = 16 / 9;
const MAP_BUTTON_SCRIM_ALPHA = 0.55;
const PORTRAIT_PIP_WIDTH = 160;
const PORTRAIT_PIP_HEIGHT = 90;
const PORTRAIT_DIAL_SIZE = 72;
const PORTRAIT_BAR_HEIGHT = 64;
const PORTRAIT_OSD_DIAL = 48;
const PLACEHOLDER_ALPHA = 0.6;
const SCRIM_ALPHA = 0.7;
const PIP_BORDER_ALPHA = 0.5;
const DECK_SCRIM_START = 0.4;
const MAP_BUTTON_SIZE = 40;
const MAP_ATTRIBUTION_CLEARANCE = 28;

export const portraitSplit = (view: FlyView, hasVideo: boolean): boolean => view === FlyView.Video && hasVideo;

export const portraitShowsCamera = (reading: VideoReading | null): boolean => reading !== null && (reading.available || (!reading.streamEnabled && reading.sourceChosen));

export const portraitVideoThumbnail = (split: boolean, reading: VideoReading | null): boolean => !split && reading?.available === true;

type FlyPortraitProps = {
  view: FlyView;
  onView: (view: FlyView) => void;
  status: ReactNode;
  video: (modifier: ModifierProps, large: boolean) => ReactNode;
  map: (modifier: ModifierProps) => ReactNode;
  keyRow: ReactNode;
  rail: (split: boolean) => ReactNode;
  overlays: ReactNode;
  actions: (layout: FlyDeckLayout) => ReactNode;
  fullScreen: boolean;
  onFullScreen: () => void;
  onExitFullScreen: () => void;
};

const ANIMATED_PADDING = 'transition-[padding] duration-300 motion-reduce:transition-none';

export const FlyPortrait = ({ view, onView, status, video, map, keyRow, rail, overlays, actions, fullScreen, onFullScreen, onExitFullScreen }: FlyPortraitProps) => {
  const videoJson = useQgcPath(VIDEO_VIEW);
  const reading = useMemo(() => videoReading(videoJson), [videoJson]);
  const hasVideo = portraitShowsCamera(reading);
  const split = portraitSplit(view, hasVideo);
  const flyScreen = useContext(LocalFlyScreenState);
  const screen = useScreenSize();
  const stickPad = useShownStickPad(screen.width);
  const chrome = useElementSize();
  const deck = useElementSize();
  const box = useElementSize();
  const safeArea = useSafeAreaInsets();
  const barTop = Math.max(safeArea.top + PORTRAIT_BAR_HEIGHT, chrome.size.height);
  const barTopPx = Math.round(barTop);
  const deckHeightPx = deck.size.height;
  const boxHeightPx = box.size.height;
  useEffect(() => {
    flyScreen.mapInsets.value = MapInsets({ top: split ? 0 : barTopPx, bottom: deckHeightPx });
  }, [split, barTopPx, deckHeightPx]);
  const screenWidth = screen.width;
  const videoHeight = screenWidth / VIDEO_ASPECT;
  const videoTop = barTop;
  const mapTop = split ? barTop + videoHeight : 0;
  const controlsTop = split ? mapTop : barTop;
  const thumbnail = portraitVideoThumbnail(split, reading);
  const tucked = useValue(flyScreen.videoTucked);
  const corner = useValue(flyScreen.pipCorner);
  const deciding = useValue(flyScreen.guidedPanels) > 0;
  const room = pipRoom(thumbnail, tucked);
  const roomAt = (at: PipCorner): number => (corner === at ? room : 0);
  const buttonsTop = AircastSpace.s3 + roomAt(PipCorner.TopEnd);
  const overlaysTop = Math.max(AircastSpace.s3 + roomAt(PipCorner.TopEnd) + MAP_BUTTON_SIZE + AircastSpace.s2, AircastSpace.s3 + roomAt(PipCorner.TopStart));
  const compassLift = roomAt(PipCorner.BottomStart);
  const railLift = roomAt(PipCorner.BottomEnd);
  const geometry = PipGeometry({
    width: screenWidth,
    pip: Size(PORTRAIT_PIP_WIDTH, PORTRAIT_PIP_HEIGHT),
    inset: AircastSpace.s3,
    pipTop: barTop + AircastSpace.s3,
    bottomStartTop: boxHeightPx - deckHeightPx - (MAP_ATTRIBUTION_CLEARANCE + PORTRAIT_PIP_HEIGHT),
    bottomEndTop: boxHeightPx - deckHeightPx - (MAP_SCALE_CLEARANCE + PORTRAIT_PIP_HEIGHT),
    split: Rect(0, videoTop, screenWidth, videoTop + videoHeight),
    full: Rect(0, 0, screenWidth, boxHeightPx > 0 ? boxHeightPx : videoTop + videoHeight),
  });
  const drag = useRef<Offset>(Offset(0, 0));
  const [pipDrag, setPipDrag] = useState<Offset>(drag.current);
  const [holding, setHolding] = useState(false);
  const target = fullScreen ? geometry.full : split ? geometry.split : geometry.pipAt(corner, pipDrag);
  const frame = useRememberVideoFrame(target, holding);
  const cameras = useRememberCameraStepper();
  const gimbalDrags = useRememberGimbalDrags();
  const syntheticAims = useRememberSyntheticAims();
  const latest = useUpdatedRef({ split, target, onExitFullScreen, fullScreen, geometry, onView, onFullScreen, gimbalDrags, syntheticAims, cameras });
  const dragTo = (next: Offset) => {
    drag.current = next;
    setPipDrag(next);
  };
  const gestures = useMemo(() => {
    const handlers: VideoGestureHandlers = {
      owned: () => !latest.current.split,
      claimsSwipe: (moved: Offset) => sideways(moved) && !latest.current.gimbalDrags && !(latest.current.syntheticAims && latest.current.split),
      onTap: () => latest.current.onView(FlyView.Video),
      onDoubleTap: () => (latest.current.fullScreen ? latest.current.onExitFullScreen() : latest.current.onFullScreen()),
      onSwipe: (moved: Offset) => {
        const swipe = videoSwipe(moved, VIDEO_SWIPE_DISTANCE);
        if (swipe === VideoSwipe.Up || swipe === VideoSwipe.Down) {
          if (latest.current.split) return;
          if (swipe === hidingSwipe(flyScreen.pipCorner.value)) {
            flyScreen.videoTucked.value = true;
            return;
          }
          latest.current.onView(FlyView.Video);
          return;
        }
        const step = cameraStep(swipe);
        if (step !== null) latest.current.cameras.step(step);
      },
      onHold: (at: Offset) => {
        setHolding(true);
        if (latest.current.split) {
          dragTo(latest.current.geometry.dragToCentre(flyScreen.pipCorner.value, plus(rectTopLeft(latest.current.target), at)));
          flyScreen.videoTucked.value = false;
          latest.current.onExitFullScreen();
          latest.current.onView(FlyView.Map);
        }
        return true;
      },
      onHoldDrag: (delta: Offset) => dragTo(plus(drag.current, delta)),
      onHoldEnd: () => {
        const now = latest.current.geometry;
        flyScreen.pipCorner.value = now.nearest(rectCenter(now.pipAt(flyScreen.pipCorner.value, drag.current)));
        dragTo(Offset(0, 0));
        setHolding(false);
      },
    };
    return videoGestures(handlers);
  }, []);
  const tuck = () => {
    flyScreen.videoTucked.value = true;
  };
  const underVideo = fullScreen || undefined;

  return (
    <div className="flex size-full flex-col" style={{ background: aircast('outdoorBackground') }}>
      <div ref={box.ref} className="relative w-full min-h-0 flex-1 overflow-hidden">
        {view === FlyView.ThreeD ? (
          <div className="absolute inset-x-0 bottom-0" style={{ top: mapTop }} aria-hidden={underVideo}>
            <Viewer3DPane className="size-full" />
          </div>
        ) : (
          map({ className: 'absolute inset-x-0 bottom-0', style: { top: mapTop }, 'aria-hidden': underVideo })
        )}
        {hasVideo ? (
          <AnimatedVisibility visible={split || !tucked} from={corner.bottom ? 'bottom' : 'top'} className={`absolute inset-0 ${PASS_THROUGH}`} style={{ zIndex: fullScreen ? FULL_SCREEN_LAYER : 0 }}>
            {video(
              {
                ...gestures,
                className: `pointer-events-auto overflow-hidden bg-black ${split ? '' : 'rounded-shape-md'}`,
                style: {
                  ...videoFrame(target, () => frame.value, cameras.nudge),
                  border: split ? undefined : `1px solid ${withAlpha(aircast('outdoorForeground'), PIP_BORDER_ALPHA)}`,
                  touchAction: 'none',
                },
              },
              split,
            )}
            <A11yActions actions={[['Show the video full screen', onFullScreen], split ? ['Make the video small', () => onView(FlyView.Map)] : ['Hide the video', tuck]]} />
          </AnimatedVisibility>
        ) : null}
        {fullScreen ? null : (
          <>
            <AnimatedVisibility visible={split} className={`absolute inset-x-0 top-0 ${PASS_THROUGH}`}>
              <div className="flex w-full flex-col" style={{ paddingTop: videoTop, height: videoTop + videoHeight }}>
                <LocalFlyOsd.Provider value>
                  <div className="flex items-center gap-2 overflow-x-auto p-2">
                    <LayoutWidget widgetKey="cameraSwitch" movable={false}>
                      <CameraSwitch splitRoom />
                    </LayoutWidget>
                    {keyRow}
                  </div>
                </LocalFlyOsd.Provider>
                <div className="relative min-h-0 w-full flex-1">
                  {reading?.decoding !== true ? (
                    <div className={`absolute inset-0 ${osdShadow()}`}>
                      <FlyNoVideoMessage />
                    </div>
                  ) : null}
                </div>
                <TelemetryOsd
                  onSmaller={() => {
                    flyScreen.videoTucked.value = false;
                    onView(FlyView.Map);
                  }}
                  onFullScreen={onFullScreen}
                />
              </div>
            </AnimatedVisibility>
            <div className={`absolute inset-0 flex flex-col ${PASS_THROUGH}`}>
              <div className="relative min-h-0 w-full flex-1">
                <div className="absolute inset-0" style={{ paddingTop: controlsTop }}>
                  <div className="relative size-full">
                    <div
                      className={`flex h-full flex-col justify-between ${ANIMATED_PADDING}`}
                      style={{ paddingInlineStart: AircastSpace.s3, paddingTop: overlaysTop, paddingInlineEnd: AircastSpace.s3, paddingBottom: MAP_ATTRIBUTION_CLEARANCE + compassLift }}
                    >
                      <div className="flex min-h-0 shrink flex-col gap-2 overflow-hidden">{overlays}</div>
                      {split ? null : (
                        <div className="pt-2">
                          <OsdCompassDial size={PORTRAIT_DIAL_SIZE} />
                        </div>
                      )}
                    </div>
                    <div
                      className={`absolute end-0 top-0 flex h-full flex-col items-end ${ANIMATED_PADDING}`}
                      style={{ paddingInlineEnd: AircastSpace.s3, paddingTop: buttonsTop, paddingBottom: MAP_SCALE_CLEARANCE + railLift }}
                    >
                      <MapButtons view={view} split={split} hasVideo={hasVideo} onView={onView} />
                      <div className="flex-1" />
                      <div className="pt-2">{rail(split)}</div>
                    </div>
                  </div>
                </div>
              </div>
              <div ref={deck.ref} className="flex w-full shrink-0 flex-col">
                {stickPad !== null && !deciding ? (
                  <div className="px-3">
                    <VirtualJoystick pad={stickPad} />
                  </div>
                ) : null}
                <div
                  className="w-full"
                  style={{ background: `linear-gradient(to bottom, transparent 0%, ${withAlpha(Color.Black, SCRIM_ALPHA)} ${DECK_SCRIM_START * 100}%)`, paddingBottom: 'env(safe-area-inset-bottom, 0px)' }}
                >
                  <LocalDeckTelemetry.Provider value={!split}>{actions(FlyDeckLayout.Bottom)}</LocalDeckTelemetry.Provider>
                </div>
              </div>
            </div>
            <AnimatedVisibility
              visible={thumbnail && tucked}
              from={corner.bottom ? 'bottom' : 'top'}
              className={`absolute ${cornerAlignment(corner)}`}
              style={{
                marginTop: corner.bottom ? 0 : barTop + AircastSpace.s3,
                marginBottom: corner.bottom ? deckHeightPx + (corner.start ? MAP_ATTRIBUTION_CLEARANCE : MAP_SCALE_CLEARANCE) : 0,
                marginInlineStart: AircastSpace.s3,
                marginInlineEnd: AircastSpace.s3,
              }}
            >
              <VideoTab
                showSwipe={growingSwipe(corner)}
                onShow={() => {
                  flyScreen.videoTucked.value = false;
                }}
              />
            </AnimatedVisibility>
            <div ref={chrome.ref} className={`absolute inset-x-0 top-0 flex w-full flex-col ${PASS_THROUGH}`}>
              <div className="relative w-full" style={{ background: `linear-gradient(to bottom, ${withAlpha(Color.Black, SCRIM_ALPHA)}, transparent)`, paddingTop: 'env(safe-area-inset-top, 0px)' }}>
                <div className="flex w-full items-center gap-2 px-3" style={{ height: PORTRAIT_BAR_HEIGHT }}>
                  {status}
                </div>
                <div className="pointer-events-none absolute inset-x-0" style={{ top: 'env(safe-area-inset-top, 0px)' }}>
                  <BatteryReturnBar />
                </div>
              </div>
              <TrafficBanner className="px-3 pb-2" />
            </div>
          </>
        )}
        {fullScreen && stickPad !== null ? (
          <div
            className={`absolute inset-x-0 bottom-0 flex justify-center px-3 pt-3 ${PASS_THROUGH}`}
            style={{ zIndex: FULL_SCREEN_LAYER + 1, paddingBottom: `calc(env(safe-area-inset-bottom, 0px) + ${AircastSpace.s3}px)` }}
          >
            <VirtualJoystick pad={stickPad} />
          </div>
        ) : null}
      </div>
    </div>
  );
};

export const VIDEO_TAB_HEIGHT = 36;
const FULL_SCREEN_LAYER = 1;

export const cornerAlignment = (corner: PipCorner): string => {
  if (corner === PipCorner.TopStart) return 'top-0 start-0';
  if (corner === PipCorner.TopEnd) return 'top-0 end-0';
  return corner === PipCorner.BottomStart ? 'bottom-0 start-0' : 'bottom-0 end-0';
};

export const pipRoom = (thumbnail: boolean, tucked: boolean): number => {
  if (!thumbnail) return 0;
  return tucked ? VIDEO_TAB_HEIGHT + AircastSpace.s3 : PORTRAIT_PIP_HEIGHT + AircastSpace.s3;
};

export type PipGeometryFields = { width: number; pip: Size; inset: number; pipTop: number; bottomStartTop: number; bottomEndTop: number; split: Rect; full: Rect };

export type PipGeometry = PipGeometryFields & {
  anchor: (corner: PipCorner) => Offset;
  pipAt: (corner: PipCorner, drag: Offset) => Rect;
  dragToCentre: (corner: PipCorner, finger: Offset) => Offset;
  nearest: (centre: Offset) => PipCorner;
};

const CORNERS: readonly PipCorner[] = [PipCorner.TopStart, PipCorner.TopEnd, PipCorner.BottomStart, PipCorner.BottomEnd];

export const PipGeometry = (fields: PipGeometryFields): PipGeometry => {
  const top = (corner: PipCorner): number => {
    if (!corner.bottom) return fields.pipTop;
    return corner.start ? fields.bottomStartTop : fields.bottomEndTop;
  };
  const anchor = (corner: PipCorner): Offset => Offset(corner.start ? fields.inset : fields.width - fields.inset - fields.pip.width, top(corner));
  return {
    ...fields,
    anchor,
    pipAt: (corner, drag) => rectAt(plus(anchor(corner), drag), fields.pip),
    dragToCentre: (corner, finger) => minus(minus(finger, anchor(corner)), Offset(fields.pip.width / 2, fields.pip.height / 2)),
    nearest: (centre) => {
      const start = centre.x < fields.width / 2;
      const bottomTop = top(start ? PipCorner.BottomStart : PipCorner.BottomEnd);
      const bottom = centre.y > (fields.pipTop + bottomTop + fields.pip.height) / 2;
      return CORNERS.find((it) => it.start === start && it.bottom === bottom) ?? PipCorner.TopStart;
    },
  };
};

const VideoTab = ({ showSwipe, onShow }: { showSwipe: VideoSwipe; onShow: () => void }) => {
  const swipes = useSwipeInsteadOfClick((swipe) => {
    if (swipe === showSwipe) onShow();
  });
  return (
    <div {...swipes} className="pointer-events-auto" style={{ touchAction: 'none' }}>
      <Surface
        onClick={onShow}
        shape="full"
        color={withAlpha(Color.Black, SCRIM_ALPHA)}
        contentColor={aircast('outdoorForeground')}
        label="Show the video"
        className="flex items-center gap-1 px-3"
        style={{ height: VIDEO_TAB_HEIGHT }}
      >
        <Drawable name="ic_videocam" size={18} />
        <Text typography="labelLarge">Video</Text>
      </Surface>
    </div>
  );
};

const TelemetryOsd = ({ onSmaller, onFullScreen }: { onSmaller: () => void; onFullScreen: () => void }) => {
  const latest = useUpdatedRef({ onSmaller, onFullScreen });
  const swipes = useMemo(() => verticalSwipe(VIDEO_SWIPE_DISTANCE, (swipe: VideoSwipe) => (swipe === VideoSwipe.Up ? latest.current.onSmaller() : latest.current.onFullScreen())), []);
  const hasVehicle = useHasVehicle();
  return (
    <div
      {...swipes}
      data-gesture
      className="flex w-full items-center gap-3 px-3 py-2"
      style={{ background: `linear-gradient(to bottom, transparent, ${withAlpha(Color.Black, SCRIM_ALPHA)})`, touchAction: 'none' }}
    >
      <OsdCompassDial size={PORTRAIT_OSD_DIAL} />
      <div className={`flex min-w-0 flex-1 items-center overflow-hidden ${osdShadow()}`}>
        <LocalFlyOsd.Provider value>{hasVehicle ? <TelemetryRow valuesShown chooser={false} compact stacked /> : <TelemetryPlaceholder />}</LocalFlyOsd.Provider>
      </div>
      <A11yActions
        actions={[
          ['Make the video small', onSmaller],
          ['Show the video full screen', onFullScreen],
        ]}
      />
    </div>
  );
};

const TelemetryPlaceholder = () => {
  const distance = useQgcValue(settingControl('settings.unitsSettings.horizontalDistanceUnits'));
  const height = useQgcValue(settingControl('settings.unitsSettings.verticalDistanceUnits'));
  const speed = useQgcValue(settingControl('settings.unitsSettings.speedUnits'));
  const units = placeholderUnits(enumIndex(distance), enumIndex(height), enumIndex(speed));
  return (
    <div className="flex flex-col gap-0.5">
      <div className="flex gap-4">
        <PlaceholderReading label="H.S" unit={units.speed} typography="labelLarge" />
        <PlaceholderReading label="V.S" unit={units.speed} typography="labelLarge" />
      </div>
      <div className="flex gap-4">
        <PlaceholderReading label="D" unit={units.distance} typography="titleLarge" />
        <PlaceholderReading label="H" unit={units.height} typography="titleLarge" />
      </div>
    </div>
  );
};

const PlaceholderReading = ({ label, unit, typography }: { label: string; unit: string; typography: Typography }) => {
  const tint = withAlpha(aircast('outdoorForeground'), PLACEHOLDER_ALPHA);
  return (
    <div className="flex items-baseline gap-1">
      <Text typography="labelMedium" color={tint} maxLines={1}>
        {label}
      </Text>
      <Text typography={typography} color={tint} maxLines={1}>
        {`— ${unit}`}
      </Text>
    </div>
  );
};

export type PlaceholderUnits = { distance: string; height: string; speed: string };
export const PlaceholderUnits = (fields: PlaceholderUnits): PlaceholderUnits => ({ ...fields });

const SPEED_UNITS: readonly string[] = ['ft/s', 'm/s', 'mph', 'km/h', 'kn'];

const enumIndex = (value: unknown): number | null => {
  if (typeof value === 'number') return Number.isFinite(value) ? Math.trunc(value) : null;
  if (typeof value === 'string' && /^[+-]?\d+$/.test(value)) return Number(value);
  return null;
};

export const placeholderUnits = (distance: number | null, height: number | null, speed: number | null): PlaceholderUnits =>
  PlaceholderUnits({
    distance: distance === 0 ? 'ft' : 'm',
    height: height === 0 ? 'ft' : 'm',
    speed: SPEED_UNITS[speed ?? 1] ?? 'm/s',
  });

const MapButtons = ({ view, split, hasVideo, onView }: { view: FlyView; split: boolean; hasVideo: boolean; onView: (view: FlyView) => void }) => {
  const viewer3dJson = useQgcPath(VIEWER3D_VIEW);
  const enabled3d = viewer3dEnabled(viewer3dJson);
  const latestOnView = useUpdatedRef(onView);
  useEffect(() => {
    const allowed = flyViewAllowed(enabled3d, view);
    if (allowed !== view) latestOnView.current(allowed);
  }, [enabled3d, view]);
  const [layers, setLayers] = useState(false);
  const threeD = view === FlyView.ThreeD;
  return (
    <>
      <div className="flex gap-2">
        {split ? <MapButton icon="ic_fullscreen" label="Enlarge the map" onClick={() => onView(FlyView.Map)} /> : null}
        {!split && hasVideo && threeD ? <MapButton icon="ic_videocam" label="Show the camera above the map" onClick={() => onView(FlyView.Video)} /> : null}
        <MapButton icon="ic_layers" label="Map layers" onClick={() => setLayers(true)} />
        {flyViewsOffered(enabled3d, view).includes(FlyView.ThreeD) ? (
          <MapButton icon={threeD ? 'ic_map' : 'ic_explore'} label={threeD ? 'Show the map' : 'Show the 3D view'} onClick={() => onView(threeD ? FlyView.Map : FlyView.ThreeD)} />
        ) : null}
      </div>
      {layers ? <MapLayersSheet onDismiss={() => setLayers(false)} /> : null}
    </>
  );
};

const MapButton = ({ icon, label, onClick }: { icon: DrawableName; label: string; onClick: () => void }) => (
  <Surface
    onClick={onClick}
    shape="full"
    color={withAlpha(Color.Black, MAP_BUTTON_SCRIM_ALPHA)}
    contentColor={aircast('outdoorForeground')}
    label={label}
    className="grid place-items-center"
    style={{ width: MAP_BUTTON_SIZE, height: MAP_BUTTON_SIZE }}
  >
    <Drawable name={icon} size={20} />
  </Surface>
);
