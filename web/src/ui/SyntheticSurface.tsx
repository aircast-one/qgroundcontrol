import { useEffect, useRef } from 'react';
import { type Json, optBoolean, optString } from '@/bridge/Json';
import { coerceAtLeast, roundToInt } from '@/kotlin/Std';
import { SyntheticDraw } from '@/ui/SyntheticView';

export const RENDER_SCALE = 0.75;

export const renderSize = (viewPx: number): number => coerceAtLeast(roundToInt(viewPx * RENDER_SCALE), 1);

export const nativeChosen = (view: Json | null): boolean => optBoolean(view, 'native');

export const nativeDraw = (view: Json | null): SyntheticDraw => {
  const draw = view === null ? null : optString(view, 'draw');
  return draw === 'drawn' ? SyntheticDraw.Drawn : draw === 'failed' ? SyntheticDraw.Failed : SyntheticDraw.Loading;
};

export const SyntheticSurface = ({ className }: { className?: string }) => {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const surface = canvas.current;
    if (surface === null) return undefined;
    const observer = new ResizeObserver(() => {
      if (surface.clientWidth <= 0 || surface.clientHeight <= 0) return;
      surface.width = renderSize(surface.clientWidth);
      surface.height = renderSize(surface.clientHeight);
    });
    observer.observe(surface);
    return () => observer.disconnect();
  }, []);
  return <canvas ref={canvas} className={`block bg-black ${className ?? ''}`} />;
};
