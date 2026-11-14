import { type CSSProperties, type Dispatch, type HTMLAttributes, type PointerEvent, type ReactNode, type RefObject, type SetStateAction, createContext, useCallback, useContext, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Button, DropdownMenu, DropdownMenuItem, Surface } from '@/m3';
import { useBackHandler, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { type SharedPreferences, getSharedPreferences, TAP } from '@/kotlin/Platform';
import { type MutableState, mutableStateOf, useValue } from '@/kotlin/State';
import { coerceAtLeast, coerceIn, delay } from '@/kotlin/Std';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { AircastSpace, Color, aircast, md, withAlpha } from '@/theme/Colors';
import { FLY_SCRIM_ALPHA, LocalAvoidedByVideoMessage, STATUS_ROW_HEIGHT, useAvoidedByVideoMessage, useFlyIsPortrait } from '@/ui/FlyShell';
import { LocalFlyScreenState } from '@/ui/FlyScreenState';
import { INSTRUMENTS_VIEW, ValueSize, instrumentVehicleClass, nextValueSize, valueSizePillText, writeValueSize } from '@/ui/InstrumentChoice';
import { LocalFlyOsd, osdShadow, useOsdBackdrop } from '@/ui/Osd';

const LAYOUT_STORE = 'fly-overlay-layout';
const HIDDEN_PREFIX = 'OverlayRigHidden-';
const OFFSET_PREFIX = 'OverlayRigOffset-';
const LANDSCAPE_SUFFIX = '@landscape';
const HIDDEN_ALPHA = 0.35;
const JIGGLE_DEGREES = 1.2;
const JIGGLE_MILLIS = 120;
const JIGGLE_SPREAD = 0x1f;
const JIGGLE_REACH_PX = 4;
const JIGGLE_ANGLE = '--aircast-jiggle';
export const BADGE_SIZE = 18;
const LONG_PRESS_TIMEOUT_MS = 400;
const TOUCH_SLOP = 8;
export const RESET_ARM_MILLIS = 4000;

export type ResetTap = { reset: boolean; armed: boolean };

export const ResetTap = (fields: ResetTap): ResetTap => ({ ...fields });

export const resetTap = (armed: boolean): ResetTap => ResetTap({ reset: armed, armed: !armed });

export const resetPillText = (armed: boolean): string => (armed ? `${TAP} again to reset` : 'Reset layout');

const INDICATOR_ORDER = 'FlyViewIndicatorOrder';

export type Offset = readonly [number, number];

export type Bounds = { readonly left: number; readonly top: number; readonly right: number; readonly bottom: number };

export type Size = { readonly width: number; readonly height: number };

const storedLayout = (): Record<string, unknown> =>
  Object.fromEntries(Object.entries(getSharedPreferences(LAYOUT_STORE).getAll()).map(([key, text]) => [key, text === 'true' ? true : text === 'false' ? false : text]));

export class OverlayLayoutState {
  readonly editing = mutableStateOf(false);
  readonly locked = mutableStateOf(false);
  readonly hidden: MutableState<ReadonlySet<string>>;
  readonly indicatorOrder: MutableState<readonly string[]>;
  readonly valueSize = mutableStateOf<ValueSize>(ValueSize.Default);
  readonly offsets: MutableState<ReadonlyMap<string, Offset>>;

  constructor(private readonly prefs: SharedPreferences) {
    const stored = storedLayout();
    this.hidden = mutableStateOf(hiddenKeys(stored));
    this.indicatorOrder = mutableStateOf(storedIndicatorOrder(stored));
    this.offsets = mutableStateOf(storedOffsets(stored));
  }

  startEditing(): void {
    if (!this.locked.value) this.editing.value = true;
  }

  lockWhileArmed(armed: boolean): void {
    this.locked.value = armed;
    if (armed) this.editing.value = false;
  }

  setHidden(key: string, hide: boolean): void {
    this.prefs.putBoolean(HIDDEN_PREFIX + key, hide);
    this.hidden.value = withHidden(this.hidden.value, key, hide);
  }

  saveIndicatorOrder(keys: readonly string[]): void {
    this.prefs.putString(INDICATOR_ORDER, keys.join(','));
    this.indicatorOrder.value = keys;
  }

  nudge(key: string, dx: number, dy: number): void {
    const [x, y] = this.offsets.value.get(key) ?? [0, 0];
    this.offsets.value = new Map(this.offsets.value).set(key, [x + dx, y + dy]);
  }

  saveOffset(key: string): void {
    const offset = this.offsets.value.get(key);
    if (offset) this.prefs.putString(OFFSET_PREFIX + key, `${offset[0]},${offset[1]}`);
  }

  reset(): void {
    Object.keys(storedLayout()).forEach((name) => this.prefs.remove(name));
    this.hidden.value = new Set();
    this.indicatorOrder.value = [];
    this.offsets.value = new Map();
  }
}

export const overlayLayoutStore = (): SharedPreferences => getSharedPreferences(LAYOUT_STORE);

export const storedIndicatorOrder = (stored: Readonly<Record<string, unknown>>): string[] => {
  const order = stored[INDICATOR_ORDER];
  return (typeof order === 'string' ? order : '').split(',').filter((it) => it.trim() !== '');
};

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

export const droppedOrder = (keys: readonly string[], centres: ReadonlyMap<string, number>, key: string, dx: number): string[] => {
  const at = (shown: string) => (centres.get(shown) ?? 0) + (shown === key ? dx : 0);
  return reorderedAmongShown(
    keys,
    keys.filter((it) => centres.has(it)).toSorted((a, b) => at(a) - at(b)),
  );
};

export const hiddenKeys = (stored: Readonly<Record<string, unknown>>): Set<string> =>
  new Set(
    Object.entries(stored)
      .filter(([key, value]) => key.startsWith(HIDDEN_PREFIX) && value === true)
      .map(([key]) => key.slice(HIDDEN_PREFIX.length)),
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

const NO_OFFSET: Offset = [0, 0];

const useKeptOnScreen = (key: string): readonly [Offset, (node: HTMLElement | null) => () => void, RefObject<HTMLElement | null>] => {
  const flyScreen = useContext(LocalFlyScreenState);
  const [correction, setCorrection] = useState<Offset>(NO_OFFSET);
  const [kept, frame] = useRootBounds((bounds) => {
    const next = shownCorrection(bounds, shownArea(flyScreen.widgetInsets.value), flyScreen.layout.offsets.value.get(key) ?? NO_OFFSET, correction);
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

export const useHoldToEditLayout = (): HTMLAttributes<HTMLElement> => {
  const layout = useContext(LocalFlyScreenState).layout;
  const editing = useValue(layout.editing);
  const locked = useValue(layout.locked);
  const press = useRef<{ id: number; x: number; y: number; timer: ReturnType<typeof setTimeout> } | null>(null);
  const cancel = () => {
    if (press.current !== null) clearTimeout(press.current.timer);
    press.current = null;
  };
  useEffect(() => cancel, []);
  if (editing || locked) return {};
  return {
    onPointerDownCapture: (event) => {
      cancel();
      if (!event.isPrimary || event.button !== 0 || !event.currentTarget.contains(event.target as Node)) return;
      const timer = setTimeout(() => {
        press.current = null;
        layout.startEditing();
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

const hashCode = (text: string): number => Array.from({ length: text.length }, (_, index) => text.charCodeAt(index)).reduce((hash, char) => (Math.imul(hash, 31) + char) | 0, 0);

const JIGGLE_STYLE_ID = 'aircast-overlay-jiggle';

const reducedMotion = (): boolean => typeof window !== 'undefined' && !!window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

export const jiggleDegrees = (size: Size): number => {
  const reach = Math.hypot(size.width, size.height) / 2;
  return reach <= 0 ? JIGGLE_DEGREES : Math.min(JIGGLE_DEGREES, (JIGGLE_REACH_PX / reach) * (180 / Math.PI));
};

export const useJiggleAngle = (key: string, on: boolean, size: Size): CSSProperties => {
  useEffect(() => {
    if (!on || document.getElementById(JIGGLE_STYLE_ID)) return;
    const style = Object.assign(document.createElement('style'), {
      id: JIGGLE_STYLE_ID,
      textContent: `@keyframes ${JIGGLE_STYLE_ID} { from { rotate: calc(-1 * var(${JIGGLE_ANGLE})) } to { rotate: var(${JIGGLE_ANGLE}) } }`,
    });
    document.head.append(style);
  }, [on]);
  return on && !reducedMotion()
    ? ({ animation: `${JIGGLE_STYLE_ID} ${JIGGLE_MILLIS + (hashCode(key) & JIGGLE_SPREAD)}ms ease-in-out infinite alternate`, [JIGGLE_ANGLE]: `${jiggleDegrees(size)}deg` } as CSSProperties)
    : {};
};

export const clampedInto = (bounds: Bounds, area: Bounds, dx: number, dy: number): Offset => [
  coerceIn(dx, area.left - bounds.left, coerceAtLeast(area.right - bounds.right, area.left - bounds.left)),
  coerceIn(dy, area.top - bounds.top, coerceAtLeast(area.bottom - bounds.bottom, area.top - bounds.top)),
];

export const LayoutDragArea = ({ widgetKey, correction, frame, className }: { widgetKey: string; correction: Offset; frame: RefObject<HTMLElement | null>; className?: string }) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const layout = flyScreen.layout;
  const placedKey = useLayoutKey(widgetKey);
  const drag = useRef<{ id: number; x: number; y: number; dragging: boolean } | null>(null);
  const end = (event: PointerEvent<HTMLDivElement>) => {
    const held = drag.current;
    if (held === null || held.id !== event.pointerId) return;
    drag.current = null;
    if (held.dragging) layout.saveOffset(placedKey);
  };
  return (
    <div
      data-gesture
      className={`touch-none ${className ?? ''}`}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { id: event.pointerId, x: event.clientX, y: event.clientY, dragging: false };
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
        const [dx, dy] = clampedInto(placed.getBoundingClientRect(), shownArea(flyScreen.widgetInsets.value), rawX * past, rawY * past);
        const [foldX, foldY] = held.dragging ? NO_OFFSET : correction;
        drag.current = { id: held.id, x: event.clientX, y: event.clientY, dragging: true };
        layout.nudge(placedKey, dx + foldX, dy + foldY);
      }}
      onPointerUp={end}
      onPointerCancel={end}
    />
  );
};

const BADGE = "absolute grid place-items-center after:absolute after:-inset-[3px] after:content-['']";

const BADGE_RISE = (BADGE_SIZE * 3) / 4;

const BADGE_PLACE: CSSProperties = { width: BADGE_SIZE, height: BADGE_SIZE, top: -BADGE_RISE, insetInlineStart: -BADGE_SIZE / 2 };

export const badgeRoom = (editing: boolean, leading: boolean): CSSProperties =>
  editing ? { paddingTop: BADGE_RISE, marginTop: -BADGE_RISE, ...(leading ? { paddingInlineStart: BADGE_SIZE / 2, marginInlineStart: -BADGE_SIZE / 2 } : {}) } : {};

const LocalLayoutParent = createContext<Dispatch<SetStateAction<number>> | null>(null);

type GroupFrame = { readonly correction: Offset; readonly frame: RefObject<HTMLElement | null> };

const LocalLayoutGroup = createContext<GroupFrame | null>(null);

export const LayoutGroup = ({ widgetKey, className, children }: { widgetKey: string; className?: string; children?: ReactNode }) => {
  const offsets = useValue(useContext(LocalFlyScreenState).layout.offsets);
  const avoided = useContext(LocalAvoidedByVideoMessage);
  const avoidedRef = useAvoidedByVideoMessage(widgetKey);
  const placedKey = useLayoutKey(widgetKey);
  const [correction, kept, frame] = useKeptOnScreen(placedKey);
  const hold = useHoldToEditLayout();
  const [x, y] = offsets.get(placedKey) ?? NO_OFFSET;
  return (
    <div ref={kept} className={className} style={{ translate: `${x + correction[0]}px ${y + correction[1]}px` }} {...hold}>
      <div ref={avoided ? avoidedRef : undefined}>
        <LocalLayoutGroup.Provider value={{ correction, frame }}>{children}</LocalLayoutGroup.Provider>
      </div>
    </div>
  );
};

export const LayoutWidget = ({
  widgetKey,
  movable = true,
  hideable = true,
  dragsWith,
  className,
  children,
}: {
  widgetKey: string;
  movable?: boolean;
  hideable?: boolean;
  dragsWith?: string;
  className?: string;
  children?: ReactNode;
}) => {
  const layout = useContext(LocalFlyScreenState).layout;
  const editing = useValue(layout.editing);
  const hidden = useValue(layout.hidden).has(widgetKey);
  const offsets = useValue(layout.offsets);
  const avoided = useContext(LocalAvoidedByVideoMessage);
  const avoidedRef = useAvoidedByVideoMessage(widgetKey);
  const osd = useContext(LocalFlyOsd);
  const placedKey = useLayoutKey(widgetKey);
  const [correction, kept, frame] = useKeptOnScreen(placedKey);
  const hold = useHoldToEditLayout();
  const group = useContext(LocalLayoutGroup);
  const parent = useContext(LocalLayoutParent);
  const [nested, setNested] = useState(0);
  useLayoutEffect(() => {
    parent?.((count) => count + 1);
    return () => parent?.((count) => count - 1);
  }, [parent]);
  const [size, setSize] = useState<Size>({ width: 0, height: 0 });
  const sized = useCallback((node: HTMLElement | null) => {
    if (!node || typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(() => setSize((was) => (was.width === node.offsetWidth && was.height === node.offsetHeight ? was : { width: node.offsetWidth, height: node.offsetHeight })));
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  const decorated = editing && size.width > 0 && size.height > 0 && nested === 0;
  const jiggle = useJiggleAngle(widgetKey, decorated, size);
  if (hidden && !editing) return null;
  const [x, y] = movable ? (offsets.get(placedKey) ?? NO_OFFSET) : NO_OFFSET;
  const [cx, cy] = movable ? correction : NO_OFFSET;
  const carried = dragsWith !== undefined && group !== null ? { key: dragsWith, ...group } : null;
  return (
    <div ref={movable ? kept : undefined} className={className} style={{ translate: `${x + cx}px ${y + cy}px` }} {...hold}>
      <div ref={avoided ? avoidedRef : undefined} className="relative" style={jiggle}>
        <div
          ref={sized}
          className={osd ? osdShadow() : undefined}
          style={{ opacity: hidden ? HIDDEN_ALPHA : 1 }}
        >
          <LocalLayoutParent.Provider value={setNested}>{children}</LocalLayoutParent.Provider>
        </div>
        {decorated && (movable || carried !== null) ? (
          <LayoutDragArea widgetKey={carried?.key ?? widgetKey} correction={carried?.correction ?? correction} frame={carried?.frame ?? frame} className="absolute inset-0" />
        ) : decorated ? (
          <div data-gesture className="absolute inset-0" />
        ) : null}
        {decorated && hideable ? (
          <Surface
            onClick={() => layout.setHidden(widgetKey, !hidden)}
            className={BADGE}
            style={BADGE_PLACE}
            shape="full"
            color={hidden ? md('primary') : md('inverseSurface')}
            contentColor={hidden ? md('onPrimary') : md('inverseOnSurface')}
            shadow
            label={hidden ? 'Show' : 'Hide'}
          >
            <MaterialIcon name={hidden ? 'add' : 'close'} filled size={BADGE_SIZE - 6} />
          </Surface>
        ) : null}
      </div>
    </div>
  );
};

export const LayoutOptionsMenu = () => {
  const layout = useContext(LocalFlyScreenState).layout;
  const valueSize = useValue(layout.valueSize);
  const vehicleClass = instrumentVehicleClass(useQgcPath(INSTRUMENTS_VIEW));
  const backdrop = useOsdBackdrop(withAlpha(Color.Black, FLY_SCRIM_ALPHA));
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

export const LayoutDoneButton = () => {
  const layout = useContext(LocalFlyScreenState).layout;
  useBackHandler(true, () => {
    layout.editing.value = false;
  });
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
