import { type HTMLAttributes, type PointerEvent, useEffect, useMemo, useRef } from 'react';
import { VideoCommands } from '@/bridge/BridgeCommands';
import { Qgc } from '@/bridge/Qgc';
import { settingControl, useQgcBool, useQgcPath, useQgcValue } from '@/bridge/QgcState';
import { recordStream } from '@/core/recorder';
import { playWhep } from '@/core/whep';
import { useUpdatedRef } from '@/kotlin/Compose';
import { createDocument } from '@/kotlin/Platform';
import { same } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { ObstacleVideoOverlay } from '@/map/ObstacleOverlay';
import { Drawable } from '@/res/Icon';
import { AircastSpace, Color, aircast, withAlpha } from '@/theme/Colors';
import { CAMERA_VIEW, cameraReading } from '@/ui/CameraControl';
import { type CameraEntry, CameraStatus } from '@/ui/Cameras';
import { CameraStatusDot, showCamera, useShownSecondCamera } from '@/ui/CameraSwitch';
import { DetectionOverlay } from '@/ui/DetectionOverlay';
import { Rect, activate, useElementSize } from '@/ui/FlyShell';
import { useHasVehicle } from '@/ui/FlyState';
import { GIMBAL_INDICATOR_PATH, GimbalModeSwitch, GimbalScreenControl, onScreenGimbal } from '@/ui/GimbalIndicator';
import { LocalNoVideoSize, NoVideoPanel, NoVideoSize } from '@/ui/NoVideoPanel';
import { osdShadow } from '@/ui/Osd';
import { type PicturePress, type PictureSpot, PictureMark, usePictureHold } from '@/ui/PictureHold';
import { ProximityRadarOverlay } from '@/ui/ProximityRadar';
import { SYNTHETIC_SOURCE, SyntheticView, shownCameraStatus, useRememberSyntheticAvailable, useRememberSyntheticChosen, useSyntheticPage } from '@/ui/SyntheticView';
import { TrackingBoxOverlay } from '@/ui/TrackingBoxOverlay';
import { VideoArOverlay } from '@/ui/VideoAr';
import { usePointerGestures } from '@/ui/VideoGestures';
import { VideoStatsPill } from '@/ui/VideoStatsPill';
import { VIDEO_VIEW, type SourceSize, type VideoReading, nativeRecordingFile, videoReading, whepEndpoint, whepRestarts } from '@/ui/VideoView';

export const CAMERA_STEP_ZOOM = 'vehicle.cameraManager.currentCameraInstance.stepZoom';

export const pinchStep = (scale: number): number => (scale < 1 ? Math.round(scale * -10) : Math.round(scale));

const spread = (points: ReadonlyMap<number, { x: number; y: number }>): number => {
  const [first, second] = [...points.values()];
  return first === undefined || second === undefined ? 0 : Math.hypot(first.x - second.x, first.y - second.y);
};

const usePinchZoom = (enabled: boolean): HTMLAttributes<HTMLElement> => {
  const live = useUpdatedRef(enabled);
  return useMemo(() => {
    const state = { points: new Map<number, { x: number; y: number }>(), scale: 1, sent: 0 };
    const press = (event: PointerEvent<HTMLElement>) => {
      if (!live.current) return;
      if (state.points.size === 0) {
        state.scale = 1;
        state.sent = 0;
      }
      state.points = new Map(state.points).set(event.pointerId, { x: event.clientX, y: event.clientY });
      if (state.points.size >= 2) event.stopPropagation();
    };
    const move = (event: PointerEvent<HTMLElement>) => {
      if (!state.points.has(event.pointerId)) return;
      const before = spread(state.points);
      state.points = new Map(state.points).set(event.pointerId, { x: event.clientX, y: event.clientY });
      if (state.points.size < 2) return;
      event.stopPropagation();
      event.preventDefault();
      const after = spread(state.points);
      state.scale = before > 0 && after > 0 ? state.scale * (after / before) : state.scale;
      const step = pinchStep(state.scale);
      if (step !== state.sent) void Qgc.invoke(CAMERA_STEP_ZOOM, step);
      state.sent = step;
    };
    const release = (event: PointerEvent<HTMLElement>) => {
      if (!state.points.has(event.pointerId)) return;
      if (state.points.size >= 2) event.stopPropagation();
      state.points = new Map([...state.points].filter(([pointer]) => pointer !== event.pointerId));
    };
    return { onPointerDownCapture: press, onPointerMoveCapture: move, onPointerUpCapture: release, onPointerCancelCapture: release };
  }, []);
};

