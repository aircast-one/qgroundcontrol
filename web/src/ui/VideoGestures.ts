import { type CSSProperties, type HTMLAttributes, type PointerEvent, type SyntheticEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';
import { useRememberCameraSwiper } from '@/ui/CameraSwitch';
import { Offset, Rect, type Size, minus } from '@/ui/FlyShell';

export type VideoSwipe = { readonly name: 'Up' | 'Down' | 'Left' | 'Right'; readonly ordinal: number };

export const VideoSwipe = {
  Up: { name: 'Up', ordinal: 0 },
  Down: { name: 'Down', ordinal: 1 },
  Left: { name: 'Left', ordinal: 2 },
  Right: { name: 'Right', ordinal: 3 },
} as const satisfies Record<string, VideoSwipe>;

export const VideoSwipeEntries: readonly VideoSwipe[] = [VideoSwipe.Up, VideoSwipe.Down, VideoSwipe.Left, VideoSwipe.Right];

export const videoSwipe = (moved: Offset, threshold: number): VideoSwipe | null =>
  Math.max(Math.abs(moved.x), Math.abs(moved.y)) < threshold
    ? null
    : Math.abs(moved.y) > Math.abs(moved.x)
      ? moved.y < 0 ? VideoSwipe.Up : VideoSwipe.Down
      : moved.x < 0 ? VideoSwipe.Left : VideoSwipe.Right;

export const sideways = (moved: Offset): boolean => Math.abs(moved.x) > Math.abs(moved.y);

export type PipCorner = { readonly name: 'TopStart' | 'TopEnd' | 'BottomStart' | 'BottomEnd'; readonly ordinal: number; readonly start: boolean; readonly bottom: boolean };

export const PipCorner = {
  TopStart: { name: 'TopStart', ordinal: 0, start: true, bottom: false },
  TopEnd: { name: 'TopEnd', ordinal: 1, start: false, bottom: false },
  BottomStart: { name: 'BottomStart', ordinal: 2, start: true, bottom: true },
  BottomEnd: { name: 'BottomEnd', ordinal: 3, start: false, bottom: true },
} as const satisfies Record<string, PipCorner>;

export const PipCornerEntries: readonly PipCorner[] = [PipCorner.TopStart, PipCorner.TopEnd, PipCorner.BottomStart, PipCorner.BottomEnd];

export const hidingSwipe = (corner: PipCorner): VideoSwipe => (corner.bottom ? VideoSwipe.Down : VideoSwipe.Up);

export const growingSwipe = (corner: PipCorner): VideoSwipe => (corner.bottom ? VideoSwipe.Up : VideoSwipe.Down);

export type VideoGestureHandlers = {
  owned: () => boolean;
  claimsSwipe: (moved: Offset) => boolean;
  onTap: () => void;
  onDoubleTap: () => void;
  onSwipe: (moved: Offset) => void;
  onHold: (at: Offset) => boolean;
  onHoldDrag: (delta: Offset) => void;
  onHoldEnd: () => void;
};

export const TOUCH_SLOP = 8;
export const LONG_PRESS_TIMEOUT_MS = 400;
export const DOUBLE_TAP_TIMEOUT_MS = 300;

type Timer = ReturnType<typeof setTimeout> | undefined;

const at = (event: PointerEvent<HTMLElement>): Offset => Offset(event.clientX, event.clientY);

const local = (event: PointerEvent<HTMLElement>): Offset => {
  const box = event.currentTarget.getBoundingClientRect();
  return Offset(event.clientX - box.left, event.clientY - box.top);
};

const distance = (moved: Offset): number => Math.hypot(moved.x, moved.y);

const consume = (event: SyntheticEvent) => {
  event.stopPropagation();
  event.preventDefault();
};

const capture = (element: Element | null, pointer: number) => {
  try {
    element?.setPointerCapture(pointer);
  } catch {
    return;
  }
};

const INTERACTIVE = 'button, a, input, textarea, select, [role=button], [role=slider], [role=switch], [role=tab], [data-gesture]';

const onControl = (event: PointerEvent<HTMLElement>): boolean => {
  const hit = event.target instanceof Element ? event.target.closest(INTERACTIVE) : null;
  return hit !== null && hit !== event.currentTarget && event.currentTarget.contains(hit);
};

type Phase = 'idle' | 'ignored' | 'pressing' | 'swiping' | 'holding' | 'waiting' | 'again';

export const videoGestures = (handlers: VideoGestureHandlers, pass: 'Initial' | 'Main' = 'Initial'): HTMLAttributes<HTMLElement> => {
  const state = {
    phase: 'idle' as Phase,
    pointer: -1,
    from: Offset(0, 0),
    last: Offset(0, 0),
    owned: false,
    pressed: new Set<number>(),
    timer: undefined as Timer,
    element: null as HTMLElement | null,
    swallow: false,
  };
  const stopTimer = () => {
    clearTimeout(state.timer);
    state.timer = undefined;
  };
  const settle = () => {
    stopTimer();
    state.phase = state.pressed.size === 0 ? 'idle' : 'ignored';
  };
  const follow = (phase: Phase) => {
    state.phase = phase;
    state.swallow = true;
    capture(state.element, state.pointer);
  };
  const down = (event: PointerEvent<HTMLElement>) => {
    if (onControl(event)) {
      state.swallow = false;
      return;
    }
    if (event.isPrimary && state.phase !== 'waiting') {
      stopTimer();
      state.pressed = new Set();
      state.phase = 'idle';
    }
    state.pressed = new Set([...state.pressed, event.pointerId]);
    if (state.phase === 'waiting') {
      stopTimer();
      consume(event);
      state.pointer = event.pointerId;
      state.element = event.currentTarget;
      state.last = at(event);
      follow('again');
      return;
    }
    if (state.phase === 'pressing' && state.pressed.size > 1) {
      settle();
      return;
    }
    if (state.phase !== 'idle') return;
    state.swallow = false;
    if (event.defaultPrevented) {
      state.phase = 'ignored';
      return;
    }
    state.owned = handlers.owned();
    if (state.owned) {
      consume(event);
      state.swallow = true;
    }
    state.phase = 'pressing';
    state.pointer = event.pointerId;
    state.element = event.currentTarget;
    state.from = at(event);
    state.last = state.from;
    const held = local(event);
    state.timer = setTimeout(() => {
      if (state.phase !== 'pressing') return;
      state.timer = undefined;
      if (!handlers.onHold(held)) {
        settle();
        return;
      }
      follow('holding');
    }, LONG_PRESS_TIMEOUT_MS);
  };
  const move = (event: PointerEvent<HTMLElement>) => {
    if (event.pointerId !== state.pointer) return;
    const now = at(event);
    if (state.phase === 'pressing') {
      if (event.defaultPrevented) {
        settle();
        return;
      }
      if (state.owned) consume(event);
      const moved = minus(now, state.from);
      if (distance(moved) <= TOUCH_SLOP) return;
      stopTimer();
      state.last = now;
      if (state.owned || handlers.claimsSwipe(moved)) {
        consume(event);
        follow('swiping');
        return;
      }
      settle();
      return;
    }
    if (state.phase === 'holding') {
      consume(event);
      handlers.onHoldDrag(minus(now, state.last));
      state.last = now;
      return;
    }
    if (state.phase === 'swiping' || state.phase === 'again') {
      consume(event);
      state.last = now;
    }
  };
  const finish = (event: PointerEvent<HTMLElement>, cancelled: boolean) => {
    state.pressed = new Set([...state.pressed].filter((pointer) => pointer !== event.pointerId));
    if (event.pointerId !== state.pointer) {
      if (state.phase === 'ignored' && state.pressed.size === 0) state.phase = 'idle';
      return;
    }
    const now = cancelled ? state.last : at(event);
    const phase = state.phase;
    if (phase === 'pressing') {
      stopTimer();
      if (cancelled) {
        settle();
        return;
      }
      if (state.owned) consume(event);
      state.phase = 'waiting';
      const owned = state.owned;
      state.timer = setTimeout(() => {
        if (state.phase !== 'waiting') return;
        state.timer = undefined;
        state.phase = 'idle';
        if (owned) handlers.onTap();
      }, DOUBLE_TAP_TIMEOUT_MS);
      return;
    }
    if (phase === 'swiping' || phase === 'holding' || phase === 'again') consume(event);
    settle();
    if (phase === 'swiping') handlers.onSwipe(minus(now, state.from));
    if (phase === 'holding') handlers.onHoldEnd();
    if (phase === 'again') handlers.onDoubleTap();
  };
  const up = (event: PointerEvent<HTMLElement>) => finish(event, false);
  const cancel = (event: PointerEvent<HTMLElement>) => finish(event, true);
  const swallowClick = (event: SyntheticEvent) => {
    if (state.swallow) consume(event);
  };
  return pass === 'Initial'
    ? { onPointerDownCapture: down, onPointerMoveCapture: move, onPointerUpCapture: up, onPointerCancelCapture: cancel, onClickCapture: swallowClick, onDoubleClickCapture: swallowClick }
    : { onPointerDown: down, onPointerMove: move, onPointerUp: up, onPointerCancel: cancel, onClickCapture: swallowClick, onDoubleClickCapture: swallowClick };
};

export const verticalSwipe = (threshold: number, onSwipe: (swipe: VideoSwipe) => void): HTMLAttributes<HTMLElement> => {
  const state = { pointer: -1, from: Offset(0, 0), last: Offset(0, 0), moving: false };
  const finish = (event: PointerEvent<HTMLElement>, now: Offset) => {
    if (event.pointerId !== state.pointer) return;
    const moving = state.moving;
    state.pointer = -1;
    state.moving = false;
    if (!moving) return;
    event.stopPropagation();
    const swipe = videoSwipe(minus(now, state.from), threshold);
    if (swipe !== null) onSwipe(swipe);
  };
  return {
    onPointerDown: (event) => {
      if (state.pointer !== -1 && !event.isPrimary) return;
      state.pointer = event.pointerId;
      state.from = at(event);
      state.last = state.from;
      state.moving = false;
    },
    onPointerMove: (event) => {
      if (event.pointerId !== state.pointer) return;
      const now = at(event);
      if (!state.moving && distance(minus(now, state.from)) <= TOUCH_SLOP) return;
      if (!state.moving) capture(event.currentTarget, event.pointerId);
      state.moving = true;
      state.last = now;
      consume(event);
    },
    onPointerUp: (event) => finish(event, at(event)),
    onPointerCancel: (event) => finish(event, state.last),
  };
};

export type PointerGestureHandlers = {
  onDragStart?: (at: Offset) => void;
  onDrag?: (at: Offset, dragged: Offset, size: Size) => void;
  onDragEnd?: () => void;
  onDoubleTap?: () => void;
};

export const usePointerGestures = (handlers: PointerGestureHandlers | null): HTMLAttributes<HTMLElement> => {
  const latest = useUpdatedRef(handlers);
  const enabled = handlers !== null;
  return useMemo(() => {
    if (!enabled) return {};
    const state = { pointer: -1, from: Offset(0, 0), last: Offset(0, 0), dragging: false, tapUpAt: -Infinity, second: false };
    const sizeOf = (event: PointerEvent<HTMLElement>) => ({ width: event.currentTarget.clientWidth, height: event.currentTarget.clientHeight });
    const end = (event: PointerEvent<HTMLElement>, cancelled: boolean) => {
      if (event.pointerId !== state.pointer) return;
      state.pointer = -1;
      if (state.dragging) {
        state.dragging = false;
        consume(event);
        if (!cancelled) latest.current?.onDragEnd?.();
        return;
      }
      if (cancelled) {
        state.second = false;
        return;
      }
      if (state.second) {
        state.second = false;
        state.tapUpAt = -Infinity;
        latest.current?.onDoubleTap?.();
        return;
      }
      state.tapUpAt = event.timeStamp;
    };
    return {
      onPointerDown: (event) => {
        if (state.pointer !== -1 && !event.isPrimary) return;
        state.pointer = event.pointerId;
        state.from = local(event);
        state.last = state.from;
        state.dragging = false;
        state.second = event.timeStamp - state.tapUpAt <= DOUBLE_TAP_TIMEOUT_MS;
      },
      onPointerMove: (event) => {
        if (event.pointerId !== state.pointer) return;
        const now = local(event);
        const handlers = latest.current;
        if (handlers?.onDrag === undefined) return;
        if (!state.dragging) {
          const moved = minus(now, state.from);
          const far = distance(moved);
          if (far <= TOUCH_SLOP) return;
          state.second = false;
          state.tapUpAt = -Infinity;
          state.dragging = true;
          capture(event.currentTarget, event.pointerId);
          consume(event);
          handlers.onDragStart?.(now);
          const overSlop = Offset(moved.x - (moved.x / far) * TOUCH_SLOP, moved.y - (moved.y / far) * TOUCH_SLOP);
          state.last = now;
          handlers.onDrag(now, overSlop, sizeOf(event));
          return;
        }
        consume(event);
        const dragged = minus(now, state.last);
        state.last = now;
        handlers.onDrag(now, dragged, sizeOf(event));
      },
      onPointerUp: (event) => end(event, false),
      onPointerCancel: (event) => end(event, true),
    };
  }, [enabled]);
};

export const CAMERA_NUDGE = 48;
export const VIDEO_SWIPE_DISTANCE = 32;

const SPRING_STIFFNESS_MEDIUM_LOW = 400;
const OMEGA = Math.sqrt(SPRING_STIFFNESS_MEDIUM_LOW);
const RECT_VISIBILITY = 0.5;
const FLOAT_VISIBILITY = 0.01;

type Motion = { readonly values: readonly number[]; readonly velocities: readonly number[] };

const resting = (values: readonly number[]): Motion => ({ values, velocities: values.map(() => 0) });

const springed = (from: Motion, to: readonly number[], seconds: number): Motion => {
  const decay = Math.exp(-OMEGA * seconds);
  const gaps = to.map((end, index) => (from.values[index] ?? end) - end);
  const speeds = to.map((_, index) => from.velocities[index] ?? 0);
  return {
    values: to.map((end, index) => end + ((gaps[index] ?? 0) + ((speeds[index] ?? 0) + OMEGA * (gaps[index] ?? 0)) * seconds) * decay),
    velocities: to.map((_, index) => ((speeds[index] ?? 0) - OMEGA * ((speeds[index] ?? 0) + OMEGA * (gaps[index] ?? 0)) * seconds) * decay),
  };
};

const reducedMotion = (): boolean => typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches === true;

type Animatable = { value: readonly number[]; current: () => readonly number[]; snapTo: (to: readonly number[]) => void; animateTo: (to: readonly number[]) => void };

const useAnimatable = (initial: readonly number[], threshold: number): Animatable => {
  const [value, setValue] = useState(initial);
  const motion = useRef<Motion>(resting(initial));
  const run = useRef(0);
  const show = useCallback((next: Motion) => {
    motion.current = next;
    setValue(next.values);
  }, []);
  const snapTo = useCallback((to: readonly number[]) => {
    run.current = run.current + 1;
    show(resting(to));
  }, []);
  const animateTo = useCallback((to: readonly number[]) => {
    if (reducedMotion() || typeof requestAnimationFrame === 'undefined') return snapTo(to);
    const id = run.current + 1;
    run.current = id;
    const start = motion.current;
    const began = performance.now();
    const frame = (now: number) => {
      if (run.current !== id) return;
      const moved = springed(start, to, Math.max(0, now - began) / 1000);
      const settled = moved.values.every((it, index) => Math.abs(it - (to[index] ?? it)) < threshold) && moved.velocities.every((it) => Math.abs(it) < threshold);
      show(settled ? resting(to) : moved);
      if (!settled) requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
    return undefined;
  }, []);
  useEffect(() => () => {
    run.current = run.current + 1;
  }, []);
  const current = useCallback(() => motion.current.values, []);
  return { value, current, snapTo, animateTo };
};

const rectValues = (rect: Rect): readonly number[] => [rect.left, rect.top, rect.right, rect.bottom];

const rectOf = (values: readonly number[]): Rect => {
  const [left = 0, top = 0, right = 0, bottom = 0] = values;
  return Rect(left, top, right, bottom);
};

const isEmpty = (rect: Rect): boolean => rect.left >= rect.right || rect.top >= rect.bottom;

export const useRememberVideoFrame = (target: Rect, holding: boolean): { value: Rect } => {
  const frame = useAnimatable(rectValues(target), RECT_VISIBILITY);
  useEffect(() => {
    if (holding || isEmpty(rectOf(frame.current()))) frame.snapTo(rectValues(target));
    else frame.animateTo(rectValues(target));
  }, [target.left, target.top, target.right, target.bottom, holding]);
  return { value: rectOf(frame.value) };
};

export const videoFrame = (target: Rect, shown: () => Rect, nudge: () => number): CSSProperties => {
  const now = shown();
  const scaleX = target.width > 0 ? now.width / target.width : 1;
  const scaleY = target.height > 0 ? now.height / target.height : 1;
  return {
    position: 'absolute',
    left: Math.round(target.left),
    top: Math.round(target.top),
    width: Math.max(Math.round(target.width), 0),
    height: Math.max(Math.round(target.height), 0),
    transformOrigin: '0 0',
    transform: `translate(${now.left - target.left + nudge()}px, ${now.top - target.top}px) scale(${scaleX}, ${scaleY})`,
  };
};

export type CameraStepper = { step: (step: number) => void; nudge: () => number };

export const useRememberCameraStepper = (): CameraStepper => {
  const swipeCamera = useUpdatedRef(useRememberCameraSwiper());
  const nudge = useAnimatable([0], FLOAT_VISIBILITY);
  return useMemo(
    () => ({
      step: (step: number) => {
        if (!swipeCamera.current(step)) return;
        nudge.snapTo([CAMERA_NUDGE * step]);
        nudge.animateTo([0]);
      },
      nudge: () => nudge.current()[0] ?? 0,
    }),
    [],
  );
};

export const cameraStep = (swipe: VideoSwipe | null): number | null => (swipe === VideoSwipe.Left ? 1 : swipe === VideoSwipe.Right ? -1 : null);
