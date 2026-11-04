import { type CSSProperties, type PointerEvent, type RefObject, useEffect, useRef, useState } from 'react';
import { type Json, optBoolean, optDouble } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { coerceAtLeast } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { Map3d } from '@/map/Map3d';
import { type SavedCamera, readCamera } from '@/map/MapCamera';
import { FlightMapPosition, type TrackPoint } from '@/map/VehicleMap';
import { Drawable } from '@/res/Icon';
import { AircastSpace, Color, aircast, md, withAlpha } from '@/theme/Colors';
import { LOOKING_TITLE } from '@/ui/ConnectingCard';
import { Offset, PASS_THROUGH, minus } from '@/ui/FlyShell';
import { osdShadow } from '@/ui/Osd';
import { SyntheticSurface, nativeDraw } from '@/ui/SyntheticSurface';
import { SYNTHETIC_VIEW, syntheticAvailable } from '@/ui/SyntheticView';
import { DOUBLE_TAP_TIMEOUT_MS, TOUCH_SLOP, usePointerGestures } from '@/ui/VideoGestures';

const MAP3D_VIEW = 'view.map3d';
const MAP3D_PAN = 'map3d.pan';
const MAP3D_TURN = 'map3d.turn';
const MAP3D_TILT = 'map3d.tilt';
const MAP3D_ZOOM = 'map3d.zoom';
const MAP3D_FOLLOW = 'map3d.follow';
const MAP3D_NORTH = 'map3d.north';
const MAP3D_OPEN = 'map3d.open';
const TURN_PER_VIEW_WIDTH_DEG = 180.0;
const WHEEL_ZOOM_RATE = 1.0 / 450.0;
const WHEEL_LINE_PX = 16.0;
const DOM_DELTA_LINE = 1;
const CONTROL_SCRIM_ALPHA = 0.55;
const CONTROL_SIZE = 40;
const CONTROL_ICON = 20;
export const BACK_TO_AIRCRAFT = 'Back to aircraft';
const COMPASS_LABEL = 'Compass. Drag to turn and tilt the view, tap to face north';
const MAP3D_TOUCH_HINT = 'Drag to move · Two fingers to turn and tilt';
const MAP3D_MOUSE_HINT = 'Drag to move · Right-drag or Shift-drag to turn and tilt';
const COARSE_POINTER = '(pointer: coarse)';
const HINT_SHOWN_MS = 8_000;
const NEEDLE_HALF_WIDTH = 0.22;
const NEEDLE_REACH = 0.8;
const NORTH = -1;
const SOUTH = 1;

export const map3dToggleText = (shown: boolean): string => (shown ? '2D' : '3D');

export const map3dToggleLabel = (shown: boolean): string => (shown ? 'Show the flat map' : 'Show the 3D map');

export const map3dHint = (coarse: boolean): string => (coarse ? MAP3D_TOUCH_HINT : MAP3D_MOUSE_HINT);

export type Map3dState = { readonly heading: number; readonly following: boolean; readonly aircraft: boolean };

export const map3dState = (view: Json | null): Map3dState => ({ heading: optDouble(view, 'heading', 0.0), following: optBoolean(view, 'following', true), aircraft: optBoolean(view, 'aircraft') });

export const map3dNote = (synthetic: Json | null, state: Map3dState): string | null => (!syntheticAvailable(synthetic) && state.following ? LOOKING_TITLE : nativeDraw(synthetic).note);

export const map3dOpenAt = (latest: TrackPoint | null, saved: SavedCamera | null): readonly number[] => {
  const at = latest ?? saved?.centre ?? null;
  return at === null ? [] : [at.latitude, at.longitude];
};

export const map3dControlsAt = (bottomInsetPx: number, logoEndInsetPx: number | null): CSSProperties =>
  logoEndInsetPx === null ? { padding: AircastSpace.s3 } : { paddingRight: logoEndInsetPx, paddingBottom: bottomInsetPx + AircastSpace.s3 };

export type Pair = readonly [Offset, Offset];

export type PinchStep = { readonly scale: number; readonly turn: number; readonly up: number };

const span = ([a, b]: Pair): number => Math.hypot(b.x - a.x, b.y - a.y);

const angle = ([a, b]: Pair): number => (Math.atan2(b.y - a.y, b.x - a.x) * 180.0) / Math.PI;

const wrapped = (degrees: number): number => ((((degrees + 180.0) % 360.0) + 360.0) % 360.0) - 180.0;

export const pinchStep = (before: Pair, after: Pair): PinchStep => ({
  scale: span(before) > 0 ? span(after) / span(before) : 1.0,
  turn: wrapped(angle(after) - angle(before)),
  up: (before[0].y + before[1].y - after[0].y - after[1].y) / 2,
});