const NoVideoArea = ({ underFlyChrome, thumb, video }: { underFlyChrome: boolean; thumb: boolean; video: VideoReading | null }) =>
  underFlyChrome ? null : (
    <div className="flex size-full items-center justify-center">
      <LocalNoVideoSize.Provider value={thumb ? NoVideoSize.Thumb : NoVideoSize.Full}>
        <NoVideoPanel video={video} />
      </LocalNoVideoSize.Provider>
    </div>
  );

export const looksForAircraft = (connected: boolean, video: VideoReading | null, syntheticChosen = false): boolean => !connected && (video?.available !== true || syntheticChosen);

export const FlyNoVideoMessage = () => {
  const videoJson = useQgcPath(VIDEO_VIEW);
  const video = useMemo(() => videoReading(videoJson), [videoJson]);
  const syntheticChosen = useRememberSyntheticChosen();
  const connected = useHasVehicle();
  const synthetic = useRememberSyntheticAvailable();
  const { ref, size } = useElementSize();
  if (looksForAircraft(connected, video, syntheticChosen)) return <AwaitingAircraftLine syntheticChosen={syntheticChosen} />;
  if (synthetic) return null;
  const compact = size.height < NO_VIDEO_FULL_HEIGHT || size.width < NO_VIDEO_FULL_WIDTH;
  return (
    <div ref={ref} className="flex size-full items-center justify-center">
      <LocalNoVideoSize.Provider value={compact ? NoVideoSize.Pill : NoVideoSize.Full}>
        <NoVideoPanel video={video} />
      </LocalNoVideoSize.Provider>
    </div>
  );
};

export const awaitingAircraftText = (syntheticChosen: boolean): string =>
  syntheticChosen ? 'Synthetic view starts when your aircraft connects' : 'Video starts when your aircraft connects';

const AwaitingAircraftLine = ({ syntheticChosen }: { syntheticChosen: boolean }) => {
  const tint = withAlpha(aircast('outdoorForeground'), AWAITING_ALPHA);
  return (
    <div className="flex size-full items-center justify-center px-4">
      <div className="flex items-center gap-2">
        <Drawable name="ic_videocam" size={18} className="shrink-0" style={{ color: tint }} />
        <Text typography="bodyMedium" color={tint} textAlign="center">
          {awaitingAircraftText(syntheticChosen)}
        </Text>
      </div>
    </div>
  );
};

const AWAITING_ALPHA = 0.7;

export const centredRegion = (region: Rect, centreX: number, minWidth: number): Rect => {
  const half = Math.min(centreX - region.left, region.right - centreX);
  return half * 2 >= minWidth ? Rect(centreX - half, region.top, centreX + half, region.bottom) : region;
};

const isEmpty = (rect: Rect): boolean => rect.left >= rect.right || rect.top >= rect.bottom;

const overlaps = (rect: Rect, other: Rect): boolean => !(rect.right <= other.left || other.right <= rect.left || rect.bottom <= other.top || other.bottom <= rect.top);

const contains = (rect: Rect, other: Rect): boolean => rect.left <= other.left && rect.top <= other.top && rect.right >= other.right && rect.bottom >= other.bottom;

const sidesAround = (room: Rect, obstacle: Rect, gap: number): readonly Rect[] => [
  Rect(obstacle.right + gap, room.top, room.right, room.bottom),
  Rect(room.left, room.top, obstacle.left - gap, room.bottom),
  Rect(room.left, obstacle.bottom + gap, room.right, room.bottom),
  Rect(room.left, room.top, room.right, obstacle.top - gap),
];

const distinct = (rooms: readonly Rect[]): readonly Rect[] => rooms.filter((room, index) => rooms.findIndex((other) => same(other, room)) === index);

