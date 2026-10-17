import { type PointerEvent, type ReactNode, type RefCallback, useEffect, useRef, useState } from 'react';
import { type Json, optBoolean, optLong, optString } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import { type Scope, useKeyedState, useUpdatedRef } from '@/kotlin/Compose';
import { coerceIn, same } from '@/kotlin/Std';
import { Color, aircast, withAlpha } from '@/theme/Colors';
import { type Offset, useAvoidedByVideoMessage } from '@/ui/FlyShell';
import { FLY_STATE, type FlyState, flyState } from '@/ui/FlyState';

export const VIRTUAL_JOYSTICK_PATH = 'view.virtualJoystick';
export const VIRTUAL_JOYSTICK_VALUE = 'vehicle.virtualTabletJoystickValue';
const DEFAULT_PERIOD_MS = 40;
const STICK_PAD_FRACTION = 0.42;
const MIN_STICK_PAD = 96;
const MAX_STICK_PAD = 200;
const KNOB_FRACTION = 0.22;
const PAD_FILL_ALPHA = 0.12;
const RING_ALPHA = 0.45;
const HELD_RING_ALPHA = 0.8;
const KNOB_ALPHA = 0.28;
const HELD_KNOB_ALPHA = 0.45;
const LEFT_STICK_KEY = 'leftStick';
const RIGHT_STICK_KEY = 'rightStick';

export type VirtualJoystickState = {
  enabled: boolean;
  show: boolean;
  sending: boolean;
  autoCenterThrottle: boolean;
  leftHandedMode: boolean;
  leftPositiveOnly: boolean;
  rightPositiveOnly: boolean;
  periodMs: number;
};

export const virtualJoystick = (view: Json | null): VirtualJoystickState | null =>
  view === null || optString(view, 'class') !== 'VirtualJoystick'
    ? null
    : {
        enabled: optBoolean(view, 'enabled'),
        show: optBoolean(view, 'show'),
        sending: optBoolean(view, 'sending'),
        autoCenterThrottle: optBoolean(view, 'autoCenterThrottle'),
        leftHandedMode: optBoolean(view, 'leftHandedMode'),
        leftPositiveOnly: optBoolean(view, 'leftPositiveOnly'),
        rightPositiveOnly: optBoolean(view, 'rightPositiveOnly'),
        periodMs: optLong(view, 'periodMs', DEFAULT_PERIOD_MS),
      };

export type StickAxes = { x: number; y: number };

export type StickStates = { previous: VirtualJoystickState | null; current: VirtualJoystickState | null };

export type Sticks = { left: Offset; right: Offset };

export const CENTRED: Offset = { x: 0.5, y: 0.5 };

const GROUND_STATES: ReadonlySet<string> = new Set(['disarmed', 'armed']);

export const sticksReset = (previous: VirtualJoystickState | null, current: VirtualJoystickState | null): boolean =>
  current?.enabled !== true || current.autoCenterThrottle !== previous?.autoCenterThrottle || current.leftHandedMode !== previous.leftHandedMode;

export const onTheGround = (fly: FlyState | null): boolean => fly?.connected === true && GROUND_STATES.has(fly.state);

export const throttleOnLeft = (state: VirtualJoystickState): boolean => !state.leftHandedMode;

export const throttleParked = (state: VirtualJoystickState, grounded: boolean): boolean => grounded && (throttleOnLeft(state) ? state.leftPositiveOnly : state.rightPositiveOnly);

export const released = (stick: Offset, throttle: boolean, autoCenter: boolean, parked: boolean): Offset => ({ x: 0.5, y: !throttle || autoCenter || parked ? 0.5 : stick.y });

export const shownSticks = (state: VirtualJoystickState, left: Offset | null, right: Offset | null, grounded: boolean, leftHeld: boolean, rightHeld: boolean): Sticks => {
  const parked = throttleParked(state, grounded);
  const park = (stick: Offset, held: boolean): Offset => (parked && !held ? { x: stick.x, y: 1 } : stick);
  const l = left ?? CENTRED;
  const r = right ?? CENTRED;
  return throttleOnLeft(state) ? { left: park(l, leftHeld), right: r } : { left: l, right: park(r, rightHeld) };
};