export const mouseTurn = (dx: number, widthPx: number): number => (dx / coerceAtLeast(widthPx, 1)) * TURN_PER_VIEW_WIDTH_DEG;

export const wheelScale = (deltaY: number, deltaMode: number): number => Math.exp(-(deltaMode === DOM_DELTA_LINE ? deltaY * WHEEL_LINE_PX : deltaY) * WHEEL_ZOOM_RATE);

const pairOf = (pointers: ReadonlyMap<number, Offset>): Pair | null => {
  const [a, b] = [...pointers.values()];
  return a !== undefined && b !== undefined ? [a, b] : null;
};

const send = (path: string, ...args: number[]) => void Qgc.invoke(path, ...args);

const needle = (direction: number): string => {
  const centre = CONTROL_ICON / 2;
  const half = centre * NEEDLE_HALF_WIDTH;
  return `M ${centre} ${centre + centre * NEEDLE_REACH * direction} L ${centre + half} ${centre} L ${centre - half} ${centre} Z`;
};

const capture = (element: Element, pointer: number) => {
  try {
    element.setPointerCapture(pointer);
  } catch {
    return;
  }
};

const Compass = ({ heading, view }: { heading: number; view: RefObject<HTMLDivElement | null> }) => {
  const dragged = useRef(false);
  const gestures = usePointerGestures({
    onDragStart: () => {
      dragged.current = true;
    },
    onDrag: (_at, moved) => {
      const box = view.current;
      if (box === null) return;
      send(MAP3D_TURN, mouseTurn(moved.x, box.clientWidth));
      send(MAP3D_TILT, -moved.y, box.clientHeight);
    },
  });
  return (
    <button
      type="button"
      {...gestures}
      onPointerDownCapture={() => {
        dragged.current = false;
      }}
      onClick={() => (dragged.current ? undefined : send(MAP3D_NORTH))}
      aria-label={COMPASS_LABEL}
      title={COMPASS_LABEL}
      className="grid touch-none place-items-center rounded-full"
      style={{ width: CONTROL_SIZE, height: CONTROL_SIZE, background: withAlpha(Color.Black, CONTROL_SCRIM_ALPHA), color: aircast('outdoorForeground') }}
    >
      <svg width={CONTROL_ICON} height={CONTROL_ICON} viewBox={`0 0 ${CONTROL_ICON} ${CONTROL_ICON}`} aria-hidden style={{ transform: `rotate(${-heading}deg)` }}>
        <path d={needle(NORTH)} style={{ fill: md('error') }} />
        <path d={needle(SOUTH)} style={{ fill: aircast('outdoorForeground') }} />
      </svg>
    </button>
  );
};

const Hint = () => (
  <Surface
    shape="full"
    color={withAlpha(Color.Black, CONTROL_SCRIM_ALPHA)}
    contentColor={aircast('outdoorForeground')}
    style={{ paddingInline: AircastSpace.s3, paddingBlock: AircastSpace.s2 }}
  >
    <Text typography="labelLarge">{map3dHint(window.matchMedia(COARSE_POINTER).matches)}</Text>
  </Surface>
);

const BackToAircraft = () => (
  <Surface
    onClick={() => send(MAP3D_FOLLOW)}
    shape="full"
    color={withAlpha(Color.Black, CONTROL_SCRIM_ALPHA)}
    contentColor={aircast('outdoorForeground')}
    className="inline-flex items-center"
    style={{ gap: AircastSpace.s2, paddingInline: AircastSpace.s3, paddingBlock: AircastSpace.s2 }}
  >
    <Drawable name="ic_my_location" size={CONTROL_ICON} />
    <Text typography="labelLarge">{BACK_TO_AIRCRAFT}</Text>
  </Surface>
);

