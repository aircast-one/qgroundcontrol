import { type SyntheticEvent, useEffect, useMemo, useRef, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import { type Json, isNull, jsonObjects, optBoolean, optDouble, optJSONArray } from '@/bridge/Json';
import { useLaunchedEffect, useProduceState, useUpdatedRef } from '@/kotlin/Compose';
import { coerceAtLeast, coerceIn, delay } from '@/kotlin/Std';
import { mutableStateMapOf, useValue } from '@/kotlin/State';
import { CircularProgressIndicator, Text } from '@/m3';
import { Color, aircast, withAlpha } from '@/theme/Colors';
import { CAMERAS_VIEW, type CameraEntry, CameraStatus, type CamerasReading, camerasReading } from '@/ui/Cameras';
import { gimbalRefusal } from '@/ui/GimbalIndicator';
import { osdShadow } from '@/ui/Osd';
import { type PicturePress, type PictureSpot, PictureMark, usePictureHold } from '@/ui/PictureHold';
import { SyntheticSurface, nativeChosen, nativeDraw } from '@/ui/SyntheticSurface';
import { type PointerGestureHandlers, usePointerGestures } from '@/ui/VideoGestures';

export const SYNTHETIC_VIEW = 'view.syntheticView';
export const SYNTHETIC_OVERLAYS = 'view.syntheticOverlays';
export const SYNTHETIC_LABEL = 'Synthetic view';
export const SYNTHETIC_SOURCE = 'Synthetic View';
export const SYNTHETIC_AIM = 'syntheticView.aim';
const TILT_SPAN_DEG = 90.0;
export const DEFAULT_TILT_DEG = -15.0;
const CAMERA_FOV_DEG = 70.0;
const SYNTHETIC_PAGE = `${import.meta.env.BASE_URL}assets/synthetic/index.html`;
const SYNTHETIC_TILES = '/assets/synthetic/tiles';
const SYNTHETIC_HOST = 'aircastHost';
const NOTE_SPINNER = 16;
const NOTE_SPINNER_STROKE = 2;
const STAND_IN_ANNOUNCED_MS = 4000;
const EDGE_NOTE_SCRIM_ALPHA = 0.35;
const EDGE_NOTE_TEXT_ALPHA = 0.75;

export type SyntheticDraw = { readonly name: 'Loading' | 'Drawn' | 'Failed'; readonly ordinal: number; readonly note: string | null };

export const SyntheticDraw = {
  Loading: { name: 'Loading', ordinal: 0, note: 'Loading 3D view…' },
  Drawn: { name: 'Drawn', ordinal: 1, note: null },
  Failed: { name: 'Failed', ordinal: 2, note: '3D view failed to start' },
} as const satisfies Record<string, SyntheticDraw>;

export const SyntheticDrawEntries: readonly SyntheticDraw[] = [SyntheticDraw.Loading, SyntheticDraw.Drawn, SyntheticDraw.Failed];

const pages = mutableStateMapOf<object, SyntheticDraw>();

export const pageOf = (draws: Iterable<SyntheticDraw>): SyntheticDraw => {
  const all = [...draws];
  return all.includes(SyntheticDraw.Drawn) ? SyntheticDraw.Drawn : all.includes(SyntheticDraw.Failed) ? SyntheticDraw.Failed : SyntheticDraw.Loading;
};

export const syntheticPage = (): SyntheticDraw => pageOf(pages.value.values());

export const useSyntheticPage = (): SyntheticDraw => pageOf(useValue(pages).values());

export const shownCameraStatus = (camera: CameraEntry, showing: boolean, page: SyntheticDraw): CameraStatus => {
  if (!showing || camera.source !== SYNTHETIC_SOURCE || camera.status !== CameraStatus.Live) return camera.status;
  if (page === SyntheticDraw.Drawn) return CameraStatus.Live;
  if (page === SyntheticDraw.Failed) return CameraStatus.NoSignal;
  return CameraStatus.Connecting;
};

export const syntheticTilePath = (path: string | null): string | null => (path !== null && path.startsWith(`${SYNTHETIC_TILES}/`) ? path.slice(SYNTHETIC_TILES.length) : null);

export const syntheticAvailable = (view: Json | null): boolean => optBoolean(view, 'available');

export const syntheticPoseScript = (view: Json): string => `window.aircast && window.aircast.pose(${JSON.stringify(view)})`;

export const syntheticOverlaysScript = (view: Json): string => `window.aircast && window.aircast.overlays(${JSON.stringify(view)})`;

export const verticalFov = (hFov: number, widthPx: number, heightPx: number): number =>
  (2.0 * Math.atan(Math.tan((hFov * Math.PI) / 360.0) * (coerceAtLeast(heightPx, 1) / coerceAtLeast(widthPx, 1))) * 180.0) / Math.PI;

export const syntheticTilt = (from: number, draggedPx: number, heightPx: number, vFov: number): number =>
  coerceIn(from + (draggedPx / coerceAtLeast(heightPx, 1)) * vFov, -TILT_SPAN_DEG, 0.0);

const mod = (value: number, divisor: number): number => ((value % divisor) + divisor) % divisor;

export const syntheticPan = (from: number, draggedPx: number, widthPx: number, hFov: number): number => mod(from - (draggedPx / coerceAtLeast(widthPx, 1)) * hFov + 180.0, 360.0) - 180.0;

export const syntheticLook = (from: Aim, dragged: { x: number; y: number }, size: { width: number; height: number }, hFov: number): Aim => [
  syntheticTilt(from[0], dragged.y, size.height, verticalFov(hFov, size.width, size.height)),
  syntheticPan(from[1], dragged.x, size.width, hFov),
];

export const syntheticAims = (view: Json | null, cameras: CamerasReading | null): boolean =>
  syntheticAvailable(view) && optBoolean(view, 'aimable') && (cameras?.cameras.some((it) => it.active && it.source === SYNTHETIC_SOURCE) ?? false);

export const useRememberSyntheticAims = (): boolean => {
  const view = useQgcPath(SYNTHETIC_VIEW);
  const camerasJson = useQgcPath(CAMERAS_VIEW);
  return useMemo(() => syntheticAims(view, camerasReading(camerasJson)), [view, camerasJson]);
};

export const useRememberSyntheticAvailable = (): boolean => useQgcDerived(SYNTHETIC_VIEW, syntheticAvailable);

export const syntheticChosen = (reading: CamerasReading | null): boolean => reading?.cameras.find((it) => it.active)?.source === SYNTHETIC_SOURCE;

const syntheticChosenOf = (json: Json | null): boolean => syntheticChosen(camerasReading(json));

export const useRememberSyntheticChosen = (): boolean => useQgcDerived(CAMERAS_VIEW, syntheticChosenOf);

type SyntheticWindow = Window & {
  aircast?: unknown;
  aircastHost?: { placed: () => void; failed: () => void };
  eval: (script: string) => unknown;
};

const servesSyntheticPage = async (): Promise<boolean> => {
  try {
    const page = await fetch(SYNTHETIC_PAGE);
    return page.ok && (await page.text()).includes(SYNTHETIC_HOST);
  } catch {
    return false;
  }
};

const evaluate = (frame: HTMLIFrameElement | null, script: string) => {
  try {
    (frame?.contentWindow as SyntheticWindow | null)?.eval(script);
  } catch {
    return;
  }
};

const useRegisteredPage = (draw: SyntheticDraw) => {
  const [shown] = useState(() => ({}));
  useEffect(() => {
    pages.set(shown, draw);
    return () => pages.remove(shown);
  }, [shown, draw]);
};

const SyntheticPage = ({ view, standIn }: { view: Json | null; standIn: string | null }) => {
  const overlays = useQgcPath(SYNTHETIC_OVERLAYS);
  const [loaded, setLoaded] = useState(false);
  const [draw, setDraw] = useState<SyntheticDraw>(SyntheticDraw.Loading);
  const [served, setServed] = useState(false);
  const frame = useRef<HTMLIFrameElement>(null);
  useRegisteredPage(draw);
  useLaunchedEffect([], async (scope) => {
    const found = await servesSyntheticPage();
    if (!scope.isActive()) return;
    if (found) setServed(true);
    else setDraw(SyntheticDraw.Failed);
  });
  useEffect(() => {
    if (view !== null && loaded) evaluate(frame.current, syntheticPoseScript(view));
  }, [view, loaded]);
  useEffect(() => {
    if (overlays !== null && loaded) evaluate(frame.current, syntheticOverlaysScript(overlays));
  }, [overlays, loaded]);
  const onLoad = (event: SyntheticEvent<HTMLIFrameElement>) => {
    const page = event.currentTarget.contentWindow as SyntheticWindow | null;
    if (page === null || page.aircast === undefined) {
      setDraw(SyntheticDraw.Failed);
      return;
    }
    page.aircastHost = { placed: () => setDraw(SyntheticDraw.Drawn), failed: () => setDraw(SyntheticDraw.Failed) };
    setLoaded(true);
  };
  return (
    <>
      {served ? <iframe ref={frame} src={SYNTHETIC_PAGE} title={SYNTHETIC_LABEL} onLoad={onLoad} className="pointer-events-none absolute inset-0 size-full border-0 bg-black" /> : <div className="absolute inset-0 bg-black" />}
      <SyntheticNote draw={draw} standIn={standIn} />
    </>
  );
};

const NativePage = ({ view, standIn }: { view: Json | null; standIn: string | null }) => {
  const draw = nativeDraw(view);
  useRegisteredPage(draw);
  return (
    <>
      <SyntheticSurface className="absolute inset-0 size-full" />
      <SyntheticNote draw={draw} standIn={standIn} />
    </>
  );
};

export type ShownNote = { readonly text: string; readonly atEdge: boolean };

export const shownNote = (draw: SyntheticDraw, standIn: string | null, announced: boolean): ShownNote | null =>
  draw.note !== null ? { text: draw.note, atEdge: false } : standIn !== null ? { text: standIn, atEdge: announced } : null;

export const gimbalAimsSynthetic = (view: Json | null, aimable: boolean): boolean => aimable && syntheticAvailable(view) && view !== null && !optBoolean(view, 'aimable', true);

const GIMBAL_STATE = 'view.gimbal';
const GIMBAL_AIM_COMMAND = 'gimbal.aim';
const GIMBAL_CENTER_COMMAND = 'gimbal.center';
const GIMBAL_AIM_EVERY_MS = 50;
const GIMBAL_AIM_SETTLES_MS = 1000;

export type Aim = readonly [number, number];

export const dragStart = (reported: Aim | null, sent: Aim | null, sentAgoMs: number): Aim => (sent !== null && sentAgoMs < GIMBAL_AIM_SETTLES_MS ? sent : (reported ?? [0.0, 0.0]));

export const gimbalAim = (view: Json | null): Aim | null => {
  const active = jsonObjects(optJSONArray(view, 'gimbals')).find((it) => optBoolean(it, 'active') && !isNull(it, 'pitch') && !isNull(it, 'bodyYaw'));
  return active === undefined ? null : [optDouble(active, 'pitch'), optDouble(active, 'bodyYaw')];
};

const uptimeMillis = (): number => performance.now();

export const SyntheticView = ({
  className,
  aimable = false,
  standIn = null,
  onGround = null,
  groundSpot = null,
}: {
  className?: string;
  aimable?: boolean;
  standIn?: string | null;
  onGround?: ((press: PicturePress) => void) | null;
  groundSpot?: PictureSpot | null;
}) => {
  const view = useQgcPath(SYNTHETIC_VIEW);
  const canAim = aimable && optBoolean(view, 'aimable');
  const aimedTilt = useUpdatedRef(view === null ? DEFAULT_TILT_DEG : optDouble(view, 'tilt', DEFAULT_TILT_DEG));
  const pan = useUpdatedRef(view === null ? 0.0 : optDouble(view, 'pan', 0.0));
  const fov = useUpdatedRef(view === null ? CAMERA_FOV_DEG : optDouble(view, 'fov', CAMERA_FOV_DEG));
  const sent = useMemo(() => ({ value: null as Aim | null }), [canAim]);
  const send = (tilt: number, turned: number) => {
    sent.value = [tilt, turned];
    void Qgc.invoke(SYNTHETIC_AIM, tilt, turned);
  };
  const steersGimbal = gimbalAimsSynthetic(view, aimable);
  const gimbalJson = useQgcPath(GIMBAL_STATE);
  const gimbalAt = useUpdatedRef(gimbalAim(gimbalJson));
  const lastAim = useRef<Aim | null>(null);
  const lastAimAt = useRef(0);
  const aimGimbal = (tilt: number, yaw: number) => {
    lastAim.current = [tilt, yaw];
    lastAimAt.current = uptimeMillis();
    void (async () => gimbalRefusal(await Qgc.call(GIMBAL_AIM_COMMAND, tilt, yaw)))();
  };
  const look = useRef({ aim: [DEFAULT_TILT_DEG, 0.0] as Aim, start: [0.0, 0.0] as Aim, moved: { x: 0, y: 0 }, sentAt: 0, size: { width: 1, height: 1 } });
  const gimbalTarget = (): Aim => syntheticLook(look.current.start, look.current.moved, look.current.size, fov.current);
  const aiming: PointerGestureHandlers | null = canAim
    ? {
        onDragStart: () => {
          look.current.aim = sent.value ?? [aimedTilt.current, pan.current];
        },
        onDrag: (_at, dragged, size) => {
          look.current.aim = syntheticLook(look.current.aim, dragged, size, fov.current);
          send(look.current.aim[0], look.current.aim[1]);
        },
        onDoubleTap: () => send(DEFAULT_TILT_DEG, 0.0),
      }
    : steersGimbal
      ? {
          onDragStart: () => {
            look.current.start = dragStart(gimbalAt.current, lastAim.current, uptimeMillis() - lastAimAt.current);
            look.current.moved = { x: 0, y: 0 };
          },
          onDragEnd: () => {
            const [tilt, yaw] = gimbalTarget();
            aimGimbal(tilt, yaw);
          },
          onDrag: (_at, dragged, size) => {
            const now = look.current;
            now.moved = { x: now.moved.x + dragged.x, y: now.moved.y + dragged.y };
            now.size = size;
            const at = uptimeMillis();
            if (at - now.sentAt < GIMBAL_AIM_EVERY_MS) return;
            now.sentAt = at;
            const [tilt, yaw] = gimbalTarget();
            aimGimbal(tilt, yaw);
          },
          onDoubleTap: () => void (async () => gimbalRefusal(await Qgc.call(GIMBAL_CENTER_COMMAND)))(),
        }
      : null;
  const gestures = usePointerGestures(aiming);
  const hold = usePictureHold(aimable ? onGround : null, true, true);
  return (
    <div {...gestures} {...hold} data-gesture={canAim ? 'look' : steersGimbal ? 'gimbal' : undefined} className={`overflow-hidden ${aiming !== null ? 'touch-none' : ''} ${className ?? ''}`}>
      <div className="relative size-full">
        {nativeChosen(view) ? <NativePage view={view} standIn={standIn} /> : <SyntheticPage view={view} standIn={standIn} />}
        {aimable && groundSpot !== null ? <PictureMark spot={groundSpot} /> : null}
      </div>
    </div>
  );
};

const SyntheticNote = ({ draw, standIn }: { draw: SyntheticDraw; standIn: string | null }) => {
  const standingIn = draw.note === null && standIn !== null;
  const announced = useProduceState(false, [standingIn], async (set) => {
    set(false);
    if (!standingIn) return;
    await delay(STAND_IN_ANNOUNCED_MS);
    set(true);
  });
  const note = shownNote(draw, standIn, announced);
  return note === null ? null : note.atEdge ? (
    <div
      role="status"
      aria-label={note.text}
      className="pointer-events-none absolute end-3 bottom-3 rounded-full px-2 py-0.5 select-none"
      style={{ background: withAlpha(Color.Black, EDGE_NOTE_SCRIM_ALPHA) }}
    >
      <Text typography="labelSmall" color={withAlpha(aircast('outdoorForeground'), EDGE_NOTE_TEXT_ALPHA)} maxLines={1}>
        {SYNTHETIC_LABEL}
      </Text>
    </div>
  ) : (
    <div className={`pointer-events-none absolute inset-0 flex items-center justify-center px-4 select-none ${osdShadow()}`}>
      <div role="status" aria-label={note.text} className="flex items-center gap-2">
        {draw === SyntheticDraw.Loading ? <CircularProgressIndicator size={NOTE_SPINNER} strokeWidth={NOTE_SPINNER_STROKE} color={aircast('outdoorForeground')} /> : null}
        <Text typography="labelLarge" color={aircast('outdoorForeground')} textAlign="center" maxLines={2}>
          {note.text}
        </Text>
      </div>
    </div>
  );
};
