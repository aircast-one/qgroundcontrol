import { type HTMLAttributes, type KeyboardEvent, type PointerEvent as ReactPointerEvent, type SyntheticEvent, useEffect, useRef, useState } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';
import { getSharedPreferences } from '@/kotlin/Platform';
import { coerceIn } from '@/kotlin/Std';
import { Drawable } from '@/res/Icon';
import { Color } from '@/theme/Colors';
import { FLY_STORE, Offset, Size } from '@/ui/FlyShell';
import { OSD_SCRIM } from '@/ui/Osd';

const PIP_PLACE_KEY = 'pipPlace';
const PIP_SCALE_KEY = 'pipScale';
const PIP_DRAG_SLOP = 8;
const PIP_SMALLEST = 0.6;
const PIP_WIDEST_SHARE = 0.7;
const PIP_GRIP_SIZE = 28;
const PIP_KEY_STEP = 0.1;

export const pipWithin = (moved: Offset, room: Size): Offset => Offset(coerceIn(moved.x, 0, Math.max(0, room.width)), coerceIn(moved.y, Math.min(0, -room.height), 0));

export const storedPipPlace = (text: string | null): Offset => {
  const [x, y] = (text ?? '').split(',').map(Number);
  return x !== undefined && y !== undefined && Number.isFinite(x) && Number.isFinite(y) ? Offset(x, y) : Offset(0, 0);
};

export const storedPipScale = (text: string | null): number => {
  const scale = Number(text ?? Number.NaN);
  return Number.isFinite(scale) && scale > 0 ? scale : 1;
};

export const pipLargestScale = (base: Size, space: Size): number => Math.max(PIP_SMALLEST, Math.min((space.width * PIP_WIDEST_SHARE) / base.width, space.height / base.height));

export const pipScaleWithin = (scale: number, largest: number): number => coerceIn(scale, PIP_SMALLEST, Math.max(PIP_SMALLEST, largest));

export const scaledPip = (base: Size, scale: number): Size => Size(Math.round(base.width * scale), Math.round(base.height * scale));

export const gripScale = (start: number, base: Size, dx: number, dy: number): number => {
  const across = dx / base.width;
  const up = -dy / base.height;
  return start + (Math.abs(across) >= Math.abs(up) ? across : up);
};

export type PipShown = { readonly moved: Offset; readonly scale: number };
export type PipRoom = { readonly base: Size; readonly space: Size; readonly scalable: boolean };

const drawnScale = (scale: number, room: PipRoom): number => (room.scalable ? pipScaleWithin(scale, pipLargestScale(room.base, room.space)) : 1);

const placeWithin = (moved: Offset, scale: number, room: PipRoom): Offset => {
  const size = scaledPip(room.base, drawnScale(scale, room));
  return pipWithin(moved, Size(room.space.width - size.width, room.space.height - size.height));
};

export const pipDrawn = (stored: PipShown, room: PipRoom): PipShown => {
  const scale = drawnScale(stored.scale, room);
  return { scale, moved: placeWithin(stored.moved, scale, room) };
};

export const pipKept = (dropped: PipShown, drawn: PipShown, stored: PipShown, room: PipRoom): PipShown => {
  const scale = room.scalable && dropped.scale !== drawn.scale ? drawnScale(dropped.scale, room) : stored.scale;
  const moved = dropped.moved.x === drawn.moved.x && dropped.moved.y === drawn.moved.y ? stored.moved : placeWithin(dropped.moved, scale, room);
  return { scale, moved };
};

export type PipGesture =
  | { readonly kind: 'move'; readonly from: Offset; readonly start: PipShown }
  | { readonly kind: 'pinch'; readonly spread: number; readonly start: PipShown }
  | { readonly kind: 'grip'; readonly from: Offset; readonly start: PipShown };

export const pipSpread = (points: ReadonlyMap<number, Offset>): number => {
  const [a, b] = [...points.values()];
  return a === undefined || b === undefined ? 0 : Math.hypot(a.x - b.x, a.y - b.y);
};

export const pipJoined = (gesture: PipGesture, points: ReadonlyMap<number, Offset>, shown: PipShown | null, scalable: boolean): PipGesture =>
  scalable && gesture.kind !== 'grip' && points.size >= 2 ? { kind: 'pinch', spread: pipSpread(points), start: shown ?? gesture.start } : gesture;

export const pipFollowed = (gesture: PipGesture, points: ReadonlyMap<number, Offset>, pointer: Offset, dragging: boolean, base: Size): PipShown | null => {
  if (gesture.kind === 'pinch') {
    const spread = pipSpread(points);
    return spread > 0 && gesture.spread > 0 ? { moved: gesture.start.moved, scale: (gesture.start.scale * spread) / gesture.spread } : null;
  }
  const [dx, dy] = [pointer.x - gesture.from.x, pointer.y - gesture.from.y];
  if (gesture.kind === 'grip') return { moved: gesture.start.moved, scale: gripScale(gesture.start.scale, base, dx, dy) };
  return !dragging && Math.hypot(dx, dy) < PIP_DRAG_SLOP ? null : { moved: Offset(gesture.start.moved.x + dx, gesture.start.moved.y + dy), scale: gesture.start.scale };
};

export type PipFrame = { moved: Offset; size: Size; adjusting: boolean; handlers: HTMLAttributes<HTMLElement>; grip: HTMLAttributes<HTMLElement> };

const readStored = (): PipShown => {
  const store = getSharedPreferences(FLY_STORE);
  return { moved: storedPipPlace(store.getString(PIP_PLACE_KEY, null)), scale: storedPipScale(store.getString(PIP_SCALE_KEY, null)) };
};