export const Map3dView = ({ className, bottomInsetPx, logoEndInsetPx }: { className: string; bottomInsetPx: number; logoEndInsetPx: number | null }) => {
  const synthetic = useQgcPath(SYNTHETIC_VIEW);
  const state = map3dState(useQgcPath(MAP3D_VIEW));
  const note = map3dNote(synthetic, state);
  const view = useRef<HTMLDivElement>(null);
  const host = useRef<HTMLDivElement>(null);
  const [hinting, setHinting] = useState(() => !Map3d.hinted());
  useEffect(() => send(MAP3D_OPEN, ...map3dOpenAt(FlightMapPosition.latest, readCamera())), []);
  useEffect(() => {
    if (!hinting) return undefined;
    Map3d.markHinted();
    const timer = setTimeout(() => setHinting(false), HINT_SHOWN_MS);
    return () => clearTimeout(timer);
  }, [hinting]);
  const press = useRef({ pointers: new Map() as ReadonlyMap<number, Offset>, from: Offset(0, 0), moving: false, twist: false, second: false, pinched: false, tapUpAt: -Infinity });
  useEffect(() => {
    const element = host.current;
    if (element === null) return undefined;
    const zoom = (event: WheelEvent) => {
      event.preventDefault();
      send(MAP3D_ZOOM, wheelScale(event.deltaY, event.deltaMode));
    };
    element.addEventListener('wheel', zoom, { passive: false });
    return () => element.removeEventListener('wheel', zoom);
  }, []);
  const down = (event: PointerEvent<HTMLDivElement>) => {
    setHinting(false);
    const now = press.current;
    const at = Offset(event.clientX, event.clientY);
    capture(event.currentTarget, event.pointerId);
    now.pointers = new Map([...now.pointers, [event.pointerId, at]]);
    if (now.pointers.size > 1) {
      now.moving = true;
      now.second = false;
      now.pinched = true;
      return;
    }
    now.from = at;
    now.moving = false;
    now.pinched = false;
    now.twist = event.pointerType === 'mouse' && (event.button === 2 || event.shiftKey);
    now.second = event.timeStamp - now.tapUpAt <= DOUBLE_TAP_TIMEOUT_MS;
  };
  const move = (event: PointerEvent<HTMLDivElement>) => {
    const now = press.current;
    const last = now.pointers.get(event.pointerId);
    if (last === undefined) return;
    const at = Offset(event.clientX, event.clientY);
    if (!now.moving && Math.hypot(at.x - now.from.x, at.y - now.from.y) <= TOUCH_SLOP) return;
    const before = pairOf(now.pointers);
    now.moving = true;
    now.second = false;
    now.pointers = new Map(now.pointers).set(event.pointerId, at);
    const after = pairOf(now.pointers);
    const width = event.currentTarget.clientWidth;
    const height = event.currentTarget.clientHeight;
    if (before !== null && after !== null) {
      const step = pinchStep(before, after);
      if (step.scale !== 1) send(MAP3D_ZOOM, step.scale);
      if (step.turn !== 0) send(MAP3D_TURN, step.turn);
      if (step.up !== 0) send(MAP3D_TILT, step.up, height);
      return;
    }
    const moved = minus(at, last);
    if (now.pinched || (moved.x === 0 && moved.y === 0)) return;
    if (now.twist) {
      if (moved.x !== 0) send(MAP3D_TURN, mouseTurn(moved.x, width));
      if (moved.y !== 0) send(MAP3D_TILT, -moved.y, height);
      return;
    }
    send(MAP3D_PAN, moved.x, moved.y, width);
  };
  const up = (event: PointerEvent<HTMLDivElement>, cancelled: boolean) => {
    const now = press.current;
    if (!now.pointers.has(event.pointerId)) return;
    now.pointers = new Map([...now.pointers].filter(([pointer]) => pointer !== event.pointerId));
    const tapped = now.pointers.size === 0 && !now.moving && !now.twist && !cancelled;
    if (tapped && now.second) send(MAP3D_FOLLOW);
    now.tapUpAt = tapped && !now.second ? event.timeStamp : -Infinity;
  };
  return (
    <div ref={view} className={`relative touch-none overflow-hidden bg-black select-none ${className}`} onContextMenu={(event) => event.preventDefault()}>
      <SyntheticSurface className="absolute inset-0 size-full" orbit />
      <div
        ref={host}
        className="absolute inset-0"
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={(event) => up(event, false)}
        onPointerCancel={(event) => up(event, true)}
      />
      {note === null ? null : (
        <div role="status" aria-label={note} className={`pointer-events-none absolute inset-0 flex items-center justify-center px-4 ${osdShadow()}`}>
          <Text typography="labelLarge" color={aircast('outdoorForeground')} textAlign="center" maxLines={2}>
            {note}
          </Text>
        </div>
      )}
      <div className={`absolute right-0 bottom-0 flex flex-col items-end ${PASS_THROUGH}`} style={{ gap: AircastSpace.s2, ...map3dControlsAt(bottomInsetPx, logoEndInsetPx) }}>
        {hinting ? <Hint /> : null}
        <div className="flex items-center" style={{ gap: AircastSpace.s2 }}>
          {state.aircraft && !state.following ? <BackToAircraft /> : null}
          <Compass heading={state.heading} view={view} />
        </div>
      </div>
    </div>
  );
};