export const messageRegion = (free: Rect, obstacles: readonly Rect[], gap: number, minWidth: number, minHeight: number): Rect | null => {
  const fits = (room: Rect) => room.width >= minWidth && room.height >= minHeight;
  const rooms = obstacles
    .filter((it) => !isEmpty(it))
    .reduce<readonly Rect[]>((found, obstacle) => {
      const split = distinct(found.flatMap((room) => (overlaps(room, obstacle) ? sidesAround(room, obstacle, gap).filter(fits) : [room])));
      return split.filter((room) => !split.some((other) => !same(other, room) && contains(other, room)));
    }, [free].filter(fits));
  return rooms.reduce<Rect | null>((best, room) => (best === null || room.width * room.height > best.width * best.height ? room : best), null);
};

export const VideoSurface = ({
  className,
  expanded = false,
  fullScreen = false,
  onClick = () => undefined,
  onDoubleTap = () => undefined,
  onGround = null,
  groundSpot = null,
}: {
  className?: string;
  expanded?: boolean;
  fullScreen?: boolean;
  onClick?: () => void;
  onDoubleTap?: () => void;
  onGround?: ((press: PicturePress) => void) | null;
  groundSpot?: PictureSpot | null;
}) => {
  const videoJson = useQgcPath(VIDEO_VIEW);
  const video = useMemo(() => videoReading(videoJson), [videoJson]);
  const synthetic = useRememberSyntheticAvailable();
  const secondCamera = useShownSecondCamera();

  if (video?.available === false) {
    if (synthetic && !expanded) return <SyntheticThumb className={className} />;
    if (synthetic) return <SyntheticView className={`size-full ${className ?? ''}`} aimable onGround={onGround} groundSpot={groundSpot} />;
    if (!expanded) return null;
    return (
      <Surface className={`size-full ${className ?? ''}`} color={Color.Black}>
        <NoVideoArea underFlyChrome={expanded && !fullScreen} thumb={false} video={video} />
      </Surface>
    );
  }

  const second = expanded ? secondCamera : null;
  return (
    <div className={`flex ${className ?? ''}`}>
      <MainPane
        video={video}
        synthetic={synthetic}
        className="h-full min-w-0 flex-1"
        expanded={expanded}
        fullScreen={fullScreen}
        onClick={onClick}
        onDoubleTap={onDoubleTap}
        onGround={expanded ? onGround : null}
        groundSpot={groundSpot}
      />
      {second !== null ? <SecondPane camera={second} className="h-full min-w-0 flex-1" /> : null}
    </div>
  );
};

export const SYNTHETIC_THUMB_LABEL = 'Synthetic view';

const SyntheticThumb = ({ className }: { className?: string }) => (
  <Surface className={`size-full ${className ?? ''}`} color={aircast('outdoorBackground')} contentColor={aircast('outdoorForeground')}>
    <div className="flex size-full flex-col items-center justify-center gap-1">
      <Drawable name="ic_explore" size={28} />
      <Text typography="labelMedium" color={aircast('outdoorForeground')}>
        {SYNTHETIC_THUMB_LABEL}
      </Text>
    </div>
  </Surface>
);

const numberOf = (value: unknown): number | null => {
  if (typeof value === 'number') return value;
  if (value === null || value === undefined) return null;
  const text = String(value).trim();
  const parsed = Number(text);
  return text === '' || Number.isNaN(parsed) ? null : parsed;
};

const fitOf = (value: unknown): number => {
  if (typeof value === 'number') return Math.trunc(value);
  const text = value === null || value === undefined ? '' : String(value).trim();
  return /^[+-]?\d+$/.test(text) ? Number(text) : VIDEO_FIT_HEIGHT;
};

