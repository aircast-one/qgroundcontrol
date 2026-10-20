import { type HTMLAttributes, type PointerEvent, type SyntheticEvent, useMemo } from 'react';
import { type Json, jsonStrings, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useUpdatedRef } from '@/kotlin/Compose';
import { screenFraction } from '@/ui/GimbalIndicator';
import { GOTO_ACTION, MAP_CLICK_PATH, type MapPoint, mapClickActions } from '@/ui/MapClickMenu';
import { LONG_PRESS_TIMEOUT_MS, TOUCH_SLOP } from '@/ui/VideoGestures';

export const GROUND_AT = 'videoAr.groundAt';

export const PICTURE_RIGHT_CLICK_HINT = 'Right-click the picture to fly there';
export const PICTURE_HOLD_HINT = 'Press and hold the picture to fly there';

export const pictureHoldHintText = (desktop: boolean): string => (desktop ? PICTURE_RIGHT_CLICK_HINT : PICTURE_HOLD_HINT);

export type PictureSpot = { readonly x: number; readonly y: number };

export type PicturePress = { readonly spot: PictureSpot; readonly point: MapPoint | null; readonly actions: readonly string[]; readonly reason: string };

export const picturePress = (spot: PictureSpot, answer: Json | null): PicturePress => {
  const point = optBoolean(answer, 'ok') ? { latitude: optDouble(answer, 'latitude'), longitude: optDouble(answer, 'longitude') } : null;
  return {
    spot,
    point: point !== null && Number.isFinite(point.latitude) && Number.isFinite(point.longitude) ? point : null,
    actions: jsonStrings(optJSONArray(answer, 'actions')),
    reason: optText(answer, 'reason'),
  };
};

const goToOffered = (view: Json | null): boolean => mapClickActions(view).some((it) => it.id === GOTO_ACTION);

export const usePictureGoTo = (): boolean => goToOffered(useQgcPath(MAP_CLICK_PATH));

const consume = (event: SyntheticEvent) => {
  event.stopPropagation();
  event.preventDefault();
};

type Timer = ReturnType<typeof setTimeout> | undefined;

export type PictureHoldProps = HTMLAttributes<HTMLElement> & { 'data-hold'?: boolean };

export const usePictureHold = (onPress: ((press: PicturePress) => void) | null, synthetic: boolean, touchHold: boolean): PictureHoldProps => {
  const latest = useUpdatedRef({ onPress, synthetic });
  const enabled = onPress !== null;
  const handlers = useMemo((): PictureHoldProps => {
    if (!enabled) return {};
    const state = { pointer: -1, from: { x: 0, y: 0 }, held: -1, touch: false, timer: undefined as Timer };
    const stop = () => {
      clearTimeout(state.timer);
      state.timer = undefined;
      state.pointer = -1;
    };
    const ask = (element: Element, clientX: number, clientY: number) => {
      const box = element.getBoundingClientRect();
      if (box.width <= 0 || box.height <= 0) return;
      const [x, y] = [clientX - box.left, clientY - box.top];
      const [pan, tilt] = screenFraction(x, y, box.width, box.height);
      const spot = { x: x / box.width, y: y / box.height };
      void Qgc.call(GROUND_AT, pan, tilt, box.width / box.height, latest.current.synthetic).then((answer) => latest.current.onPress?.(picturePress(spot, answer)));
    };
    const release = (event: PointerEvent<HTMLElement>) => {
      if (event.pointerId === state.pointer) stop();
      if (event.pointerId !== state.held) return;
      state.held = -1;
      consume(event);
    };
    return {
      onContextMenu: (event) => {
        event.preventDefault();
        if (!state.touch) ask(event.currentTarget, event.clientX, event.clientY);
      },
      onPointerDownCapture: (event) => {
        state.touch = event.pointerType === 'touch';
        if (event.button === 2) event.stopPropagation();
        if (!event.isPrimary) stop();
        if (!event.isPrimary || event.pointerType === 'mouse' || !touchHold) return;
        stop();
        const element = event.currentTarget;
        const [pointer, x, y] = [event.pointerId, event.clientX, event.clientY];
        state.pointer = pointer;
        state.from = { x, y };
        state.timer = setTimeout(() => {
          stop();
          state.held = pointer;
          ask(element, x, y);
        }, LONG_PRESS_TIMEOUT_MS);
      },
      onPointerMoveCapture: (event) => {
        if (event.pointerId === state.held) consume(event);
        if (event.pointerId === state.pointer && Math.hypot(event.clientX - state.from.x, event.clientY - state.from.y) > TOUCH_SLOP) stop();
      },
      onPointerUpCapture: release,
      onPointerCancelCapture: release,
      'data-hold': touchHold || undefined,
    };
  }, [enabled, touchHold]);
  return handlers;
};

const MARK_SIZE = 32;
const MARK_RADIUS = 13;

export const PictureMark = ({ spot }: { spot: PictureSpot }) => (
  <svg
    role="img"
    aria-label="Chosen spot"
    width={MARK_SIZE}
    height={MARK_SIZE}
    viewBox={`${-MARK_SIZE / 2} ${-MARK_SIZE / 2} ${MARK_SIZE} ${MARK_SIZE}`}
    className="pointer-events-none absolute -translate-x-1/2 -translate-y-1/2"
    style={{ left: `${spot.x * 100}%`, top: `${spot.y * 100}%` }}
  >
    <circle r={MARK_RADIUS} fill="none" stroke="rgba(0, 0, 0, 0.6)" strokeWidth={4} />
    <circle r={MARK_RADIUS - 1} fill="none" stroke="#FFFFFF" strokeWidth={2} />
    <circle r={2} fill="#FFFFFF" />
  </svg>
);
