import { type CSSProperties, type ReactNode, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { type Json, isNull } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { VideoCommands } from '@/bridge/BridgeCommands';
import { settingControl, useQgcBool, useQgcPath } from '@/bridge/QgcState';
import { useKeyedState, useLaunchedEffect, useProduceState, useUpdatedRef } from '@/kotlin/Compose';
import { coerceIn, delay, format } from '@/kotlin/Std';
import { AlertDialog, FilterChip, RadioButton, Slider, Surface, Text, TextButton } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import { Color, aircast, argb, md, withAlpha } from '@/theme/Colors';
import {
  CAMERA_CURRENT_STREAM,
  CAMERA_PHOTO_LAPSE,
  CAMERA_PHOTO_MODE,
  CAMERA_SET_MODE,
  CAMERA_SHEET_VIDEO_SETTINGS,
  CAMERA_START_PURSUIT,
  CAMERA_STOP_PURSUIT,
  CAMERA_STOP_TRACKING,
  CAMERA_THERMAL_MODE,
  CAMERA_THERMAL_OPACITY,
  CAMERA_TRACKING_ARMED,
  CAMERA_VIEW,
  CAMERA_ZOOM,
  type CameraPanel,
  type CameraReading,
  type CameraShutter,
  type DestructiveAction,
  PHOTO_LAPSE_MAX_S,
  PHOTO_LAPSE_MIN_S,
  type PursuitReading,
  THERMAL_MODES,
  type ThermalReading,
  type TrackingReading,
  ZOOM_HIGHEST,
  ZOOM_LOWEST,
  cameraDetails,
  cameraReading,
  destructiveActions,
  destructiveInvokePath,
  destructiveReasonFor,
  lapsePlan,
  modeTapSwitches,
  pursuitReading,
  shutterCaption,
  shutterReadout,
  thermalModeLabel,
  thermalOpacityIsOffered,
  thermalReading,
  trackingReading,
  trackingToggleLabel,
  zoomText,
} from '@/ui/CameraControl';
import { CameraDefinitionSettings } from '@/ui/CameraSettingsSection';
import { OpenOnRequest } from '@/ui/FlyScreenState';
import { useHasVehicle } from '@/ui/FlyState';
import { LensSection } from '@/ui/LensSection';
import { osdShadow, useOsdBackdrop } from '@/ui/Osd';
import { factFromControl } from '@/ui/ParameterForm';
import { RailSlider, RcCameraControls } from '@/ui/RcCameraControls';
import { FactRow } from '@/ui/SettingsScreen';
import { SectionHeader } from '@/ui/SetupRow';
import { VIDEO_VIEW, videoReading } from '@/ui/VideoView';

const REFUSAL_MS = 4000;
const ZOOM_CHIP_BORDER_ALPHA = 0.4;

const MANAGER = 'vehicle.cameraManager';
export const CAMERA_SHEET = 'camera';

export const SHOW_PHOTO_VIDEO_CONTROL = 'settings.flyViewSettings.showPhotoVideoControl';
const CAMERA_SCRIM_ALPHA = 0.55;
const SHUTTER_SIZE = 56;
const SHUTTER_STOP_SIZE = 20;
const SHUTTER_DOT_SIZE = 26;
const SHUTTER_RING = 4;
const SHUTTER_STOP_CORNER = 4;
const DISABLED_SHUTTER_ALPHA = 0.38;
const CAMERA_CLUSTER_GAP = 8;
const ZOOM_IDLE_MS = 3000;
const MODE_IDLE_ALPHA = 0.6;
const MODE_ROW_HEIGHT = 28;
const CAMERA_RAIL_MAX_WIDTH = 120;
const REC_DOT_SIZE = 8;
const REC_BLINK_MS = 600;
export const RECORD_RED = argb(0xffe53935);

const useTimedRefusal = (): readonly [string | null, (refusal: string | null) => void] => {
  const [refused, setRefused] = useState<string | null>(null);
  useLaunchedEffect([refused], async (scope) => {
    if (refused !== null) {
      await delay(REFUSAL_MS);
      if (scope.isActive()) setRefused(null);
    }
  });
  return [refused, setRefused];
};

const useConflatedWrites = (path: string): ((value: number) => void) => {
  const queue = useRef<{ writing: boolean; next: number | null }>({ writing: false, next: null });
  return useCallback(
    (value: number) => {
      const drain = async (): Promise<void> => {
        const next = queue.current.next;
        queue.current = { writing: next !== null, next: null };
        if (next === null) return;
        await Qgc.set(path, next);
        return drain();
      };
      const writing = queue.current.writing;
      queue.current = { writing: true, next: value };
      if (!writing) void drain();
    },
    [path],
  );
};

const pressShutter = (shutter: CameraShutter, onRefused: (refusal: string | null) => void) =>
  void (async () => onRefused(shutter.action === null ? null : await Qgc.refusalOf(shutter.action)))();

export const CameraControlLayer = ({ className, shutters = true }: { className?: string; shutters?: boolean }) => {
  const hasVehicle = useHasVehicle();
  const shown = useQgcBool(settingControl(SHOW_PHOTO_VIDEO_CONTROL));
  const cameraJson = useQgcPath(CAMERA_VIEW);
  const camera = useMemo(() => cameraReading(cameraJson), [cameraJson]);
  const [refused, setRefused] = useTimedRefusal();
  const [details, setDetails] = useState(false);
  const thermal = useMemo(() => thermalReading(cameraJson), [cameraJson]);
  const tracking = useMemo(() => trackingReading(cameraJson), [cameraJson]);
  const pursuit = useMemo(() => pursuitReading(cameraJson), [cameraJson]);
  const destructive = useMemo(() => destructiveActions(cameraJson), [cameraJson]);

  if (!shown) return null;

  const panel = camera !== null && hasVehicle && camera.panel?.visible === true ? camera.panel : null;
  if (camera === null || panel === null) {
    return (
      <div className={`flex items-center ${className ?? ''}`}>
        <RcCameraControls />
        {shutters ? <StreamShutter /> : null}
      </div>
    );
  }
  const zoomLabel = zoomText(camera);
  const lapse = lapsePlan(camera);

  return (
    <>
      <OpenOnRequest name={CAMERA_SHEET} open={() => setDetails(true)} />
      <div className={className} style={{ color: aircast('outdoorForeground') }}>
        {shutters ? (
          <div className="flex items-center">
            <RcCameraControls />
            <div className="px-1.5 py-2">
              <ShutterCentredRail
                centre={panel.shutters.length > 0 ? SHUTTER_SIZE : 0}
                style={{ maxWidth: CAMERA_RAIL_MAX_WIDTH }}
                above={zoomLabel !== null ? <ZoomChip camera={camera} label={zoomLabel} /> : null}
                shutter={panel.shutters.map((shutter) => (
                  <ShutterButton key={shutter.video ? 'video' : 'photo'} caption={shutterCaption(panel, shutter)} shutter={shutter} onPress={() => pressShutter(shutter, setRefused)} />
                ))}
                below={
                  <>
                    <CameraModeSwitch camera={camera} panel={panel} onRefused={setRefused} />
                    {lapse !== null ? <Text typography="labelSmall">{lapse}</Text> : null}
                    {refused !== null ? (
                      <Text typography="bodySmall" color={md('error')} className="px-1">
                        {refused}
                      </Text>
                    ) : null}
                  </>
                }
              />
            </div>
          </div>
        ) : (
          <div className="flex items-center" style={{ gap: CAMERA_CLUSTER_GAP }}>
            {zoomLabel !== null ? <ZoomChip camera={camera} label={zoomLabel} /> : null}
            <RcCameraControls tiltSlider={false} />
          </div>
        )}
      </div>
      {details ? (
        <CameraDetailsSheet
          camera={camera}
          thermal={thermal}
          tracking={tracking}
          pursuit={pursuit}
          destructive={destructive}
          storage={[panel.freeText, panel.batteryText].filter((it): it is string => it !== null)}
          current={camera.selected ?? 0}
          onSelect={(index) => {
            void Qgc.set(`${MANAGER}.currentCamera`, index);
            setDetails(false);
          }}
          onDismiss={() => setDetails(false)}
        />
      ) : null}
    </>
  );
};

const ZoomChip = ({ camera, label }: { camera: CameraReading; label: string }) => {
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState<HTMLDivElement | null>(null);
  const foreground = aircast('outdoorForeground');
  const backdrop = useOsdBackdrop(withAlpha(Color.Black, CAMERA_SCRIM_ALPHA));
  return (
    <div ref={setAnchor} className="relative">
      <Surface onClick={() => setOpen(!open)} shape="full" color={backdrop} contentColor={foreground} border={`1px solid ${withAlpha(foreground, ZOOM_CHIP_BORDER_ALPHA)}`} label={`Zoom ${label}`}>
        <span className="flex items-center gap-1 px-2.5 py-1.5">
          <Drawable name="ic_search" size={16} />
          <Text typography="labelLarge">{label}</Text>
        </span>
      </Surface>
      {open && anchor !== null ? (
        <BesidePopup anchor={anchor} onDismissRequest={() => setOpen(false)}>
          <ZoomSlider camera={camera} onIdle={() => setOpen(false)} />
        </BesidePopup>
      ) : null}
    </div>
  );
};

type AnchorBounds = { left: number; top: number; right: number; bottom: number };
type PopupSize = { width: number; height: number };
type PopupOffset = { x: number; y: number };

export const BesideAnchor = {
  calculatePosition: (anchorBounds: AnchorBounds, windowSize: PopupSize, layoutDirection: 'Ltr' | 'Rtl', popupContentSize: PopupSize): PopupOffset => ({
    x: coerceIn(layoutDirection === 'Ltr' ? anchorBounds.left - popupContentSize.width : anchorBounds.right, 0, Math.max(windowSize.width - popupContentSize.width, 0)),
    y: coerceIn(
      anchorBounds.top + Math.trunc((anchorBounds.bottom - anchorBounds.top) / 2) - Math.trunc(popupContentSize.height / 2),
      0,
      Math.max(windowSize.height - popupContentSize.height, 0),
    ),
  }),
};

export const BesidePopup = ({ anchor, onDismissRequest, children }: { anchor: HTMLElement; onDismissRequest: () => void; children: ReactNode }) => {
  const [popup, setPopup] = useState<HTMLDivElement | null>(null);
  const [offset, setOffset] = useState<PopupOffset | null>(null);
  const dismiss = useUpdatedRef(onDismissRequest);
  useLayoutEffect(() => {
    if (popup === null) return undefined;
    const place = () => {
      const bounds = anchor.getBoundingClientRect();
      const size = popup.getBoundingClientRect();
      setOffset(
        BesideAnchor.calculatePosition(
          { left: bounds.left, top: bounds.top, right: bounds.right, bottom: bounds.bottom },
          { width: window.innerWidth, height: window.innerHeight },
          getComputedStyle(anchor).direction === 'rtl' ? 'Rtl' : 'Ltr',
          { width: size.width, height: size.height },
        ),
      );
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(popup);
    observer.observe(anchor);
    window.addEventListener('resize', place);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', place);
    };
  }, [anchor, popup]);
  useEffect(() => {
    const outside = (event: PointerEvent) => {
      const target = event.target;
      if (target instanceof Node && !anchor.contains(target) && popup?.contains(target) !== true) dismiss.current();
    };
    document.addEventListener('pointerdown', outside);
    return () => document.removeEventListener('pointerdown', outside);
  }, [anchor, popup]);
  return createPortal(
    <div ref={setPopup} className="fixed z-50" style={{ left: offset?.x ?? 0, top: offset?.y ?? 0, visibility: offset === null ? 'hidden' : undefined }}>
      {children}
    </div>,
    document.body,
  );
};

const ZoomSlider = ({ camera, onIdle }: { camera: CameraReading; onIdle: () => void }) => {
  const [dragging, setDragging] = useState<number | null>(null);
  const [held, setHeld] = useState(false);
  const write = useConflatedWrites(CAMERA_ZOOM);
  useLaunchedEffect([camera.zoomLevel], () => {
    if (!held) setDragging(null);
  });
  useLaunchedEffect([held, dragging], async (scope) => {
    if (!held) {
      await delay(ZOOM_IDLE_MS);
      if (scope.isActive()) onIdle();
    }
  });
  const level = dragging ?? camera.zoomLevel;
  return (
    <div style={{ paddingInline: CAMERA_CLUSTER_GAP }}>
      <Surface shape="medium" color={md('surfaceContainerHigh')} contentColor={md('onSurface')} shadow>
        <div className="flex flex-col items-center py-2">
          <Text typography="labelSmall">Zoom</Text>
          <Text typography="labelLarge">{`${Math.trunc(level)}%`}</Text>
          <div style={{ height: 140 }}>
            <RailSlider
              value={level}
              onValueChange={(next) => {
                setHeld(true);
                setDragging(next);
                write(next);
              }}
              onValueChangeFinished={() => setHeld(false)}
              valueRange={[ZOOM_LOWEST, ZOOM_HIGHEST]}
              label="Zoom"
            />
          </div>
        </div>
      </Surface>
    </div>
  );
};

const CameraModeSwitch = ({ camera, panel, onRefused }: { camera: CameraReading; panel: CameraPanel; onRefused: (refusal: string | null) => void }) => {
  const white = aircast('outdoorForeground');
  return (
    <div className="flex flex-col items-center">
      {[false, true]
        .filter((video) => camera.hasModes || video !== panel.inPhotoMode)
        .map((video) => {
          const current = video !== panel.inPhotoMode;
          const name = video ? 'VIDEO' : 'PHOTO';
          const selectable = !current && camera.hasModes && (video ? panel.selectVideoEnabled : panel.selectPhotoEnabled);
          return (
            <button
              key={name}
              type="button"
              disabled={!selectable}
              title={selectable ? `Switch to ${name.toLowerCase()}` : undefined}
              onClick={() => {
                if (modeTapSwitches(camera, video)) void (async () => onRefused(await Qgc.refusalOf(CAMERA_SET_MODE, video ? 'video' : 'photo')))();
              }}
              className={`px-2 py-1.5 text-label-small ${osdShadow()}`}
              style={{ minHeight: MODE_ROW_HEIGHT, fontWeight: current ? 700 : 400, color: current ? white : withAlpha(white, MODE_IDLE_ALPHA) }}
            >
              {name}
            </button>
          );
        })}
    </div>
  );
};

export const CameraShutters = () => {
  const hasVehicle = useHasVehicle();
  const shown = useQgcBool(settingControl(SHOW_PHOTO_VIDEO_CONTROL));
  const cameraJson = useQgcPath(CAMERA_VIEW);
  const camera = useMemo(() => cameraReading(cameraJson), [cameraJson]);
  const panel = camera !== null && hasVehicle && camera.panel?.visible === true ? camera.panel : null;
  const [refused, setRefused] = useTimedRefusal();

  if (!shown) return null;
  if (camera === null || panel === null) return <StreamShutter />;

  return (
    <div className="flex items-center gap-1">
      {camera.hasModes ? <CameraModeSwitch camera={camera} panel={panel} onRefused={setRefused} /> : null}
      <div className="flex flex-col items-center gap-1">
        {panel.shutters.map((shutter) => (
          <ShutterButton key={shutter.video ? 'video' : 'photo'} caption={shutterCaption(panel, shutter)} shutter={shutter} onPress={() => pressShutter(shutter, setRefused)} />
        ))}
        {refused !== null ? (
          <Text typography="bodySmall" color={md('error')}>
            {refused}
          </Text>
        ) : null}
      </div>
    </div>
  );
};

const RAIL_GAP = 8;

const ShutterCentredRail = ({ centre, style, above, shutter, below }: { centre: number; style?: CSSProperties; above: ReactNode; shutter: ReactNode; below: ReactNode }) => (
  <div className="grid justify-items-center" style={{ gridTemplateRows: `1fr ${centre}px 1fr`, ...style }}>
    <div className="row-start-1 flex flex-col items-center self-end empty:hidden" style={{ gap: RAIL_GAP, marginBottom: RAIL_GAP }}>
      {above}
    </div>
    <div className="row-span-2 row-start-2 flex flex-col items-center self-start" style={{ gap: RAIL_GAP }}>
      <div className="flex flex-col items-center" style={{ gap: RAIL_GAP }}>
        {shutter}
      </div>
      <div className="flex flex-col items-center empty:hidden" style={{ gap: RAIL_GAP }}>
        {below}
      </div>
    </div>
  </div>
);

export const VIDEO_RECORDING_STATE = 'video.recording';

export const STREAM_NOT_RECORDABLE = "Can't record this camera";
const RECORD_CLOCK_TICK_MS = 1000;

export const recordClock = (seconds: number): string => format('%02d:%02d:%02d', Math.trunc(seconds / 3600), Math.trunc(seconds / 60) % 60, seconds % 60);

export const streamShutter = (recording: boolean, recordable: boolean, elapsedSeconds: number | null): CameraShutter => ({
  label: recording ? 'Stop recording' : 'Start recording',
  recording,
  enabled: recordable || recording,
  action: null,
  video: true,
  readout: elapsedSeconds === null ? '' : recordClock(elapsedSeconds),
  readoutActive: recording && elapsedSeconds !== null,
});

const StreamShutter = () => {
  const videoJson = useQgcPath(VIDEO_VIEW);
  const recording = useQgcBool(VIDEO_RECORDING_STATE);
  if (videoReading(videoJson)?.decoding !== true) return null;
  return <StreamShutterShown videoJson={videoJson} recording={recording} />;
};

const StreamShutterShown = ({ videoJson, recording }: { videoJson: Json | null; recording: boolean }) => {
  const recordable = videoJson === null || isNull(videoJson, 'deviceCamera');
  const elapsed = useProduceState<number | null>(null, [recording], async (set, scope) => {
    const started = performance.now();
    set(recording ? 0 : null);
    const tick = async (): Promise<void> => {
      if (!recording || !scope.isActive()) return;
      await delay(RECORD_CLOCK_TICK_MS);
      set(Math.trunc((performance.now() - started) / RECORD_CLOCK_TICK_MS));
      return tick();
    };
    await tick();
  });
  return (
    <div className="flex flex-col items-center">
      <ShutterButton caption={null} shutter={streamShutter(recording, recordable, elapsed)} onPress={() => void VideoCommands.setRecording(!recording)} />
      {!recordable ? (
        <Text typography="labelSmall" className={osdShadow()}>
          {STREAM_NOT_RECORDABLE}
        </Text>
      ) : null}
    </div>
  );
};

const RecDot = () => {
  const dot = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    const element = dot.current;
    if (element === null || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return undefined;
    const blink = element.animate([{ opacity: 1 }, { opacity: 0 }], { duration: REC_BLINK_MS, iterations: Infinity, direction: 'alternate' });
    return () => blink.cancel();
  }, []);
  return <span ref={dot} className="block rounded-full" style={{ width: REC_DOT_SIZE, height: REC_DOT_SIZE, background: RECORD_RED }} />;
};