const MainPane = ({
  video,
  synthetic,
  className,
  expanded,
  fullScreen,
  onClick,
  onDoubleTap,
  onGround,
  groundSpot,
}: {
  video: VideoReading | null;
  synthetic: boolean;
  className: string;
  expanded: boolean;
  fullScreen: boolean;
  onClick: () => void;
  onDoubleTap: () => void;
  onGround: ((press: PicturePress) => void) | null;
  groundSpot: PictureSpot | null;
}) => {
  const cameraJson = useQgcPath(CAMERA_VIEW);
  const zoomable = useMemo(() => cameraReading(cameraJson)?.hasZoom === true, [cameraJson]);
  const showGrid = useQgcBool(settingControl('settings.videoSettings.gridLines'));
  const fitMode = useQgcValue(settingControl('settings.videoSettings.videoFit'));
  const aspectSetting = useQgcValue(settingControl('settings.videoSettings.aspectRatio'));
  const aspect = videoAspect(video?.sourceSize ?? null, numberOf(aspectSetting));
  const decoding = video?.decoding === true;
  const pinch = usePinchZoom(expanded && zoomable);
  const taps = usePointerGestures(expanded && decoding ? { onDoubleTap } : null);
  const gimbalJson = useQgcPath(GIMBAL_INDICATOR_PATH);
  const holdDrives = useMemo(() => {
    const control = onScreenGimbal(gimbalJson);
    return control !== null && control.enabled && control.clickAndDrag;
  }, [gimbalJson]);
  const hold = usePictureHold(decoding ? onGround : null, false, !holdDrives);
  const { ref, size } = useElementSize();
  const [contentWidth, contentHeight] = videoContentSize(size.width, size.height, aspect, fitOf(fitMode));
  const press: HTMLAttributes<HTMLElement> = expanded ? { ...pinch, ...taps } : { role: 'button', tabIndex: 0, onClick, onKeyDown: activate(onClick) };

  return (
    <div ref={ref} {...press} className={`relative flex items-center justify-center overflow-hidden ${expanded ? 'touch-none' : ''} ${className}`}>
      <div {...hold} className="relative flex-none" style={{ width: contentWidth, height: contentHeight }}>
        <VideoChannelSurface channel={MAIN_VIDEO_CHANNEL} className="absolute inset-0 size-full" />
        {decoding ? (
          <>
            {showGrid && !fullScreen ? <VideoGrid className="absolute inset-0 size-full" /> : null}
            {expanded ? <VideoArOverlay className="absolute inset-0 size-full" /> : null}
            <ProximityRadarOverlay className="absolute inset-0 size-full" />
            <ObstacleVideoOverlay className="absolute inset-0 size-full" showText={expanded} />
            {expanded ? <GimbalScreenControl className="absolute inset-0 size-full" /> : null}
            <DetectionOverlay className="absolute inset-0 size-full" />
            <TrackingBoxOverlay className="absolute inset-0 size-full" />
            {expanded ? <VideoStatsPill className="absolute inset-0 size-full" /> : null}
            {groundSpot !== null ? <PictureMark spot={groundSpot} /> : null}
          </>
        ) : null}
      </div>
      {expanded && decoding ? <GimbalModeSwitch className="absolute end-2 bottom-2" /> : null}
      {!decoding && synthetic && !expanded ? (
        <SyntheticThumb className="absolute inset-0" />
      ) : !decoding && synthetic ? (
        <SyntheticView className="absolute inset-0 size-full" aimable standIn={video?.standIn ?? null} onGround={onGround} groundSpot={groundSpot} />
      ) : !decoding ? (
        <Surface className="absolute inset-0 size-full" color={expanded ? Color.Black : aircast('outdoorBackground')}>
          <NoVideoArea underFlyChrome={expanded && !fullScreen} thumb={!expanded} video={video} />
        </Surface>
      ) : null}
    </div>
  );
};

const SecondPane = ({ camera, className }: { camera: CameraEntry; className: string }) => {
  const page = useSyntheticPage();
  const show = () => showCamera(camera.slot);
  return (
    <div
      role="button"
      tabIndex={0}
      aria-label={`Second camera, ${camera.title}`}
      aria-description={`Make ${camera.title} the main view`}
      onClick={show}
      onKeyDown={activate(show)}
      className={`relative cursor-pointer overflow-hidden ${className}`}
      style={{ marginInlineStart: PANE_DIVIDER }}
    >
      {camera.source === SYNTHETIC_SOURCE ? <SyntheticView className="absolute inset-0 size-full" aimable={false} /> : <VideoChannelSurface channel={PIP_VIDEO_CHANNEL} className="absolute inset-0 size-full" />}
      {camera.status !== CameraStatus.Live ? (
        <div className="absolute inset-0 flex items-center justify-center" style={{ background: aircast('outdoorBackground') }}>
          <Text typography="labelLarge" color={aircast('outdoorForeground')}>
            {camera.status.label}
          </Text>
        </div>
      ) : null}
      <div className={`absolute start-0 top-0 flex items-center ${osdShadow()}`} style={{ padding: AircastSpace.s2, gap: AircastSpace.s1 }}>
        <CameraStatusDot status={shownCameraStatus(camera, true, page)} />
        <Text typography="labelMedium" color={aircast('outdoorForeground')} maxLines={1}>
          {camera.short}
        </Text>
      </div>
    </div>
  );
};

