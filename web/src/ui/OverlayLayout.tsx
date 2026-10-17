import { type CSSProperties, type HTMLAttributes, type PointerEvent, type ReactNode, useCallback, useContext, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Button, Surface, TextButton, radius } from '@/m3';
import { useBackHandler, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { type SharedPreferences, getSharedPreferences } from '@/kotlin/Platform';
import { type MutableState, mutableStateOf, useValue } from '@/kotlin/State';
import { coerceAtLeast, coerceIn, delay } from '@/kotlin/Std';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { LocalAvoidedByVideoMessage, useAvoidedByVideoMessage, useFlyIsPortrait } from '@/ui/FlyShell';
import { LocalFlyScreenState } from '@/ui/FlyScreenState';
import { INSTRUMENTS_VIEW, ValueSize, instrumentVehicleClass, nextValueSize, valueSizePillText, writeValueSize } from '@/ui/InstrumentChoice';
import { LocalFlyOsd, osdShadow } from '@/ui/Osd';

const LAYOUT_STORE = 'fly-overlay-layout';
const HIDDEN_PREFIX = 'OverlayRigHidden-';
const OFFSET_PREFIX = 'OverlayRigOffset-';
const LANDSCAPE_SUFFIX = '@landscape';
const HIDDEN_ALPHA = 0.35;
const JIGGLE_DEGREES = 1.2;
const JIGGLE_MILLIS = 120;
const JIGGLE_SPREAD = 0x1f;
const BADGE_SIZE = 24;
const LONG_PRESS_TIMEOUT_MS = 400;
const TOUCH_SLOP = 8;
export const RESET_ARM_MILLIS = 4000;

export type ResetTap = { reset: boolean; armed: boolean };

export const ResetTap = (fields: ResetTap): ResetTap => ({ ...fields });

export const resetTap = (armed: boolean): ResetTap => ResetTap({ reset: armed, armed: !armed });

export const resetPillText = (armed: boolean): string => (armed ? 'Tap again to reset' : 'Reset layout');

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

export const onScreenCorrection = (bounds: Bounds, root: Size, offset: Offset): Offset => {
  const [dx, dy] = clampedDrag(bounds.left, bounds.top, bounds.right, bounds.bottom, root.width, root.height, 0, 0);
  return [pulledBack(bounds.right - bounds.left > root.width ? 0 : dx, offset[0]), pulledBack(bounds.bottom - bounds.top > root.height ? 0 : dy, offset[1])];
};

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

const rootSize = (): Size => ({ width: window.innerWidth, height: window.innerHeight });

const useRootBounds = (seen: (bounds: Bounds, root: Size) => void): ((node: HTMLElement | null) => () => void) => {
  const latest = useUpdatedRef(seen);
  const node = useRef<HTMLElement | null>(null);
  useLayoutEffect(() => {
    if (node.current) latest.current(node.current.getBoundingClientRect(), rootSize());
  });
  return useCallback((element: HTMLElement | null) => {
    node.current = element;
    const measure = () => {
      if (element) latest.current(element.getBoundingClientRect(), rootSize());
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
};

const useLayoutKey = (key: string): string => orientedKey(key, !useFlyIsPortrait());

const useKeptOnScreen = (key: string) => {
  const layout = useContext(LocalFlyScreenState).layout;
  return useRootBounds((bounds, root) => {
    const [x, y] = layout.offsets.value.get(key) ?? [0, 0];
    const [dx, dy] = onScreenCorrection(bounds, root, [x, y]);
    if (dx === 0 && dy === 0) return;
    layout.nudge(key, dx, dy);
    layout.saveOffset(key);
  });
};

export const useHoldToEditLayout = (): HTMLAttributes<HTMLElement> => {
  const layout = useContext(LocalFlyScreenState).layout;
  const editing = useValue(layout.editing);
  const locked = useValue(layout.locked);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cancel = () => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
  };
  useEffect(() => cancel, []);
  if (editing || locked) return {};
  return {
    onPointerDownCapture: (event) => {
      cancel();
      if (!event.currentTarget.contains(event.target as Node)) return;
      timer.current = setTimeout(() => {
        timer.current = null;
        layout.startEditing();
      }, LONG_PRESS_TIMEOUT_MS);
    },
    onPointerUpCapture: cancel,
    onPointerCancelCapture: cancel,
    onPointerLeave: cancel,
  };
};

const hashCode = (text: string): number => Array.from({ length: text.length }, (_, index) => text.charCodeAt(index)).reduce((hash, char) => (Math.imul(hash, 31) + char) | 0, 0);

const JIGGLE_STYLE_ID = 'aircast-overlay-jiggle';

const reducedMotion = (): boolean => typeof window !== 'undefined' && !!window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

const useJiggleAngle = (key: string, on: boolean): CSSProperties => {
  useEffect(() => {
    if (!on || document.getElementById(JIGGLE_STYLE_ID)) return;
    const style = Object.assign(document.createElement('style'), {
      id: JIGGLE_STYLE_ID,
      textContent: `@keyframes ${JIGGLE_STYLE_ID} { from { rotate: ${-JIGGLE_DEGREES}deg } to { rotate: ${JIGGLE_DEGREES}deg } }`,
    });
    document.head.append(style);
  }, [on]);
  return on && !reducedMotion() ? { animation: `${JIGGLE_STYLE_ID} ${JIGGLE_MILLIS + (hashCode(key) & JIGGLE_SPREAD)}ms ease-in-out infinite alternate` } : {};
};

export const clampedDrag = (left: number, top: number, right: number, bottom: number, width: number, height: number, dx: number, dy: number): Offset => [
  coerceIn(dx, -left, coerceAtLeast(width - right, -left)),
  coerceIn(dy, -top, coerceAtLeast(height - bottom, -top)),
];

export const LayoutDragArea = ({ widgetKey, className }: { widgetKey: string; className?: string }) => {
  const layout = useContext(LocalFlyScreenState).layout;
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
      className={`touch-none ${className ?? ''}`}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { id: event.pointerId, x: event.clientX, y: event.clientY, dragging: false };
      }}
      onPointerMove={(event) => {
        const held = drag.current;
        if (held === null || held.id !== event.pointerId) return;
        const rawX = event.clientX - held.x;
        const rawY = event.clientY - held.y;
        const distance = Math.hypot(rawX, rawY);
        if (!held.dragging && distance <= TOUCH_SLOP) return;
        const past = held.dragging ? 1 : (distance - TOUCH_SLOP) / distance;
        const bounds = event.currentTarget.getBoundingClientRect();
        const root = rootSize();
        const [dx, dy] = clampedDrag(bounds.left, bounds.top, bounds.right, bounds.bottom, root.width, root.height, rawX * past, rawY * past);
        drag.current = { id: held.id, x: event.clientX, y: event.clientY, dragging: true };
        layout.nudge(placedKey, dx, dy);
      }}
      onPointerUp={end}
      onPointerCancel={end}
    />
  );
};