const ShutterButton = ({ caption, shutter, onPress }: { caption: string | null; shutter: CameraShutter; onPress: () => void }) => {
  const white = aircast('outdoorForeground');
  const readout = shutterReadout(shutter);
  const mark = shutter.recording ? SHUTTER_STOP_SIZE : SHUTTER_DOT_SIZE;
  return (
    <div className="flex flex-col items-center gap-0.5">
      <Surface
        onClick={onPress}
        enabled={shutter.enabled}
        label={shutter.label}
        shape="full"
        color={shutter.video ? Color.Transparent : white}
        border={shutter.video ? `${SHUTTER_RING}px solid ${white}` : undefined}
        className="grid place-items-center"
        style={{ width: SHUTTER_SIZE, height: SHUTTER_SIZE, opacity: shutter.enabled ? 1 : DISABLED_SHUTTER_ALPHA }}
      >
        {shutter.video ? <span className="block" style={{ width: mark, height: mark, background: RECORD_RED, borderRadius: shutter.recording ? SHUTTER_STOP_CORNER : '50%' }} /> : null}
      </Surface>
      {caption !== null ? <Text typography="labelSmall">{caption}</Text> : null}
      {readout !== null ? (
        <div className="flex items-center gap-1">
          {shutter.video ? <RecDot /> : null}
          <Text typography="labelMedium" style={{ fontVariantNumeric: 'tabular-nums' }}>
            {readout}
          </Text>
        </div>
      ) : null}
    </div>
  );
};