const PANE_DIVIDER = 2;

export const MAIN_VIDEO_CHANNEL = 0;
export const PIP_VIDEO_CHANNEL = 1;

export class PipSurfaces {
  private attached = 0;

  constructor(private readonly report: (shown: boolean) => void) {}

  created(): void {
    this.attached = this.attached + 1;
    this.report(this.attached > 0);
  }

  destroyed(): void {
    this.attached = this.attached - 1;
    this.report(this.attached > 0);
  }
}

const pipSurfaces = new PipSurfaces((shown) => void VideoCommands.setPipShown(shown));

export const VideoChannelSurface = ({ channel, className }: { channel: number; className?: string }) => {
  const view = useQgcPath(VIDEO_VIEW);
  const endpoint = whepEndpoint(view, channel);
  const restarts = whepRestarts(view, channel);
  const recordingFile = channel === MAIN_VIDEO_CHANNEL ? nativeRecordingFile(view) : null;
  const video = useRef<HTMLVideoElement>(null);
  useEffect(() => {
    if (channel !== PIP_VIDEO_CHANNEL) return undefined;
    pipSurfaces.created();
    return () => pipSurfaces.destroyed();
  }, [channel]);
  useEffect(() => {
    const element = video.current;
    if (endpoint === null || element === null) return undefined;
    return playWhep(element, endpoint, (reading) => void VideoCommands.reportPlayback(channel, reading));
  }, [endpoint, channel, restarts]);
  useEffect(() => {
    const stream = video.current?.srcObject;
    if (recordingFile === null || !(stream instanceof MediaStream)) return undefined;
    const stop = recordStream(stream, recordingFile, (recording) => createDocument(recording.name, recording.type, recording));
    void VideoCommands.reportRecording(true);
    return () => {
      stop();
      void VideoCommands.reportRecording(false);
    };
  }, [recordingFile]);
  return <video ref={video} muted playsInline autoPlay className={`block bg-black object-fill ${className ?? ''}`} />;
};

const NO_VIDEO_FULL_HEIGHT = 230;
const NO_VIDEO_FULL_WIDTH = 260;
export const NO_VIDEO_PILL_WIDTH = 180;
export const NO_VIDEO_PILL_HEIGHT = 48;
export const VIDEO_FIT_WIDTH = 0;
export const VIDEO_FIT_HEIGHT = 1;
export const VIDEO_FILL = 2;
export const VIDEO_NO_CROP = 3;
const GRID_FRACTIONS = 3;
const GRID_COLOUR = withAlpha(Color.White, 0.5);

export const videoAspect = (source: SourceSize | null, setting: number | null): number =>
  source !== null && source.width > 0 && source.height > 0 ? source.width / source.height : setting !== null && setting > 0 ? setting : 0.0;

export const videoContentSize = (width: number, height: number, aspect: number, fit: number): readonly [number, number] => {
  if (aspect === 0.0 || width <= 0 || height <= 0) return [width, height];
  const box = width / height;
  const shownWidth = fit === VIDEO_FIT_HEIGHT || (fit === VIDEO_FILL && box < aspect) || (fit === VIDEO_NO_CROP && box > aspect) ? height * aspect : width;
  const shownHeight = fit === VIDEO_FIT_WIDTH || (fit === VIDEO_FILL && box > aspect) || (fit === VIDEO_NO_CROP && box < aspect) ? width / aspect : height;
  return [shownWidth, shownHeight];
};

const VideoGrid = ({ className }: { className?: string }) => (
  <svg className={`pointer-events-none ${className ?? ''}`} aria-hidden>
    {Array.from({ length: GRID_FRACTIONS - 1 }, (_, index) => {
      const at = `${(index + 1) * 33}%`;
      return (
        <g key={index} stroke={GRID_COLOUR} strokeWidth={1}>
          <line x1={at} y1="0" x2={at} y2="100%" />
          <line x1="0" y1={at} x2="100%" y2={at} />
        </g>
      );
    })}
  </svg>
);
