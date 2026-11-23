import { type CSSProperties, type Dispatch, type HTMLAttributes, type PointerEvent, type ReactNode, type RefCallback, type RefObject, type SetStateAction, createContext, useCallback, useContext, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Button, DropdownMenu, DropdownMenuItem, Surface, Text } from '@/m3';
import { useEscapeHandler, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { type SharedPreferences, getSharedPreferences, TAP } from '@/kotlin/Platform';
import { type MutableState, mutableStateOf, useValue } from '@/kotlin/State';
import { coerceAtLeast, coerceIn, delay } from '@/kotlin/Std';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { AircastSpace, aircast, md } from '@/theme/Colors';
import { LocalAvoidedByVideoMessage, STATUS_ROW_HEIGHT, useAvoidedByVideoMessage, useFlyIsPortrait } from '@/ui/FlyShell';
import { LocalFlyScreenState } from '@/ui/FlyScreenState';
import { useArmed } from '@/ui/FlyState';
import { INSTRUMENTS_VIEW, ValueSize, instrumentVehicleClass, nextValueSize, valueSizePillText, writeValueSize } from '@/ui/InstrumentChoice';
import { LocalFlyOsd, OSD_SCRIM, osdShadow, useOsdBackdrop } from '@/ui/Osd';

const LAYOUT_STORE = 'fly-overlay-layout';
const HIDDEN_PREFIX = 'OverlayRigHidden-';
const OFFSET_PREFIX = 'OverlayRigOffset-';
const SIZE_PREFIX = 'OverlayRigSize-';
const LANDSCAPE_SUFFIX = '@landscape';
const HIDDEN_ALPHA = 0.35;
const JIGGLE_DEGREES = 1.2;
const JIGGLE_MILLIS = 120;
const JIGGLE_SPREAD = 0x1f;
const JIGGLE_REACH_PX = 4;
const JIGGLE_ANGLE = '--aircast-jiggle';
const LIFT_SCALE = 1.1;
const LIFT_MILLIS = 220;
const LIFT_EASING = 'cubic-bezier(0.34, 1.56, 0.64, 1)';
const LIFT_SHADOW = 'drop-shadow(0 6px 10px rgb(0 0 0 / 0.5))';
export const BADGE_SIZE = 18;
const LONG_PRESS_TIMEOUT_MS = 400;
const TOUCH_SLOP = 8;
export const RESET_ARM_MILLIS = 4000;

export type ResetTap = { reset: boolean; armed: boolean };

export const ResetTap = (fields: ResetTap): ResetTap => ({ ...fields });

export const resetTap = (armed: boolean): ResetTap => ResetTap({ reset: armed, armed: !armed });

export const resetPillText = (armed: boolean): string => (armed ? `${TAP} again to reset` : 'Reset positions and hidden items');

export const LAYOUT_EDITING_IN_FLIGHT = 'Editing the layout. Return, Land and Kill motors still work.';

const LAYOUT_NOTICE_MAX_WIDTH = 560;

const INDICATOR_ORDER = 'FlyViewIndicatorOrder';
const DOCKED_READINGS = 'FlyViewDockedReadings';
const FLOATED_INDICATORS = 'FlyViewFloatedIndicators';

export type Offset = readonly [number, number];

export type Bounds = { readonly left: number; readonly top: number; readonly right: number; readonly bottom: number };

export type Size = { readonly width: number; readonly height: number };

export type Carry = { readonly item: Element; readonly pointerId: number; readonly x: number; readonly y: number };

export type Landing = { readonly key: string; readonly x: number; readonly y: number };

const storedLayout = (): Record<string, unknown> =>
  Object.fromEntries(Object.entries(getSharedPreferences(LAYOUT_STORE).getAll()).map(([key, text]) => [key, text === 'true' ? true : text === 'false' ? false : text]));

export class OverlayLayoutState {
  readonly editing = mutableStateOf(false);
  readonly hidden: MutableState<ReadonlySet<string>>;
  readonly indicatorOrder: MutableState<readonly string[]>;
  readonly valueSize = mutableStateOf<ValueSize>(ValueSize.Default);
  readonly offsets: MutableState<ReadonlyMap<string, Offset>>;
  readonly docked: MutableState<readonly string[]>;
  readonly floated: MutableState<readonly string[]>;
  readonly sizes: MutableState<ReadonlyMap<string, number>>;
  carry: Carry | null = null;
  landing: Landing | null = null;

  constructor(private readonly prefs: SharedPreferences) {
    const stored = storedLayout();
    this.hidden = mutableStateOf(hiddenKeys(stored));
    this.indicatorOrder = mutableStateOf(storedIndicatorOrder(stored));
    this.offsets = mutableStateOf(storedOffsets(stored));
    this.docked = mutableStateOf(storedKeys(stored, DOCKED_READINGS));
    this.floated = mutableStateOf(storedKeys(stored, FLOATED_INDICATORS));
    this.sizes = mutableStateOf(storedSizes(stored));
  }

  resize(key: string): void {
    const next = nextItemSize(this.sizes.value.get(key) ?? 1);
    this.prefs.putString(SIZE_PREFIX + key, String(next));
    this.sizes.value = new Map(this.sizes.value).set(key, next);
  }

  saveDocked(keys: readonly string[]): void {
    this.prefs.putString(DOCKED_READINGS, keys.join(','));
    this.docked.value = keys;
  }

  saveFloated(keys: readonly string[]): void {
    this.prefs.putString(FLOATED_INDICATORS, keys.join(','));
    this.floated.value = keys;
  }

  startEditing(carry: Carry | null = null): void {
    this.carry = carry;
    this.editing.value = true;
  }

  setHidden(key: string, hide: boolean): void {
    this.prefs.putBoolean(HIDDEN_PREFIX + key, hide);
    this.hidden.value = withHidden(this.hidden.value, key, hide);
  }

  saveIndicatorOrder(keys: readonly string[]): void {
    this.prefs.putString(INDICATOR_ORDER, keys.join(','));
    this.indicatorOrder.value = keys;
  }

  nudge(key: string, dx: number, dy: number, from: Offset = NO_OFFSET): void {
    const [x, y] = this.offsets.value.get(key) ?? from;
    this.offsets.value = new Map(this.offsets.value).set(key, [x + dx, y + dy]);
  }

  forget(keys: readonly string[]): void {
    const placed = keys.flatMap((key) => [key, orientedKey(key, true)]);
    [...placed.flatMap((key) => [OFFSET_PREFIX + key, HIDDEN_PREFIX + key]), ...keys.map((key) => SIZE_PREFIX + key)].forEach((name) => this.prefs.remove(name));
    this.sizes.value = new Map([...this.sizes.value].filter(([key]) => !keys.includes(key)));
    this.offsets.value = new Map([...this.offsets.value].filter(([key]) => !placed.includes(key)));
    this.hidden.value = new Set([...this.hidden.value].filter((key) => !keys.includes(key)));
    if (this.docked.value.some((key) => keys.includes(key))) this.saveDocked(this.docked.value.filter((key) => !keys.includes(key)));
  }

  saveOffset(key: string): void {
    const offset = this.offsets.value.get(key);
    if (offset) this.prefs.putString(OFFSET_PREFIX + key, `${offset[0]},${offset[1]}`);
  }

  reset(): void {
    Object.keys(storedLayout()).forEach((name) => this.prefs.remove(name));
    this.hidden.value = hiddenKeys({});
    this.indicatorOrder.value = [];
    this.offsets.value = new Map();
    this.docked.value = [];
    this.floated.value = [];
    this.sizes.value = new Map();
  }
}

export const overlayLayoutStore = (): SharedPreferences => getSharedPreferences(LAYOUT_STORE);

const storedKeys = (stored: Readonly<Record<string, unknown>>, name: string): string[] => {
  const keys = stored[name];
  return (typeof keys === 'string' ? keys : '').split(',').filter((it) => it.trim() !== '');
};

export const storedIndicatorOrder = (stored: Readonly<Record<string, unknown>>): string[] => storedKeys(stored, INDICATOR_ORDER);

export const INDICATOR_CELL = 'data-indicator';

export const PULL_OUT_PX = 40;

export type BarCell = { readonly key: string; readonly centre: number; readonly shown: boolean };

export const droppedIntoBar = (cells: readonly BarCell[], key: string, x: number): string[] => {
  const others = cells.filter((cell) => cell.key !== key);
  const before = others.findIndex((cell) => cell.shown && cell.centre > x);
  const keys = others.map((cell) => cell.key);
  return before < 0 ? [...keys, key] : [...keys.slice(0, before), key, ...keys.slice(before)];
};

export const barOrderWith = (key: string, x: number): string[] =>
  droppedIntoBar(
    Array.from(document.querySelectorAll<HTMLElement>(`[${INDICATOR_CELL}]`)).map((node) => {
      const box = node.getBoundingClientRect();
      return { key: node.getAttribute(INDICATOR_CELL) ?? '', centre: (box.left + box.right) / 2, shown: box.width > 0 };
    }),
    key,
    x,
  );

export const onScreenCorrection = (bounds: Bounds, area: Bounds, offset: Offset): Offset => {
  const [dx, dy] = clampedInto(bounds, area, 0, 0);
  return [pulledBack(bounds.right - bounds.left > area.right - area.left ? 0 : dx, offset[0]), pulledBack(bounds.bottom - bounds.top > area.bottom - area.top ? 0 : dy, offset[1])];
};

export const shownCorrection = (bounds: Bounds, area: Bounds, offset: Offset, shown: Offset): Offset =>
  onScreenCorrection({ left: bounds.left - shown[0], top: bounds.top - shown[1], right: bounds.right - shown[0], bottom: bounds.bottom - shown[1] }, area, offset);

export const visibleArea = (root: Size, reserved: Bounds): Bounds => ({
  left: reserved.left,
  top: reserved.top,
  right: root.width - reserved.right,
  bottom: root.height - reserved.bottom,
});

export const pulledBack = (correction: number, offset: number): number => {
  if (offset > 0) return coerceIn(correction, -offset, 0);
  if (offset < 0) return coerceIn(correction, 0, -offset);
  return 0;
};

export const orientedKey = (key: string, landscape: boolean): string => (landscape ? `${key}${LANDSCAPE_SUFFIX}` : key);

export const orderedKeys = (available: readonly string[], order: readonly string[]): string[] => [
  ...order.filter((it) => available.includes(it)),
  ...available.filter((it) => !order.includes(it)),
];

export const movedKey = (keys: readonly string[], key: string, delta: number): string[] | null => {
  const from = keys.indexOf(key);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= keys.length) return null;
  const rest = keys.filter((_, index) => index !== from);
  return [...rest.slice(0, to), key, ...rest.slice(to)];
};