const TrackingToggle = ({ reading }: { reading: TrackingReading }) => (
  <TextButton
    contentColor={aircast('outdoorForeground')}
    onClick={() =>
      void (async () => {
        if (reading.requested) {
          await Qgc.set(CAMERA_TRACKING_ARMED, false);
          await Qgc.invoke(CAMERA_STOP_TRACKING);
        } else {
          await Qgc.set(CAMERA_TRACKING_ARMED, true);
        }
      })()
    }
  >
    <Text typography="labelLarge" maxLines={1}>
      {trackingToggleLabel(reading)}
    </Text>
  </TextButton>
);

const ChaseControl = ({ reading }: { reading: PursuitReading }) => {
  const [confirming, setConfirming] = useState(false);
  const [refused, setRefused] = useTimedRefusal();
  if (reading.engaged)
    return (
      <div className="px-3">
        <Surface
          onClick={() => void Qgc.invoke(CAMERA_STOP_PURSUIT)}
          shape="full"
          color={md('error')}
          contentColor={md('onError')}
          className="flex w-full items-center justify-center py-3"
          label="Stop chasing the target"
        >
          <Text typography="titleMedium" maxLines={1}>
            Chasing — Stop
          </Text>
        </Surface>
      </div>
    );
  return (
    <>
      <div className="flex items-center px-3">
        <TextButton contentColor={aircast('outdoorForeground')} enabled={reading.ready} onClick={() => setConfirming(true)}>
          <Text typography="labelLarge" maxLines={1}>
            Chase
          </Text>
        </TextButton>
      </div>
      {!reading.ready && reading.reason.trim() !== '' ? (
        <Text typography="labelSmall" color={md('onSurfaceVariant')} className="px-sheet">
          {reading.reason}
        </Text>
      ) : null}
      {refused !== null ? (
        <Text typography="bodySmall" color={md('error')} className="px-sheet">
          {refused}
        </Text>
      ) : null}
      {confirming ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title="Chase the target?"
          text="The aircraft will fly itself toward the locked target. Keep the area clear and be ready to take manual control."
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(false);
                void (async () => setRefused(await Qgc.refusalOf(CAMERA_START_PURSUIT)))();
              }}
            >
              <span style={{ color: md('error') }}>Chase</span>
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};