const BADGE = 'absolute end-0 top-0 grid place-items-center';

export const LayoutWidget = ({ widgetKey, movable = true, hideable = true, children }: { widgetKey: string; movable?: boolean; hideable?: boolean; children?: ReactNode }) => {
  const layout = useContext(LocalFlyScreenState).layout;
  const editing = useValue(layout.editing);
  const hidden = useValue(layout.hidden).has(widgetKey);
  const offsets = useValue(layout.offsets);
  const avoided = useContext(LocalAvoidedByVideoMessage);
  const avoidedRef = useAvoidedByVideoMessage(widgetKey);
  const osd = useContext(LocalFlyOsd);
  const placedKey = useLayoutKey(widgetKey);
  const kept = useKeptOnScreen(placedKey);
  const hold = useHoldToEditLayout();
  const [shown, setShown] = useState(false);
  const sized = useCallback((node: HTMLElement | null) => {
    if (!node || typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(() => setShown(node.offsetWidth > 0 && node.offsetHeight > 0));
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  const decorated = editing && shown;
  const jiggle = useJiggleAngle(widgetKey, decorated);
  if (hidden && !editing) return null;
  const [x, y] = movable ? (offsets.get(placedKey) ?? [0, 0]) : [0, 0];
  return (
    <div ref={movable ? kept : undefined} style={{ translate: `${x}px ${y}px` }} {...hold}>
      <div ref={avoided ? avoidedRef : undefined} className="relative" style={jiggle}>
        <div
          ref={sized}
          className={osd ? osdShadow() : undefined}
          style={{ opacity: hidden ? HIDDEN_ALPHA : 1, ...(decorated ? { outline: `1px solid ${md('outline')}`, outlineOffset: -1, borderRadius: radius('medium') } : {}) }}
        >
          {children}
        </div>
        {decorated && movable ? <LayoutDragArea widgetKey={widgetKey} className="absolute inset-0" /> : null}
        {decorated && hideable ? (
          <Surface
            onClick={() => layout.setHidden(widgetKey, !hidden)}
            className={BADGE}
            style={{ width: BADGE_SIZE, height: BADGE_SIZE }}
            shape="full"
            color={hidden ? md('primary') : md('inverseSurface')}
            contentColor={hidden ? md('onPrimary') : md('inverseOnSurface')}
            shadow
            label={hidden ? 'Show' : 'Hide'}
          >
            <MaterialIcon name={hidden ? 'add' : 'close'} filled size={BADGE_SIZE - 8} />
          </Surface>
        ) : null}
        {decorated && !hideable ? (
          <Surface className={BADGE} style={{ width: BADGE_SIZE, height: BADGE_SIZE }} shape="full" color={md('surfaceContainerHighest')} contentColor={md('onSurfaceVariant')} shadow>
            <MaterialIcon name="lock" filled size={BADGE_SIZE - 10} label="Always shown" />
          </Surface>
        ) : null}
      </div>
    </div>
  );
};

const EditBar = ({ className }: { className?: string }) => {
  const layout = useContext(LocalFlyScreenState).layout;
  const valueSize = useValue(layout.valueSize);
  useBackHandler(true, () => {
    layout.editing.value = false;
  });
  const vehicleClass = instrumentVehicleClass(useQgcPath(INSTRUMENTS_VIEW));
  const [armed, setArmed] = useState(false);
  useLaunchedEffect([armed], async (scope) => {
    if (!armed) return;
    await delay(RESET_ARM_MILLIS);
    if (scope.isActive()) setArmed(false);
  });
  return (
    <Surface className={`w-full ${className ?? ''}`} shape="large" color={md('surfaceContainerHigh')} shadow>
      <div className="flex items-center px-2 py-1">
        <TextButton
          onClick={() => {
            const next = nextValueSize(valueSize);
            layout.valueSize.value = next;
            writeValueSize(vehicleClass, next);
          }}
        >
          {valueSizePillText(valueSize)}
        </TextButton>
        <TextButton
          contentColor={armed ? md('error') : undefined}
          onClick={() => {
            const tap = resetTap(armed);
            if (tap.reset) layout.reset();
            setArmed(tap.armed);
          }}
        >
          {resetPillText(armed)}
        </TextButton>
        <span className="flex-1" />
        <Button
          onClick={() => {
            layout.editing.value = false;
          }}
        >
          Done
        </Button>
      </div>
    </Surface>
  );
};

export const OverlayEditBar = ({ className }: { className?: string }) => {
  const editing = useValue(useContext(LocalFlyScreenState).layout.editing);
  return editing ? <EditBar className={className} /> : null;
};