export const stickValues = (state: VirtualJoystickState, sticks: Sticks): readonly number[] =>
  joystickValues(stickAxes(sticks.left.x, sticks.left.y, state.leftPositiveOnly), stickAxes(sticks.right.x, sticks.right.y, state.rightPositiveOnly), state.leftHandedMode);

const moved = { pending: false, wake: null as (() => void) | null };

const signalMoved = () => {
  moved.pending = true;
  moved.wake?.();
};

const awaitMoved = (milliseconds: number): Promise<void> => {
  if (moved.pending) {
    moved.pending = false;
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    const finish = () => {
      clearTimeout(timer);
      if (moved.wake === finish) moved.wake = null;
      moved.pending = false;
      resolve();
    };
    const timer = setTimeout(() => {
      if (moved.wake === finish) moved.wake = null;
      resolve();
    }, milliseconds);
    moved.wake = finish;
  });
};

const touch = { at: 0 };

export const VirtualStickSender = {
  left: null as Offset | null,
  right: null as Offset | null,
  layout: null as readonly [boolean, boolean] | null,
  leftHeld: false,
  rightHeld: false,

  moveLeft(stick: Offset, at: number): void {
    VirtualStickSender.left = stick;
    if (touch.at === 0) touch.at = at;
    signalMoved();
  },

  moveRight(stick: Offset, at: number): void {
    VirtualStickSender.right = stick;
    if (touch.at === 0) touch.at = at;
    signalMoved();
  },

  start(scope: Scope): void {
    const paths = [VIRTUAL_JOYSTICK_PATH, FLY_STATE];
    Qgc.watch(paths);
    const run = { generation: 0, last: undefined as VirtualJoystickState | null | undefined, states: { previous: null, current: null } as StickStates, released: false };
    const live = (generation: number) => scope.isActive() && run.generation === generation;
    const stop = () => {
      if (run.released) return;
      run.released = true;
      unsubscribe();
      Qgc.unwatch(paths);
    };
    const send = async (generation: number, sending: VirtualJoystickState): Promise<void> => {
      if (!live(generation)) return scope.isActive() ? undefined : stop();
      const sticks = shownSticks(sending, VirtualStickSender.left, VirtualStickSender.right, onTheGround(flyState(Qgc.path(FLY_STATE))), VirtualStickSender.leftHeld, VirtualStickSender.rightHeld);
      const touched = touch.at;
      touch.at = 0;
      const touchAge = touched > 0 ? [Math.max(0, Math.round(performance.now() - touched))] : [];
      await Qgc.invoke(VIRTUAL_JOYSTICK_VALUE, ...stickValues(sending, sticks), ...touchAge);
      if (!live(generation)) return scope.isActive() ? undefined : stop();
      await awaitMoved(sending.periodMs);
      return send(generation, sending);
    };
    const collect = () => {
      if (!scope.isActive()) return stop();
      const next = virtualJoystick(Qgc.path(VIRTUAL_JOYSTICK_PATH));
      if (run.last !== undefined && same(run.last, next)) return undefined;
      run.last = next;
      run.states = { previous: run.states.current, current: next };
      run.generation += 1;
      if (sticksReset(run.states.previous, run.states.current)) {
        VirtualStickSender.left = null;
        VirtualStickSender.right = null;
      }
      return next !== null && next.enabled && next.sending ? void send(run.generation, next) : undefined;
    };
    const unsubscribe = Qgc.subscribe(VIRTUAL_JOYSTICK_PATH, collect);
    collect();
  },
};

export const stickAxes = (fractionX: number, fractionY: number, positiveOnly: boolean): StickAxes => {
  const pctUp = 1.0 - coerceIn(fractionY, 0, 1);
  return { x: coerceIn(fractionX, 0, 1) * 2.0 - 1.0, y: positiveOnly ? pctUp : pctUp * 2.0 - 1.0 };
};