const SheetRadioRow = ({ label, selected, onClick }: { label: string; selected: boolean; onClick: () => void }) => (
  <div onClick={onClick} className="flex min-h-14 w-full cursor-pointer items-center gap-4 px-4 hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]">
    <RadioButton selected={selected} onClick={null} label={label} />
    <Text typography="bodyLarge">{label}</Text>
  </div>
);

const PHOTO_MODES: readonly (readonly [string, boolean])[] = [
  ['Single', false],
  ['Time lapse', true],
];

const CameraDetailsSheet = ({
  camera,
  thermal,
  tracking,
  pursuit,
  destructive,
  storage,
  current,
  onSelect,
  onDismiss,
}: {
  camera: CameraReading;
  thermal: ThermalReading | null;
  tracking: TrackingReading | null;
  pursuit: PursuitReading | null;
  destructive: readonly DestructiveAction[];
  storage: readonly string[];
  current: number;
  onSelect: (index: number) => void;
  onDismiss: () => void;
}) => (
  <AircastSheet onDismissRequest={onDismiss}>
    <Text typography="titleLarge" className="px-4">
      {camera.title.trim() === '' ? 'Camera' : camera.title}
    </Text>
    {cameraDetails(camera).map(([label, reading]) => (
      <div key={label} className="flex w-full justify-between px-4 py-1">
        <Text typography="bodyMedium">{label}</Text>
        <Text typography="bodyMedium">{reading}</Text>
      </div>
    ))}
    {storage.map((line, at) => (
      <Text key={at} typography="bodyMedium" className="px-4 py-1">
        {line}
      </Text>
    ))}
    {camera.labels.length > 1 ? (
      <>
        <SectionHeader text="Cameras" />
        {camera.labels.map((label, index) => (
          <SheetRadioRow key={index} label={label} selected={index === current} onClick={() => onSelect(index)} />
        ))}
      </>
    ) : null}
    {camera.streamLabels.length > 1 ? (
      <>
        <SectionHeader text="Video stream" />
        {camera.streamLabels.map((label, index) => (
          <SheetRadioRow key={index} label={label} selected={index === camera.currentStream} onClick={() => void Qgc.set(CAMERA_CURRENT_STREAM, index)} />
        ))}
      </>
    ) : null}
    {camera.hasZoom ? (
      <>
        <SectionHeader text="Zoom" />
        <DetailsZoom zoomLevel={camera.zoomLevel} />
      </>
    ) : null}
    {camera.capturesPhotos ? (
      <>
        <SectionHeader text="Photo mode" />
        <div className="flex gap-2 px-4">
          {PHOTO_MODES.map(([label, lapse], index) => (
            <FilterChip key={label} selected={camera.timelapse === lapse} onClick={() => void Qgc.set(CAMERA_PHOTO_MODE, index)} label={label} />
          ))}
        </div>
        {camera.timelapse ? <LapseInterval lapseSeconds={camera.lapseSeconds} /> : null}
      </>
    ) : null}
    {tracking !== null ? (
      <div className="px-3">
        <TrackingToggle reading={tracking} />
      </div>
    ) : null}
    {pursuit !== null ? <ChaseControl reading={pursuit} /> : null}
    <LensSection />
    {thermal !== null ? (
      <>
        <SectionHeader text="Thermal view mode" />
        {THERMAL_MODES.map((token, index) => (
          <SheetRadioRow key={token} label={thermalModeLabel(token)} selected={token === thermal.mode} onClick={() => void Qgc.set(CAMERA_THERMAL_MODE, index)} />
        ))}
        {thermalOpacityIsOffered(thermal) ? (
          <>
            <SectionHeader text="Blend opacity" />
            <ThermalOpacity opacity={thermal.opacity} />
          </>
        ) : null}
      </>
    ) : null}
    {camera.hasVideoStream ? <VideoSettingFacts /> : null}
    <CameraDefinitionSettings />
    {destructive.map((action) => (
      <DestructiveRow key={action.id} action={action} />
    ))}
    <div className="pb-6" />
  </AircastSheet>
);