export const reorderedAmongShown = (keys: readonly string[], shownOrder: readonly string[]): string[] => {
  const slots = keys.flatMap((key, index) => (shownOrder.includes(key) ? [index] : []));
  const placed = new Map(slots.map((slot, at) => [slot, shownOrder[at]]));
  return keys.map((key, index) => placed.get(index) ?? key);
};

export const movedAmongShown = (keys: readonly string[], shown: ReadonlySet<string>, key: string, delta: number): string[] | null => {
  const moved = movedKey(
    keys.filter((it) => shown.has(it)),
    key,
    delta,
  );
  return moved === null ? null : reorderedAmongShown(keys, moved);
};

const nearestIndex = (lines: readonly number[], y: number): number => lines.reduce((best, line, index) => (Math.abs(line - y) < Math.abs((lines[best] ?? 0) - y) ? index : best), 0);

export const droppedOrder = (keys: readonly string[], centres: ReadonlyMap<string, Offset>, key: string, dx: number, dy: number): string[] => {
  const lines = [...new Set([...centres.values()].map(([, y]) => Math.round(y)))].toSorted((a, b) => a - b);
  const place = (shown: string): Offset => {
    const [x, y] = centres.get(shown) ?? NO_OFFSET;
    return shown === key ? [nearestIndex(lines, y + dy), x + dx] : [nearestIndex(lines, y), x];
  };
  return reorderedAmongShown(
    keys,
    keys.filter((it) => centres.has(it)).toSorted((a, b) => place(a)[0] - place(b)[0] || place(a)[1] - place(b)[1]),
  );
};

