import type { Rect } from './FlyShell';

export const NO_VIDEO_PILL_WIDTH = 180;
export const NO_VIDEO_PILL_HEIGHT = 48;

export const awaitingAircraftText = (syntheticChosen: boolean): string =>
  syntheticChosen ? 'Synthetic view starts when your aircraft connects' : 'Video starts when your aircraft connects';

export const centredRegion = (region: Rect, centreX: number, minWidth: number): Rect => {
  const half = Math.min(centreX - region.left, region.right - centreX);
  return half * 2 >= minWidth ? { ...region, left: centreX - half, right: centreX + half, width: half * 2 } : region;
};

export const messageRegion = (_free: Rect, _obstacles: readonly Rect[], _gap: number, _minWidth: number, _minHeight: number): Rect | null => null;

export const FlyNoVideoMessage = () => null;

export const VideoSurface = (_props: { className?: string; expanded?: boolean; fullScreen?: boolean; onClick?: () => void; onDoubleTap?: () => void }) => null;