export const joystickValues = (left: StickAxes, right: StickAxes, leftHanded: boolean): readonly number[] =>
  leftHanded ? [left.x, left.y, right.x, right.y] : [right.x, right.y, left.x, left.y];

export const stickPad = (shortSide: number, room: number = Number.POSITIVE_INFINITY): number => coerceIn(Math.min(shortSide * STICK_PAD_FRACTION, room), MIN_STICK_PAD, MAX_STICK_PAD);

export const knobTravel = (pad: number): number => pad / 2 - pad * KNOB_FRACTION;

export const knobOffset = (stick: Offset, travel: number): Offset => {
  const wanted = { x: (stick.x - 0.5) * (2 * travel), y: (stick.y - 0.5) * (2 * travel) };
  const distance = Math.hypot(wanted.x, wanted.y);
  return distance > travel ? { x: wanted.x * (travel / distance), y: wanted.y * (travel / distance) } : wanted;
};

export const dragged = (start: Offset, movedBy: Offset, travel: number): Offset => ({
  x: coerceIn(start.x + movedBy.x / (2 * travel), 0, 1),
  y: coerceIn(start.y + movedBy.y / (2 * travel), 0, 1),
});

const shownOf = (json: Json | null): boolean => virtualJoystick(json)?.show === true;

export const useShownStickPad = (shortSide: number, room: number = Number.POSITIVE_INFINITY): number | null => {
  const shown = useQgcDerived(VIRTUAL_JOYSTICK_PATH, shownOf);
  return shown ? stickPad(shortSide, room) : null;
};

const groundedOf = (json: Json | null): boolean => onTheGround(flyState(json));

export const VirtualJoystick = ({ pad, className, trailing = null }: { pad: number; className?: string; trailing?: ReactNode }) => {
  const view = useQgcPath(VIRTUAL_JOYSTICK_PATH);
  const state = virtualJoystick(view);
  return state !== null && state.show ? <ShownVirtualJoystick state={state} pad={pad} className={className} trailing={trailing} /> : null;
};

const ShownVirtualJoystick = ({ state, pad, className, trailing }: { state: VirtualJoystickState; pad: number; className?: string; trailing: ReactNode }) => {
  const grounded = useQgcDerived(FLY_STATE, groundedOf);
  const layout = [state.autoCenterThrottle, state.leftHandedMode] as const;
  const kept = same(VirtualStickSender.layout, layout);
  const [left, setLeft] = useKeyedState<Offset>(layout, () => (kept ? VirtualStickSender.left : null) ?? CENTRED);
  const [right, setRight] = useKeyedState<Offset>(layout, () => (kept ? VirtualStickSender.right : null) ?? CENTRED);
  const [leftHeld, setLeftHeld] = useState(false);
  const [rightHeld, setRightHeld] = useState(false);
  const latestState = useUpdatedRef(state);
  const parked = throttleParked(state, grounded);
  const latestParked = useUpdatedRef(parked);
  const avoidLeft = useAvoidedByVideoMessage(LEFT_STICK_KEY);
  const avoidRight = useAvoidedByVideoMessage(RIGHT_STICK_KEY);

  useEffect(() => {
    VirtualStickSender.layout = layout;
    VirtualStickSender.left = left;
    VirtualStickSender.right = right;
    VirtualStickSender.leftHeld = leftHeld;
    VirtualStickSender.rightHeld = rightHeld;
  }, [layout[0], layout[1], left, right, leftHeld, rightHeld]);

  useEffect(
    () => () => {
      const current = latestState.current;
      const throttleLeft = throttleOnLeft(current);
      VirtualStickSender.left = VirtualStickSender.left === null ? null : released(VirtualStickSender.left, throttleLeft, current.autoCenterThrottle, latestParked.current);
      VirtualStickSender.right = VirtualStickSender.right === null ? null : released(VirtualStickSender.right, !throttleLeft, current.autoCenterThrottle, latestParked.current);
      VirtualStickSender.leftHeld = false;
      VirtualStickSender.rightHeld = false;
    },
    [],
  );

  const shown = shownSticks(state, left, right, grounded, leftHeld, rightHeld);
  const release = (stick: Offset, throttle: boolean): Offset => released(stick, throttle, latestState.current.autoCenterThrottle, latestParked.current);
  return (
    <div className={`flex w-full items-end ${className ?? ''}`}>
      <ThumbPad
        stick={shown.left}
        description="Left virtual stick"
        size={pad}
        padRef={avoidLeft}
        onHeld={setLeftHeld}
        onMove={(stick, at) => {
          setLeft(stick);
          VirtualStickSender.moveLeft(stick, at);
        }}
        onRelease={(stick, at) => {
          const rest = release(stick, throttleOnLeft(latestState.current));
          setLeft(rest);
          VirtualStickSender.moveLeft(rest, at);
        }}
      />
      <div className="flex-1" />
      <ThumbPad
        stick={shown.right}
        description="Right virtual stick"
        size={pad}
        padRef={avoidRight}
        onHeld={setRightHeld}
        onMove={(stick, at) => {
          setRight(stick);
          VirtualStickSender.moveRight(stick, at);
        }}
        onRelease={(stick, at) => {
          const rest = release(stick, !throttleOnLeft(latestState.current));
          setRight(rest);
          VirtualStickSender.moveRight(rest, at);
        }}
      />
      {trailing}
    </div>
  );
};