const DetailsZoom = ({ zoomLevel }: { zoomLevel: number }) => {
  const [dragging, setDragging] = useState<number | null>(null);
  const [dragged, setDragged] = useState(false);
  const write = useConflatedWrites(CAMERA_ZOOM);
  useLaunchedEffect([zoomLevel], () => {
    if (!dragged) setDragging(null);
  });
  return (
    <div className="px-sheet">
      <Slider
        value={dragging ?? zoomLevel}
        onValueChange={(level) => {
          setDragged(true);
          setDragging(level);
          write(level);
        }}
        onValueChangeFinished={() => setDragged(false)}
        valueRange={[ZOOM_LOWEST, ZOOM_HIGHEST]}
        label="Zoom"
      />
    </div>
  );
};

const LapseInterval = ({ lapseSeconds }: { lapseSeconds: number | null }) => {
  const [interval, setTyped] = useKeyedState([lapseSeconds], () => coerceIn(lapseSeconds ?? 1.0, PHOTO_LAPSE_MIN_S, PHOTO_LAPSE_MAX_S));
  const latest = useRef(interval);
  latest.current = interval;
  return (
    <>
      <Text typography="titleSmall" className="whitespace-pre px-sheet py-2">
        {`Photo Interval (seconds)  ${Math.trunc(interval)}`}
      </Text>
      <div className="px-sheet">
        <Slider
          value={interval}
          onValueChange={(next) => {
            latest.current = next;
            setTyped(next);
          }}
          onValueChangeFinished={() => void Qgc.set(CAMERA_PHOTO_LAPSE, Math.trunc(latest.current))}
          valueRange={[PHOTO_LAPSE_MIN_S, PHOTO_LAPSE_MAX_S]}
          steps={PHOTO_LAPSE_MAX_S - PHOTO_LAPSE_MIN_S - 1}
          label="Photo Interval (seconds)"
        />
      </div>
    </>
  );
};

