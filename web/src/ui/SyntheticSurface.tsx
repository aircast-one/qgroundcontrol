import { useEffect, useRef } from 'react';
import { type Json, optBoolean, optString } from '@/bridge/Json';
import { Channel, invoke } from '@tauri-apps/api/core';
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

const desktop: boolean = import.meta.env.DESKTOP === true;

const paintJpeg = (canvas: HTMLCanvasElement) => {
  const context = canvas.getContext('2d');
  return (jpeg: ArrayBuffer) =>
    void createImageBitmap(new Blob([jpeg], { type: 'image/jpeg' })).then((picture) => {
      if (canvas.width !== picture.width) canvas.width = picture.width;
      if (canvas.height !== picture.height) canvas.height = picture.height;
      context?.drawImage(picture, 0, 0);
      picture.close();
    });
};

const nativeHandoff = {
  surfaceCreated: (canvas: HTMLCanvasElement, width: number, height: number, orbit: boolean): Promise<number> =>
    invoke<number>('synthetic_open', { width, height, orbit, frames: new Channel<ArrayBuffer>(paintJpeg(canvas)) }),
  surfaceChanged: (surface: Promise<number>, width: number, height: number) => void surface.then((key) => invoke('synthetic_resize', { key, width, height })),
  surfaceDestroyed: (surface: Promise<number>) => void surface.then((key) => invoke('synthetic_close', { key })),
};

const workerHandoff = {
  surfaceCreated: (canvas: HTMLCanvasElement, width: number, height: number, orbit: boolean): Promise<number> => {
    const surface = canvas.transferControlToOffscreen();
    return callTransferring<number>('syntheticAttach', [surface], surface, width, height, orbit);
  },
  surfaceChanged: (surface: Promise<number>, width: number, height: number) => void surface.then((key) => call('syntheticResize', key, width, height)),
  surfaceDestroyed: (surface: Promise<number>) => void surface.then((key) => call('syntheticRelease', key)),
};

const surfaceHandoff = desktop ? nativeHandoff : workerHandoff;

export const SyntheticSurface = ({ className, orbit = false }: { className?: string; orbit?: boolean }) => {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const view = host.current;
    if (view === null) return undefined;
    const canvas = document.createElement('canvas');
    canvas.className = 'block size-full bg-black';
    view.append(canvas);
    const surface = surfaceHandoff.surfaceCreated(canvas, renderSize(canvas.clientWidth), renderSize(canvas.clientHeight), orbit);
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
  }, [orbit]);
  return <div ref={host} className={className} />;
};