export const READING_BAND_PX = 120;

export const rowsCovered = ([x, y]: Offset, pitch: number, height: number): number[] =>
  Math.abs(x) >= READING_BAND_PX ? [] : Array.from({ length: Math.max(0, Math.ceil((height - y) / pitch) + 1) }, (_, row) => row).filter((row) => y - height < -row * pitch && y > -(row + 1) * pitch);

export const homeRows = (keys: readonly string[], taken: ReadonlySet<number>): ReadonlyMap<string, number> => {
  const free = Array.from({ length: keys.length + taken.size }, (_, row) => row).filter((row) => !taken.has(row));
  return new Map(keys.toReversed().map((key, at) => [key, free[at] ?? at]));
};

export const MINIMAP_KEY = 'minimap';

export const HEADING_TAPE_KEY = 'headingTape';

export const MINIMAP_NAME = 'Mini map';

export const BATTERY_BAR_KEY = 'batteryBar';

export const batteryBarPillText = (hidden: boolean): string => `Battery bar: ${hidden ? 'Hidden' : 'Shown'}`;

const HIDDEN_BY_DEFAULT: readonly string[] = [MINIMAP_KEY];

export const hiddenKeys = (stored: Readonly<Record<string, unknown>>): Set<string> =>
  new Set([
    ...HIDDEN_BY_DEFAULT.filter((key) => stored[HIDDEN_PREFIX + key] !== false),
    ...Object.entries(stored)
      .filter(([key, value]) => key.startsWith(HIDDEN_PREFIX) && value === true)
      .map(([key]) => key.slice(HIDDEN_PREFIX.length)),
  ]);

export const ITEM_SIZES: readonly number[] = [1, 1.25, 1.5, 2];

export const nextItemSize = (size: number): number => ITEM_SIZES[(ITEM_SIZES.indexOf(size) + 1) % ITEM_SIZES.length] ?? 1;

export const resizeLabel = (size: number): string => (nextItemSize(size) > size ? 'Make bigger' : 'Back to normal size');

export const storedSizes = (stored: Readonly<Record<string, unknown>>): Map<string, number> =>
  new Map(
    Object.entries(stored).flatMap(([key, value]): [string, number][] => {
      const size = key.startsWith(SIZE_PREFIX) && typeof value === 'string' ? toDoubleOrNull(value) : null;
      return size === null || !ITEM_SIZES.includes(size) ? [] : [[key.slice(SIZE_PREFIX.length), size]];
    }),
  );

export const withHidden = (hidden: ReadonlySet<string>, key: string, hide: boolean): Set<string> =>
  hide ? new Set([...hidden, key]) : new Set([...hidden].filter((it) => it !== key));

export const storedOffsets = (stored: Readonly<Record<string, unknown>>): Map<string, Offset> =>
  new Map(
    Object.entries(stored).flatMap(([key, value]): [string, Offset][] => {
      if (!key.startsWith(OFFSET_PREFIX) || typeof value !== 'string') return [];
      const parts = value.split(',').flatMap((it) => toDoubleOrNull(it) ?? []);
      const [x, y] = parts;
      return parts.length === 2 && x !== undefined && y !== undefined ? [[key.slice(OFFSET_PREFIX.length), [x, y]]] : [];
    }),
  );

const shownArea = (reserved: Bounds): Bounds => visibleArea({ width: window.innerWidth, height: window.innerHeight }, reserved);

const useRootBounds = (seen: (bounds: Bounds) => void): readonly [(node: HTMLElement | null) => () => void, RefObject<HTMLElement | null>] => {
  const latest = useUpdatedRef(seen);
  const node = useRef<HTMLElement | null>(null);
  useLayoutEffect(() => {
    if (node.current) latest.current(node.current.getBoundingClientRect());
  });
  const ref = useCallback((element: HTMLElement | null) => {
    node.current = element;
    const measure = () => {
      if (element) latest.current(element.getBoundingClientRect());
    };
    const observer = typeof ResizeObserver === 'undefined' || !element ? null : new ResizeObserver(measure);
    if (element) observer?.observe(element);
    window.addEventListener('resize', measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener('resize', measure);
      node.current = null;
    };
  }, []);
  return [ref, node];
};