const ThermalOpacity = ({ opacity }: { opacity: number | null }) => {
  const [typed, setTyped] = useKeyedState([opacity], () => opacity ?? 0.0);
  const latest = useRef(typed);
  latest.current = typed;
  return (
    <div className="px-sheet">
      <Slider
        value={typed}
        onValueChange={(next) => {
          latest.current = next;
          setTyped(next);
        }}
        onValueChangeFinished={() => void Qgc.set(CAMERA_THERMAL_OPACITY, latest.current)}
        valueRange={[0, 100]}
        label="Blend opacity"
      />
    </div>
  );
};

const VideoSettingFacts = () => {
  const facts = useProduceState<readonly (readonly [Fact, string])[]>([], [], async (set) => {
    const read = await Promise.all(
      [...CAMERA_SHEET_VIDEO_SETTINGS].map(async ([path, title]) => {
        try {
          const fact = factFromControl(await Qgc.get(settingControl(path)));
          return fact === null ? null : ([fact, title] as const);
        } catch {
          return null;
        }
      }),
    );
    set(read.filter((it): it is readonly [Fact, string] => it !== null));
  });
  return (
    <>
      {facts.map(([fact, title]) => (
        <FactRow key={fact.path} fact={fact} title={sentenceCase(title)} />
      ))}
    </>
  );
};

const DestructiveRow = ({ action }: { action: DestructiveAction }) => {
  const [confirming, setConfirming] = useState(false);
  const reason = destructiveReasonFor(action);
  return (
    <>
      <div className="flex w-full items-center justify-between ps-4 pe-3">
        <Text typography="bodyLarge">{sentenceCase(action.label)}</Text>
        <TextButton onClick={() => setConfirming(true)} enabled={action.ready}>
          <span style={{ color: md('error') }}>{action.button}</span>
        </TextButton>
      </div>
      {reason !== null ? (
        <Text typography="labelSmall" color={md('onSurfaceVariant')} className="px-sheet">
          {reason}
        </Text>
      ) : null}
      {confirming ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title={action.title}
          text={action.prompt}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(false);
                const path = destructiveInvokePath(action.id);
                if (path !== null) void (action.id === 'formatStorage' ? Qgc.invoke(path, 1) : Qgc.invoke(path));
              }}
            >
              <span style={{ color: md('error') }}>Yes</span>
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>No</TextButton>}
        />
      ) : null}
    </>
  );
};