type Gesture = { pointer: number; origin: Offset; start: Offset; travel: number };

const ThumbPad = ({
  stick,
  description,
  size,
  padRef,
  onHeld,
  onMove,
  onRelease,
}: {
  stick: Offset;
  description: string;
  size: number;
  padRef: RefCallback<HTMLElement>;
  onHeld: (held: boolean) => void;
  onMove: (stick: Offset, at: number) => void;
  onRelease: (stick: Offset, at: number) => void;
}) => {
  const ink = aircast('outdoorForeground');
  const latestStick = useUpdatedRef(stick);
  const [held, setHeld] = useState(false);
  const gesture = useRef<Gesture | null>(null);

  const down = (event: PointerEvent<HTMLDivElement>) => {
    if (gesture.current !== null) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = { pointer: event.pointerId, origin: { x: event.clientX, y: event.clientY }, start: latestStick.current, travel: knobTravel(event.currentTarget.getBoundingClientRect().width) };
    setHeld(true);
    onHeld(true);
  };
  const move = (event: PointerEvent<HTMLDivElement>) => {
    const active = gesture.current;
    if (active === null || active.pointer !== event.pointerId) return;
    onMove(dragged(active.start, { x: event.clientX - active.origin.x, y: event.clientY - active.origin.y }, active.travel), event.timeStamp);
  };
  const lift = (event: PointerEvent<HTMLDivElement>) => {
    if (gesture.current === null || gesture.current.pointer !== event.pointerId) return;
    gesture.current = null;
    setHeld(false);
    onHeld(false);
    onRelease(latestStick.current, event.timeStamp);
  };

  const radius = size / 2;
  const at = knobOffset(stick, knobTravel(size));
  return (
    <div
      ref={padRef}
      role="img"
      aria-label={description}
      className="shrink-0 select-none"
      style={{ width: size, height: size, touchAction: 'none' }}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={lift}
      onPointerCancel={lift}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden>
        <circle cx={radius} cy={radius} r={radius} style={{ fill: withAlpha(Color.Black, PAD_FILL_ALPHA) }} />
        <circle cx={radius} cy={radius} r={radius - 0.75} strokeWidth={1.5} style={{ fill: 'none', stroke: withAlpha(ink, held ? HELD_RING_ALPHA : RING_ALPHA) }} />
        <circle cx={radius + at.x} cy={radius + at.y} r={size * KNOB_FRACTION} style={{ fill: withAlpha(ink, held ? HELD_KNOB_ALPHA : KNOB_ALPHA) }} />
      </svg>
    </div>
  );
};