const useLayoutKey = (key: string): string => orientedKey(key, !useFlyIsPortrait());

const useBothRefs = (first: RefCallback<HTMLElement> | undefined, second: RefCallback<HTMLElement> | undefined): RefCallback<HTMLElement> =>
  useCallback(
    (node: HTMLElement | null) => {
      const released = [first?.(node), second?.(node)];
      return () => released.forEach((release) => release?.());
    },
    [first, second],
  );

const NO_OFFSET: Offset = [0, 0];

export const LocalLayoutBounds = createContext<RefObject<HTMLElement | null> | null>(null);

export const intersected = (a: Bounds, b: Bounds): Bounds => ({ left: Math.max(a.left, b.left), top: Math.max(a.top, b.top), right: Math.min(a.right, b.right), bottom: Math.min(a.bottom, b.bottom) });

const useLayoutArea = (): (() => Bounds) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const bounds = useContext(LocalLayoutBounds);
  return () => {
    const reserved = flyScreen.widgetInsets.value;
    const box = bounds?.current?.getBoundingClientRect();
    return box === undefined ? shownArea(reserved) : intersected(shownArea({ ...reserved, top: 0 }), box);
  };
};

const useKeptOnScreen = (key: string, home: Offset): readonly [Offset, (node: HTMLElement | null) => () => void, RefObject<HTMLElement | null>] => {
  const flyScreen = useContext(LocalFlyScreenState);
  const area = useLayoutArea();
  const [correction, setCorrection] = useState<Offset>(NO_OFFSET);
  const [kept, frame] = useRootBounds((bounds) => {
    const next = shownCorrection(bounds, area(), flyScreen.layout.offsets.value.get(key) ?? home, correction);
    setCorrection((was) => (was[0] === next[0] && was[1] === next[1] ? was : next));
  });
  return [correction, kept, frame];
};

export const swallowReleaseClick = (): void => {
  const release = new AbortController();
  const listening = { capture: true, signal: release.signal };
  window.addEventListener(
    'click',
    (event) => {
      event.stopPropagation();
      event.preventDefault();
      release.abort();
    },
    listening,
  );
  window.addEventListener('pointerdown', () => release.abort(), listening);
  window.addEventListener('keydown', () => release.abort(), listening);
};

const LAYOUT_ITEM = 'data-layout-item';

const within = (box: DOMRect, x: number, y: number): boolean => x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;

export const overStatusRow = (x: number, y: number): boolean => {
  const row = document.querySelector('[data-status-row]')?.getBoundingClientRect();
  return row !== undefined && within(row, x, y);
};

const overLayoutItem = (x: number, y: number): boolean => Array.from(document.querySelectorAll(`[${LAYOUT_ITEM}]`)).some((item) => within(item.getBoundingClientRect(), x, y));

const itemUnder = (target: Element, x: number, y: number): Element | null => {
  const under = Array.from(document.querySelectorAll(`[${LAYOUT_ITEM}]`)).filter((item) => within(item.getBoundingClientRect(), x, y));
  return target.closest(`[${LAYOUT_ITEM}]`) ?? under.findLast((item) => !under.some((other) => other !== item && item.contains(other))) ?? null;
};

const KEEPS_EDITING = `[${LAYOUT_ITEM}], button, a, input, select, textarea, [role="button"], [role="slider"], [role="switch"], [role="checkbox"], [role="menuitem"]`;

const OPEN_POPUP = '[role="dialog"], [role="menu"], [role="listbox"]';

const CLICK_SLOP_PX = 6;

export const endsLayoutEditing = (target: Element | null, x: number, y: number): boolean =>
  document.querySelector(OPEN_POPUP) === null && (target === null || target.closest(KEEPS_EDITING) === null) && !overLayoutItem(x, y);

const useHoldToEdit = (accepts: (event: PointerEvent<HTMLElement>) => boolean): HTMLAttributes<HTMLElement> => {
  const layout = useContext(LocalFlyScreenState).layout;
  const editing = useValue(layout.editing);
  const press = useRef<{ id: number; x: number; y: number; timer: ReturnType<typeof setTimeout> } | null>(null);
  const cancel = () => {
    if (press.current !== null) clearTimeout(press.current.timer);
    press.current = null;
  };
  useEffect(() => cancel, []);
  if (editing) return {};
  return {
    onPointerDownCapture: (event) => {
      cancel();
      if (!event.isPrimary || event.button !== 0 || !accepts(event)) return;
      const [target, pointerId, x, y] = [event.target as Element, event.pointerId, event.clientX, event.clientY];
      const timer = setTimeout(() => {
        press.current = null;
        const item = itemUnder(target, x, y);
        layout.startEditing(item === null ? null : { item, pointerId, x, y });
        if (layout.editing.value) swallowReleaseClick();
      }, LONG_PRESS_TIMEOUT_MS);
      press.current = { id: event.pointerId, x: event.clientX, y: event.clientY, timer };
    },
    onPointerMoveCapture: (event) => {
      const down = press.current;
      if (down !== null && down.id === event.pointerId && Math.hypot(event.clientX - down.x, event.clientY - down.y) > TOUCH_SLOP) cancel();
    },
    onPointerUpCapture: cancel,
    onPointerCancelCapture: cancel,
    onPointerLeave: cancel,
  };
};

