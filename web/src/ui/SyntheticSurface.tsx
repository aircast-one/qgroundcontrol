import { useEffect, useRef } from 'react';
import { type Json, optBoolean, optString } from '@/bridge/Json';
import { call, callTransferring } from '@/core/client';
import { coerceAtLeast, roundToInt } from '@/kotlin/Std';
import { SyntheticDraw } from '@/ui/SyntheticView';

export const RENDER_SCALE = 0.75;

export const renderSize = (viewPx: number): number => coerceAtLeast(roundToInt(viewPx * RENDER_SCALE), 1);

export const nativeChosen = (view: Json | null): boolean => optBoolean(view, 'native');

export const nativeDraw = (view: Json | null): SyntheticDraw => {
  const draw = view === null ? null : optString(view, 'draw');
  return draw === 'drawn' ? SyntheticDraw.Drawn : draw === 'failed' ? SyntheticDraw.Failed : SyntheticDraw.Loading;
};

const surfaceHandoff = {
  surfaceCreated: (canvas: HTMLCanvasElement, width: number, height: number): Promise<number> => {
    const surface = canvas.transferControlToOffscreen();
    return callTransferring<number>('syntheticAttach', [surface], surface, width, height);
  },
  surfaceChanged: (surface: Promise<number>, width: number, height: number) => void surface.then((key) => call('syntheticResize', key, width, height)),
  surfaceDestroyed: (surface: Promise<number>) => void surface.then((key) => call('syntheticRelease', key)),
};

export const SyntheticSurface = ({ className }: { className?: string }) => {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const view = host.current;
    if (view === null) return undefined;
    const canvas = document.createElement('canvas');
    canvas.className = 'block size-full bg-black';
    view.append(canvas);
    const surface = surfaceHandoff.surfaceCreated(canvas, renderSize(canvas.clientWidth), renderSize(canvas.clientHeight));
    const observer = new ResizeObserver(() => {
      if (canvas.clientWidth <= 0 || canvas.clientHeight <= 0) return;
      surfaceHandoff.surfaceChanged(surface, renderSize(canvas.clientWidth), renderSize(canvas.clientHeight));
    });
    observer.observe(canvas);
    return () => {
      observer.disconnect();
      surfaceHandoff.surfaceDestroyed(surface);
      canvas.remove();
    };
  }, []);
  return <div ref={host} className={className} />;
};