const writeStored = (kept: PipShown, before: PipShown): void => {
  const store = getSharedPreferences(FLY_STORE);
  if (kept.moved.x !== before.moved.x || kept.moved.y !== before.moved.y) store.putString(PIP_PLACE_KEY, `${kept.moved.x},${kept.moved.y}`);
  if (kept.scale !== before.scale) store.putString(PIP_SCALE_KEY, `${kept.scale}`);
};

export const usePipFrame = (base: Size, space: Size, scalable: boolean): PipFrame => {
  const [stored, setStored] = useState(readStored);
  const [live, setLive] = useState<PipShown | null>(null);
  const room: PipRoom = { base, space, scalable };
  const rest = pipDrawn(stored, room);
  const drawn = live === null ? rest : pipDrawn(live, room);
  const latest = useUpdatedRef({ stored, rest, room });
  const points = useRef<ReadonlyMap<number, Offset>>(new Map());
  const gesture = useRef<PipGesture | null>(null);
  const shown = useRef<PipShown | null>(null);
  const dropped = useRef(false);
  const release = useRef<() => void>(() => undefined);
  useEffect(() => () => release.current(), []);
  const keep = (kept: PipShown) => {
    writeStored(kept, latest.current.stored);
    setStored(kept);
  };
  const settle = (): PipShown | null => {
    release.current();
    release.current = () => undefined;
    points.current = new Map();
    gesture.current = null;
    const kept = shown.current;
    shown.current = null;
    setLive(null);
    return kept;
  };
  const follow = (event: PointerEvent) => {
    const at = gesture.current;
    if (at === null || !points.current.has(event.pointerId)) return;
    points.current = new Map(points.current).set(event.pointerId, Offset(event.clientX, event.clientY));
    const next = pipFollowed(at, points.current, Offset(event.clientX, event.clientY), shown.current !== null, latest.current.room.base);
    if (next === null) return;
    shown.current = next;
    setLive(next);
  };
  const end = (event: PointerEvent) => {
    if (!points.current.has(event.pointerId)) return;
    points.current = new Map([...points.current].filter(([pointer]) => pointer !== event.pointerId));
    if (points.current.size > 0) return;
    const kept = settle();
    if (kept === null) return;
    const now = latest.current;
    keep(pipKept(kept, now.rest, now.stored, now.room));
    dropped.current = true;
    window.setTimeout(() => {
      dropped.current = false;
    }, 0);
  };
  const join = (event: PointerEvent) => {
    const at = gesture.current;
    if (at === null || points.current.has(event.pointerId)) return;
    event.stopPropagation();
    points.current = new Map(points.current).set(event.pointerId, Offset(event.clientX, event.clientY));
    gesture.current = pipJoined(at, points.current, shown.current, latest.current.room.scalable);
  };
  const shield = (event: Event) => {
    if (gesture.current !== null) event.stopPropagation();
  };
  const press = (event: ReactPointerEvent<HTMLElement>, gripped: boolean) => {
    if (event.button !== 0) return;
    if (gripped) event.stopPropagation();
    if (gesture.current !== null) settle();
    const listeners: readonly (readonly [string, (event: never) => void, boolean])[] = [
      ['pointermove', follow, false],
      ['pointerup', end, false],
      ['pointercancel', end, false],
      ['pointerdown', join, true],
      ['touchstart', shield, true],
      ['blur', settle, false],
    ];
    listeners.map(([name, listener, capture]) => window.addEventListener(name, listener as EventListener, capture));
    release.current = () => listeners.map(([name, listener, capture]) => window.removeEventListener(name, listener as EventListener, capture));
    const from = Offset(event.clientX, event.clientY);
    points.current = new Map([[event.pointerId, from]]);
    dropped.current = false;
    gesture.current = { kind: gripped ? 'grip' : 'move', from, start: latest.current.rest };
  };
  const swallowDrop = (event: SyntheticEvent) => {
    if (!dropped.current) return;
    dropped.current = false;
    event.stopPropagation();
    event.preventDefault();
  };
  const resizeByKey = (event: KeyboardEvent<HTMLElement>) => {
    const step = event.key === '+' || event.key === '=' ? PIP_KEY_STEP : event.key === '-' ? -PIP_KEY_STEP : 0;
    const now = latest.current;
    if (step === 0 || !now.room.scalable) return;
    event.preventDefault();
    event.stopPropagation();
    keep(pipKept({ moved: now.rest.moved, scale: now.rest.scale + step }, now.rest, now.stored, now.room));
  };
  return {
    moved: drawn.moved,
    size: scaledPip(base, drawn.scale),
    adjusting: live !== null,
    handlers: { onPointerDown: (event) => press(event, false), onClickCapture: swallowDrop, onKeyDownCapture: resizeByKey, 'aria-keyshortcuts': scalable ? '+ -' : undefined },
    grip: { onPointerDown: (event) => press(event, true), onClick: (event) => event.stopPropagation() },
  };
};

export const PipGrip = ({ grip }: { grip: HTMLAttributes<HTMLElement> }) => (
  <div
    {...grip}
    data-testid="pip-resize"
    aria-hidden
    className="pointer-events-none absolute end-1 top-1 hidden cursor-nesw-resize touch-none place-items-center rounded-full opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100 pointer-fine:grid"
    style={{ width: PIP_GRIP_SIZE, height: PIP_GRIP_SIZE, background: OSD_SCRIM, color: Color.White }}
  >
    <Drawable name="ic_fullscreen" size={16} />
  </div>
);