export const useHoldToEditLayout = (): HTMLAttributes<HTMLElement> =>
  useHoldToEdit((event) => event.currentTarget.contains(event.target as Node) && (event.target as Element).closest('[data-gesture]') === null);

export const useHoldThroughToEditLayout = (): HTMLAttributes<HTMLElement> =>
  useHoldToEdit((event) => {
    const target = event.target as Element;
    const pictureHolds = event.pointerType !== 'mouse' && target.closest('[data-hold]') !== null;
    return !pictureHolds && target.closest(`[${LAYOUT_ITEM}]`) === null && overLayoutItem(event.clientX, event.clientY);
  });

const hashCode = (text: string): number => Array.from({ length: text.length }, (_, index) => text.charCodeAt(index)).reduce((hash, char) => (Math.imul(hash, 31) + char) | 0, 0);

const JIGGLE_KEYFRAMES = 'aircast-overlay-jiggle';

const reducedMotion = (): boolean => typeof window !== 'undefined' && !!window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

export const jiggleDegrees = (size: Size): number => {
  const reach = Math.hypot(size.width, size.height) / 2;
  return reach <= 0 ? JIGGLE_DEGREES : Math.min(JIGGLE_DEGREES, (JIGGLE_REACH_PX / reach) * (180 / Math.PI));
};

export const useJiggleAngle = (key: string, on: boolean, size: Size): CSSProperties =>
  on && !reducedMotion()
    ? ({ animation: `${JIGGLE_KEYFRAMES} ${JIGGLE_MILLIS + (hashCode(key) & JIGGLE_SPREAD)}ms ease-in-out infinite alternate`, [JIGGLE_ANGLE]: `${jiggleDegrees(size)}deg` } as CSSProperties)
    : {};

export const liftStyle = (lifted: boolean): CSSProperties => ({
  scale: lifted ? LIFT_SCALE : 1,
  filter: lifted ? LIFT_SHADOW : 'none',
  transition: reducedMotion() ? 'none' : `scale ${LIFT_MILLIS}ms ${LIFT_EASING}, filter ${LIFT_MILLIS}ms ease-out`,
});

export const LayoutLifted = createContext(false);

export const clampedInto = (bounds: Bounds, area: Bounds, dx: number, dy: number): Offset => [
  coerceIn(dx, area.left - bounds.left, coerceAtLeast(area.right - bounds.right, area.left - bounds.left)),
  coerceIn(dy, area.top - bounds.top, coerceAtLeast(area.bottom - bounds.bottom, area.top - bounds.top)),
];

export const liftedBounds = (bounds: Bounds): Bounds => {
  const growX = Math.max(((LIFT_SCALE - 1) / 2) * (bounds.right - bounds.left), JIGGLE_REACH_PX);
  const growY = Math.max(((LIFT_SCALE - 1) / 2) * (bounds.bottom - bounds.top), JIGGLE_REACH_PX);
  return { left: bounds.left - growX, top: bounds.top - growY, right: bounds.right + growX, bottom: bounds.bottom + growY };
};

const ARROW_STEPS: Readonly<Record<string, Offset>> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };

export const ARROW_STEP_PX = 8;

export const arrowStep = (key: string, far: boolean): Offset | null => {
  const step = ARROW_STEPS[key];
  const scale = far ? ARROW_STEP_PX * 4 : ARROW_STEP_PX;
  return step === undefined ? null : [step[0] * scale, step[1] * scale];
};

const captured = (node: HTMLElement, pointerId: number): boolean => {
  try {
    node.setPointerCapture(pointerId);
    return true;
  } catch {
    return false;
  }
};

export const LayoutDragArea = ({
  widgetKey,
  home = NO_OFFSET,
  item,
  correction,
  frame,
  onLift,
  onBar,
  className,
  style,
  children,
}: {
  widgetKey: string;
  home?: Offset;
  item: RefObject<HTMLElement | null>;
  correction: Offset;
  frame: RefObject<HTMLElement | null>;
  onLift: (lifted: boolean) => void;
  onBar?: (x: number) => void;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
}) => {
  const layout = useContext(LocalFlyScreenState).layout;
  const area = useLayoutArea();
  const reach = (): Bounds => (onBar === undefined ? area() : { ...area(), top: 0 });
  const placedKey = useLayoutKey(widgetKey);
  const drag = useRef<{ id: number; x: number; y: number; dragging: boolean } | null>(null);
  const self = useRef<HTMLDivElement | null>(null);
  useLayoutEffect(() => {
    const carry = layout.carry;
    const node = self.current;
    if (carry === null || node === null || item.current !== carry.item) return;
    layout.carry = null;
    if (!captured(node, carry.pointerId)) return;
    drag.current = { id: carry.pointerId, x: carry.x, y: carry.y, dragging: false };
    onLift(true);
  }, []);
  const end = (event: PointerEvent<HTMLDivElement>) => {
    const held = drag.current;
    if (held === null || held.id !== event.pointerId) return;
    drag.current = null;
    onLift(false);
    if (held.dragging && onBar !== undefined && event.type === 'pointerup' && overStatusRow(event.clientX, event.clientY)) onBar(event.clientX);
    else if (held.dragging) layout.saveOffset(placedKey);
  };
  return (
    <div
      ref={self}
      data-gesture
      role="button"
      tabIndex={0}
      aria-label="Move"
      aria-keyshortcuts="ArrowLeft ArrowRight ArrowUp ArrowDown"
      className={`touch-none select-none cursor-grab outline-none focus-visible:ring-2 focus-visible:ring-primary active:cursor-grabbing ${className ?? ''}`}
      style={style}
      onKeyDown={(event) => {
        const step = arrowStep(event.key, event.shiftKey);
        const placed = frame.current;
        if (step === null || placed === null) return;
        event.preventDefault();
        const [dx, dy] = clampedInto(placed.getBoundingClientRect(), area(), step[0], step[1]);
        layout.nudge(placedKey, dx + correction[0], dy + correction[1], home);
        layout.saveOffset(placedKey);
      }}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { id: event.pointerId, x: event.clientX, y: event.clientY, dragging: false };
        onLift(true);
      }}
      onPointerMove={(event) => {
        const held = drag.current;
        const placed = frame.current;
        if (held === null || held.id !== event.pointerId || placed === null) return;
        const rawX = event.clientX - held.x;
        const rawY = event.clientY - held.y;
        const distance = Math.hypot(rawX, rawY);
        if (!held.dragging && distance <= TOUCH_SLOP) return;
        const past = held.dragging ? 1 : (distance - TOUCH_SLOP) / distance;
        const [dx, dy] = clampedInto(liftedBounds(placed.getBoundingClientRect()), reach(), rawX * past, rawY * past);
        const [foldX, foldY] = held.dragging ? NO_OFFSET : correction;
        drag.current = { id: held.id, x: event.clientX, y: event.clientY, dragging: true };
        layout.nudge(placedKey, dx + foldX, dy + foldY, home);
      }}
      onPointerUp={end}
      onPointerCancel={end}
    >
      {children}
    </div>
  );
};

