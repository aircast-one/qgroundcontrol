import { type PointerEvent, useEffect, useRef } from 'react';
import type { Json } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { coerceAtLeast } from '@/kotlin/Std';
import { Text } from '@/m3';
import { aircast } from '@/theme/Colors';
import { LOOKING_TITLE } from '@/ui/ConnectingCard';
import { Offset, minus } from '@/ui/FlyShell';
import { osdShadow } from '@/ui/Osd';
import { SyntheticSurface, nativeDraw } from '@/ui/SyntheticSurface';
import { SYNTHETIC_VIEW, syntheticAvailable } from '@/ui/SyntheticView';
import { DOUBLE_TAP_TIMEOUT_MS, TOUCH_SLOP } from '@/ui/VideoGestures';

const MAP3D_PAN = 'map3d.pan';
const MAP3D_TURN = 'map3d.turn';
const MAP3D_TILT = 'map3d.tilt';
const MAP3D_ZOOM = 'map3d.zoom';
const MAP3D_FOLLOW = 'map3d.follow';
const TURN_PER_VIEW_WIDTH_DEG = 180.0;
const WHEEL_ZOOM_RATE = 1.0 / 450.0;
const WHEEL_LINE_PX = 16.0;
const DOM_DELTA_LINE = 1;

export const map3dNote = (view: Json | null): string | null => (syntheticAvailable(view) ? nativeDraw(view).note : LOOKING_TITLE);

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

const capture = (element: Element, pointer: number) => {
  try {
    element.setPointerCapture(pointer);
  } catch {
    return;
  }
};

export const Map3dView = ({ className }: { className?: string }) => {
  const view = useQgcPath(SYNTHETIC_VIEW);
  const note = map3dNote(view);
  const host = useRef<HTMLDivElement>(null);
  const press = useRef({ pointers: new Map() as ReadonlyMap<number, Offset>, from: Offset(0, 0), moving: false, twist: false, second: false, tapUpAt: -Infinity });
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
    const now = press.current;
    const at = Offset(event.clientX, event.clientY);
    capture(event.currentTarget, event.pointerId);
    now.pointers = new Map([...now.pointers, [event.pointerId, at]]);
    if (now.pointers.size > 1) {
      now.moving = true;
      now.second = false;
      return;
    }
    now.from = at;
    now.moving = false;
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
      send(MAP3D_ZOOM, step.scale);
      send(MAP3D_TURN, step.turn);
      send(MAP3D_TILT, step.up, height);
      return;
    }
    const moved = minus(at, last);
    if (now.twist) {
      send(MAP3D_TURN, mouseTurn(moved.x, width));
      send(MAP3D_TILT, -moved.y, height);
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
    <div
      ref={host}
      className={`relative touch-none overflow-hidden bg-black select-none ${className ?? ''}`}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={(event) => up(event, false)}
      onPointerCancel={(event) => up(event, true)}
      onContextMenu={(event) => event.preventDefault()}
    >
      <SyntheticSurface className="absolute inset-0 size-full" orbit />
      {note === null ? null : (
        <div role="status" aria-label={note} className={`pointer-events-none absolute inset-0 flex items-center justify-center px-4 ${osdShadow()}`}>
          <Text typography="labelLarge" color={aircast('outdoorForeground')} textAlign="center" maxLines={2}>
            {note}
          </Text>
        </div>
      )}
    </div>
  );
};
