import type { CSSProperties, HTMLAttributes } from 'react';
import type { Offset, Rect } from './FlyShell';

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

export const VIDEO_SWIPE_DISTANCE = 32;

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

export const videoGestures = (_handlers: VideoGestureHandlers, _pass: 'Initial' | 'Main' = 'Initial'): HTMLAttributes<HTMLElement> => ({});

export const verticalSwipe = (_threshold: number, _onSwipe: (swipe: VideoSwipe) => void): HTMLAttributes<HTMLElement> => ({});

export const useRememberVideoFrame = (target: Rect, _holding: boolean): { value: Rect } => ({ value: target });

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

export const useRememberCameraStepper = (): CameraStepper => ({ step: () => undefined, nudge: () => 0 });

export const cameraStep = (swipe: VideoSwipe | null): number | null => (swipe === VideoSwipe.Left ? 1 : swipe === VideoSwipe.Right ? -1 : null);