const BADGE = "absolute grid place-items-center after:absolute after:-inset-[13px] after:content-['']";

const NAMED_BADGE = "absolute flex items-center gap-0.5 whitespace-nowrap pe-1.5 text-label-small after:absolute after:-inset-[13px] after:content-['']";

const BADGE_RISE = (BADGE_SIZE * 3) / 4;

const BADGE_PLACE: CSSProperties = { width: BADGE_SIZE, height: BADGE_SIZE, top: -BADGE_RISE, insetInlineStart: -BADGE_SIZE / 2 };

const BESIDE_BADGE = "absolute grid place-items-center after:absolute after:-inset-x-[13px] after:-inset-y-[3px] after:content-['']";

const BADGE_GAP = 4;

const OUTER_BADGE = {
  start: "absolute grid place-items-center after:absolute after:-inset-y-[3px] after:end-0 after:-start-[13px] after:content-['']",
  end: "absolute grid place-items-center after:absolute after:-inset-y-[3px] after:start-0 after:-end-[13px] after:content-['']",
} as const;

const besidePlace = (side: 'start' | 'end', nth = 0): CSSProperties => ({
  width: BADGE_SIZE,
  height: BADGE_SIZE,
  top: '50%',
  marginTop: -BADGE_SIZE / 2,
  [side === 'start' ? 'insetInlineStart' : 'insetInlineEnd']: -(BADGE_SIZE + BADGE_GAP) * (nth + 1),
});

export const badgeRoom = (editing: boolean, leading: boolean): CSSProperties =>
  editing ? { paddingTop: BADGE_RISE, marginTop: -BADGE_RISE, ...(leading ? { paddingInlineStart: BADGE_SIZE / 2, marginInlineStart: -BADGE_SIZE / 2 } : {}) } : {};

const LocalLayoutParent = createContext<Dispatch<SetStateAction<number>> | null>(null);

type GroupFrame = { readonly correction: Offset; readonly frame: RefObject<HTMLElement | null>; readonly lift: (lifted: boolean) => void };

const LocalLayoutGroup = createContext<GroupFrame | null>(null);

export const LayoutGroup = ({ widgetKey, className, children }: { widgetKey: string; className?: string; children?: ReactNode }) => {
  const offsets = useValue(useContext(LocalFlyScreenState).layout.offsets);
  const avoided = useContext(LocalAvoidedByVideoMessage);
  const avoidedRef = useAvoidedByVideoMessage(widgetKey);
  const placedKey = useLayoutKey(widgetKey);
  const [correction, kept, frame] = useKeptOnScreen(placedKey, NO_OFFSET);
  const hold = useHoldToEditLayout();
  const [lifted, setLifted] = useState(false);
  const placed = useBothRefs(kept, avoided ? avoidedRef : undefined);
  const [x, y] = offsets.get(placedKey) ?? NO_OFFSET;
  return (
    <div ref={placed} className={className} style={{ translate: `${x + correction[0]}px ${y + correction[1]}px`, zIndex: lifted ? 1 : undefined }} {...hold}>
      <div style={liftStyle(lifted)}>
        <LocalLayoutGroup.Provider value={{ correction, frame, lift: setLifted }}>
          <LayoutLifted.Provider value={lifted}>{children}</LayoutLifted.Provider>
        </LocalLayoutGroup.Provider>
      </div>
    </div>
  );
};

export const badgeLabel = (hidden: boolean, name: string | undefined, removes = false): string => [hidden ? 'Show' : removes ? 'Remove' : 'Hide', ...(name === undefined ? [] : [name.toLowerCase()])].join(' ');

export const LayoutWidget = ({
  widgetKey,
  home = NO_OFFSET,
  badgeBeside,
  movable = true,
  hideable = true,
  live = false,
  dragsWith,
  onBar,
  onRemove,
  resizable = false,
  name,
  className,
  children,
}: {
  widgetKey: string;
  home?: Offset;
  badgeBeside?: 'start' | 'end';
  name?: string;
  movable?: boolean;
  hideable?: boolean;
  live?: boolean;
  dragsWith?: string;
  onBar?: (x: number) => void;
  onRemove?: () => void;
  resizable?: boolean;
  className?: string;
  children?: ReactNode;
}) => {
  const layout = useContext(LocalFlyScreenState).layout;
  const editing = useValue(layout.editing);
  const scale = useValue(layout.sizes).get(widgetKey) ?? 1;
  const hidden = useValue(layout.hidden).has(widgetKey);
  const offsets = useValue(layout.offsets);
  const avoided = useContext(LocalAvoidedByVideoMessage);
  const avoidedRef = useAvoidedByVideoMessage(widgetKey);
  const osd = useContext(LocalFlyOsd);
  const placedKey = useLayoutKey(widgetKey);
  const [correction, kept, frame] = useKeptOnScreen(placedKey, home);
  const area = useLayoutArea();
  useLayoutEffect(() => {
    const landing = layout.landing;
    const node = frame.current;
    if (landing === null || landing.key !== widgetKey || node === null) return;
    layout.landing = null;
    const box = node.getBoundingClientRect();
    const [dx, dy] = clampedInto(box, area(), landing.x - (box.left + box.right) / 2, landing.y - (box.top + box.bottom) / 2);
    layout.nudge(placedKey, dx, dy, home);
    layout.saveOffset(placedKey);
  }, []);
  const hold = useHoldToEditLayout();
  const group = useContext(LocalLayoutGroup);
  const parent = useContext(LocalLayoutParent);
  const [nested, setNested] = useState(0);
  const [lifted, setLifted] = useState(false);
  const carriedAloft = useContext(LayoutLifted);
  useLayoutEffect(() => {
    parent?.((count) => count + 1);
    return () => parent?.((count) => count - 1);
  }, [parent]);
  const [size, setSize] = useState<Size>({ width: 0, height: 0 });
  const item = useRef<HTMLElement | null>(null);
  const sized = useCallback((node: HTMLElement | null) => {
    item.current = node;
    if (!node || typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(() => setSize((was) => (was.width === node.offsetWidth && was.height === node.offsetHeight ? was : { width: node.offsetWidth, height: node.offsetHeight })));
    observer.observe(node);
    return () => {
      observer.disconnect();
      item.current = null;
    };
  }, []);
  const decorated = editing && size.width > 0 && size.height > 0 && nested === 0;
  const jiggle = useJiggleAngle(widgetKey, decorated && !lifted && !carriedAloft, size);
  const placed = useBothRefs(movable ? kept : undefined, avoided ? avoidedRef : undefined);
  if (hidden && !editing) return null;
  const [x, y] = movable ? (offsets.get(placedKey) ?? home) : NO_OFFSET;
  const [cx, cy] = movable ? correction : NO_OFFSET;
  const carried = dragsWith !== undefined && group !== null ? { key: dragsWith, ...group } : null;
  return (
    <div ref={placed} className={className} style={{ translate: `${x + cx}px ${y + cy}px`, zIndex: lifted ? 1 : undefined }} {...hold}>
      <div className="relative" style={{ ...jiggle, ...liftStyle(lifted) }}>
        <div
          ref={sized}
          data-layout-item
          className={osd ? osdShadow() : undefined}
          style={{ opacity: hidden ? HIDDEN_ALPHA : 1, zoom: resizable ? scale : undefined }}
        >
          <LocalLayoutParent.Provider value={setNested}>{children}</LocalLayoutParent.Provider>
        </div>
        {decorated && (movable || carried !== null) ? (
          <LayoutDragArea
            widgetKey={carried?.key ?? widgetKey}
            home={carried === null ? home : NO_OFFSET}
            item={item}
            correction={carried?.correction ?? correction}
            frame={carried?.frame ?? frame}
            onLift={carried?.lift ?? setLifted}
            onBar={carried === null ? onBar : undefined}
            className={live ? BADGE : 'absolute inset-0'}
            style={live ? BADGE_PLACE : undefined}
          >
            {live ? (
              <span className="grid size-full place-items-center rounded-full shadow" style={{ background: md('inverseSurface'), color: md('inverseOnSurface') }}>
                <MaterialIcon name="open_with" size={BADGE_SIZE - 6} />
              </span>
            ) : null}
          </LayoutDragArea>
        ) : decorated && !live ? (
          <div data-gesture className="absolute inset-0" />
        ) : null}
        {decorated && hideable ? (
          <Surface
            onClick={() => (onRemove !== undefined && !hidden ? onRemove() : layout.setHidden(widgetKey, !hidden))}
            className={name !== undefined ? NAMED_BADGE : badgeBeside !== undefined ? BESIDE_BADGE : BADGE}
            style={name !== undefined ? { ...BADGE_PLACE, width: 'auto' } : badgeBeside !== undefined ? besidePlace(badgeBeside) : BADGE_PLACE}
            shape="full"
            color={hidden ? md('primary') : md('inverseSurface')}
            contentColor={hidden ? md('onPrimary') : md('inverseOnSurface')}
            shadow
            label={badgeLabel(hidden, name, onRemove !== undefined)}
          >
            <MaterialIcon name={hidden ? 'add' : 'close'} filled size={BADGE_SIZE - 6} className="mx-[3px]" />
            {name === undefined ? null : <span aria-hidden>{name}</span>}
          </Surface>
        ) : null}
        {decorated && resizable && !hidden ? (
          <Surface
            onClick={() => layout.resize(widgetKey)}
            className={badgeBeside !== undefined ? OUTER_BADGE[badgeBeside] : BADGE}
            style={badgeBeside !== undefined ? besidePlace(badgeBeside, 1) : { ...BADGE_PLACE, insetInlineStart: undefined, insetInlineEnd: -BADGE_SIZE / 2 }}
            shape="full"
            color={md('inverseSurface')}
            contentColor={md('inverseOnSurface')}
            shadow
            label={resizeLabel(scale)}
          >
            <MaterialIcon name={nextItemSize(scale) > scale ? 'open_in_full' : 'close_fullscreen'} size={BADGE_SIZE - 6} className="mx-[3px]" />
          </Surface>
        ) : null}
      </div>
    </div>
  );
};

export const LayoutOptionsMenu = () => {
  const flyScreen = useContext(LocalFlyScreenState);
  const layout = flyScreen.layout;
  const valueSize = useValue(layout.valueSize);
  const barHidden = useValue(layout.hidden).has(BATTERY_BAR_KEY);
  const vehicleClass = instrumentVehicleClass(useQgcPath(INSTRUMENTS_VIEW));
  const backdrop = useOsdBackdrop(OSD_SCRIM);
  const [open, setOpen] = useState(false);
  const [armed, setArmed] = useState(false);
  useLaunchedEffect([armed], async (scope) => {
    if (!armed) return;
    await delay(RESET_ARM_MILLIS);
    if (scope.isActive()) setArmed(false);
  });
  return (
    <div className="relative">
      <Surface
        onClick={() => setOpen(true)}
        shape="full"
        color={backdrop}
        contentColor={aircast('outdoorForeground')}
        label="Layout options"
        className="grid place-items-center"
        style={{ width: STATUS_ROW_HEIGHT + AircastSpace.s2, height: STATUS_ROW_HEIGHT + AircastSpace.s2 }}
      >
        <Drawable name="ic_tune" size={22} />
      </Surface>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        <DropdownMenuItem
          text={valueSizePillText(valueSize)}
          closeOnClick={false}
          onClick={() => {
            const next = nextValueSize(valueSize);
            layout.valueSize.value = next;
            writeValueSize(vehicleClass, next);
          }}
        />
        <DropdownMenuItem text={batteryBarPillText(barHidden)} closeOnClick={false} onClick={() => layout.setHidden(BATTERY_BAR_KEY, !barHidden)} />
        <DropdownMenuItem
          text={resetPillText(armed)}
          color={armed ? md('error') : undefined}
          closeOnClick={false}
          onClick={() => {
            const tap = resetTap(armed);
            if (tap.reset) layout.reset();
            setArmed(tap.armed);
            if (tap.reset) setOpen(false);
          }}
        />
      </DropdownMenu>
    </div>
  );
};

export const ADD_READING = 'Add reading';

export const LayoutAddReadingButton = () => {
  const flyScreen = useContext(LocalFlyScreenState);
  const backdrop = useOsdBackdrop(OSD_SCRIM);
  return (
    <Surface
      onClick={() => {
        flyScreen.choosingReadings.value = true;
      }}
      shape="full"
      color={backdrop}
      contentColor={aircast('outdoorForeground')}
      label={ADD_READING}
      className="flex shrink-0 items-center gap-1 ps-2 pe-3"
      style={{ height: STATUS_ROW_HEIGHT + AircastSpace.s2 }}
    >
      <Drawable name="ic_add" size={22} />
      <Text typography="labelLarge" color={aircast('outdoorForeground')}>
        {ADD_READING}
      </Text>
    </Surface>
  );
};

export const LayoutEditingNotice = () => {
  const armed = useArmed();
  const editing = useValue(useContext(LocalFlyScreenState).layout.editing);
  return editing && armed ? (
    <div role="status" className="self-center rounded-shape-sm px-3 py-1.5 text-center" style={{ background: OSD_SCRIM, maxWidth: LAYOUT_NOTICE_MAX_WIDTH }}>
      <Text typography="labelMedium" color={aircast('outdoorForeground')}>
        {LAYOUT_EDITING_IN_FLIGHT}
      </Text>
    </div>
  ) : null;
};

export const LayoutDoneButton = () => {
  const layout = useContext(LocalFlyScreenState).layout;
  useEscapeHandler(true, () => {
    layout.editing.value = false;
  });
  useEffect(() => {
    const pressed = { x: 0, y: 0, outside: false };
    const ends = (event: MouseEvent) => endsLayoutEditing(event.target instanceof Element ? event.target : null, event.clientX, event.clientY);
    const press = (event: MouseEvent) => {
      pressed.x = event.clientX;
      pressed.y = event.clientY;
      pressed.outside = ends(event);
    };
    const click = (event: MouseEvent) => {
      if (pressed.outside && Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y) <= CLICK_SLOP_PX && ends(event)) layout.editing.value = false;
    };
    document.addEventListener('pointerdown', press, true);
    document.addEventListener('click', click, true);
    return () => {
      document.removeEventListener('pointerdown', press, true);
      document.removeEventListener('click', click, true);
    };
  }, [layout]);
  return (
    <Button
      onClick={() => {
        layout.editing.value = false;
      }}
    >
      Done
    </Button>
  );
};
