import { useEffect, useRef } from 'react';
import { type Json, optBoolean, optString } from '@/bridge/Json';
import { Channel, invoke } from '@tauri-apps/api/core';
import { AppCommands } from '@/bridge/BridgeCommands';
import { call, callTransferring } from '@/core/client';
import { onMac } from '@/kotlin/Keys';
import { coerceAtLeast, coerceIn, roundToInt } from '@/kotlin/Std';
import { SyntheticDraw } from '@/ui/SyntheticView';

export const RENDER_SCALE = 0.75;

export const renderSize = (viewPx: number): number => coerceAtLeast(roundToInt(viewPx * RENDER_SCALE), 1);

export const nativeChosen = (view: Json | null): boolean => optBoolean(view, 'native');

export const nativeDraw = (view: Json | null): SyntheticDraw => {
  const draw = view === null ? null : optString(view, 'draw');
  return draw === 'drawn' ? SyntheticDraw.Drawn : draw === 'failed' ? SyntheticDraw.Failed : SyntheticDraw.Loading;
};

export type SurfacePath = 'layer' | 'jpeg' | 'worker';

export const surfacePath = (desktop: boolean, mac: boolean): SurfacePath => (!desktop ? 'worker' : mac ? 'layer' : 'jpeg');

export type Area = { readonly left: number; readonly top: number; readonly width: number; readonly height: number };

export type Placement = {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly scale: number;
  readonly pixelWidth: number;
  readonly pixelHeight: number;
  readonly viewportHeight: number;
  readonly shown: boolean;
};

export const placementOf = (rect: Area, layout: { readonly width: number; readonly height: number }, scale: number, viewportHeight: number, visible: boolean): Placement => ({
  x: rect.left,
  y: rect.top,
  width: rect.width,
  height: rect.height,
  scale,
  pixelWidth: roundToInt(layout.width * scale),
  pixelHeight: roundToInt(layout.height * scale),
  viewportHeight,
  shown: visible && rect.width > 0 && rect.height > 0 && layout.width > 0 && layout.height > 0,
});

export const samePlacement = (a: Placement, b: Placement | null): boolean => b !== null && (Object.keys(a) as (keyof Placement)[]).every((key) => a[key] === b[key]);

const edges = (values: readonly number[]): readonly (readonly [number, number])[] => {
  const sorted = [...new Set(values)].toSorted((a, b) => a - b);
  return sorted.slice(1).map((end, index) => [sorted[index] ?? 0, end] as const);
};

const inside = (area: Area, x: number, y: number): boolean => x > area.left && x < area.left + area.width && y > area.top && y < area.top + area.height;

export const paintedCells = (holes: readonly Area[], box: Area): readonly Area[] => {
  const cut = holes.map((hole) => {
    const left = coerceIn(hole.left - box.left, 0, box.width);
    const top = coerceIn(hole.top - box.top, 0, box.height);
    return { left, top, width: coerceIn(hole.left + hole.width - box.left, 0, box.width) - left, height: coerceIn(hole.top + hole.height - box.top, 0, box.height) - top };
  });
  const columns = edges([0, box.width, ...cut.flatMap((hole) => [hole.left, hole.left + hole.width])]);
  const rows = edges([0, box.height, ...cut.flatMap((hole) => [hole.top, hole.top + hole.height])]);
  return rows
    .flatMap(([top, bottom]) => columns.map(([left, right]) => ({ left, top, width: right - left, height: bottom - top })))
    .filter((cell) => !cut.some((hole) => inside(hole, cell.left + cell.width / 2, cell.top + cell.height / 2)));
};

export const holeBackground = (color: string, holes: readonly Area[], box: Area): string => {
  const cells = paintedCells(holes, box);
  return cells.length === 0 ? 'none' : cells.map((cell) => `linear-gradient(${color}, ${color}) ${cell.left}px ${cell.top}px / ${cell.width}px ${cell.height}px no-repeat border-box`).join(', ');
};

const EDGE_PX = 0.5;

export const covers = (hole: Area, box: Area): boolean =>
  box.width > 0 &&
  box.height > 0 &&
  box.left >= hole.left - EDGE_PX &&
  box.top >= hole.top - EDGE_PX &&
  box.left + box.width <= hole.left + hole.width + EDGE_PX &&
  box.top + box.height <= hole.top + hole.height + EDGE_PX;

export const overlap = (a: Area, b: Area): Area => {
  const left = Math.max(a.left, b.left);
  const top = Math.max(a.top, b.top);
  return { left, top, width: Math.max(0, Math.min(a.left + a.width, b.left + b.width) - left), height: Math.max(0, Math.min(a.top + a.height, b.top + b.height) - top) };
};

export const stackOrder = (position: string, zIndex: string): number => (zIndex !== 'auto' ? Number(zIndex) : position !== 'static' ? 0 : -0.5);

export const paintedBelow = (sibling: number, path: number, siblingFirst: boolean): boolean => sibling < path || (sibling === path && siblingFirst);

const SYNTHETIC_LOG = 'desktop.synthetic';


const logged = (event: string) => (error: unknown) => void AppCommands.recordLog(SYNTHETIC_LOG, `event=${event} reason="${String(error)}"`);

const paintJpeg = (canvas: HTMLCanvasElement) => {
  const context = canvas.getContext('2d');
  const order = { arrived: 0, painted: 0 };
  return (jpeg: ArrayBuffer) => {
    order.arrived += 1;
    const sequence = order.arrived;
    void createImageBitmap(new Blob([jpeg], { type: 'image/jpeg' })).then((picture) => {
      if (sequence > order.painted) {
        order.painted = sequence;
        if (canvas.width !== picture.width) canvas.width = picture.width;
        if (canvas.height !== picture.height) canvas.height = picture.height;
        context?.drawImage(picture, 0, 0);
      }
      picture.close();
    }, logged('frame_undecodable'));
  };
};

type Handoff = {
  surfaceCreated: (canvas: HTMLCanvasElement, width: number, height: number, orbit: boolean) => Promise<number>;
  surfaceChanged: (surface: Promise<number>, width: number, height: number) => void;
  surfaceDestroyed: (surface: Promise<number>) => void;
};

const jpegHandoff: Handoff = {
  surfaceCreated: (canvas, width, height, orbit) => {
    const opened = invoke<number>('synthetic_open', { width, height, orbit, frames: new Channel<ArrayBuffer>(paintJpeg(canvas)) });
    opened.catch(logged('open_refused'));
    return opened;
  },
  surfaceChanged: (surface, width, height) => void surface.then((key) => invoke('synthetic_resize', { key, width, height })).catch(logged('resize_refused')),
  surfaceDestroyed: (surface) => void surface.then((key) => invoke('synthetic_close', { key })).catch(logged('close_refused')),
};

const workerHandoff: Handoff = {
  surfaceCreated: (canvas, width, height, orbit) => {
    const surface = canvas.transferControlToOffscreen();
    return callTransferring<number>('syntheticAttach', [surface], surface, width, height, orbit);
  },
  surfaceChanged: (surface, width, height) => void surface.then((key) => call('syntheticResize', key, width, height)),
  surfaceDestroyed: (surface) => void surface.then((key) => call('syntheticRelease', key)),
};

const showFrames = (canvas: HTMLCanvasElement, orbit: boolean, handoff: Handoff): (() => void) => {
  canvas.className = 'block size-full bg-black';
  const surface = handoff.surfaceCreated(canvas, renderSize(canvas.clientWidth), renderSize(canvas.clientHeight), orbit);
  const observer = new ResizeObserver(() => {
    if (canvas.clientWidth <= 0 || canvas.clientHeight <= 0) return;
    handoff.surfaceChanged(surface, renderSize(canvas.clientWidth), renderSize(canvas.clientHeight));
  });
  observer.observe(canvas);
  return () => {
    observer.disconnect();
    handoff.surfaceDestroyed(surface);
  };
};

const BACKGROUND = ['background-image', 'background-position', 'background-size', 'background-repeat', 'background-origin', 'background-clip', 'background-color'];

type Kept = { readonly element: HTMLElement; readonly property: string; readonly value: string; readonly priority: string };

const kept = (element: HTMLElement, properties: readonly string[]): readonly Kept[] =>
  properties.map((property) => ({ element, property, value: element.style.getPropertyValue(property), priority: element.style.getPropertyPriority(property) }));

const restore = (all: readonly Kept[]) =>
  all.forEach(({ element, property, value, priority }) => (value === '' ? element.style.removeProperty(property) : element.style.setProperty(property, value, priority)));

const ancestors = (element: Element): readonly HTMLElement[] => (element.parentElement === null ? [] : [element.parentElement, ...ancestors(element.parentElement)]);

const stackOf = (element: Element): number => {
  const style = getComputedStyle(element);
  return stackOrder(style.position, style.zIndex);
};

const siblingsBelow = (node: HTMLElement, parent: HTMLElement): readonly HTMLElement[] => {
  const siblings = [...parent.children].filter((child): child is HTMLElement => child instanceof HTMLElement);
  const at = siblings.indexOf(node);
  return siblings.filter((sibling, index) => sibling !== node && paintedBelow(stackOf(sibling), stackOf(node), index < at));
};

const clipsOverflow = (element: Element): boolean => {
  const style = getComputedStyle(element);
  return style.overflowX !== 'visible' || style.overflowY !== 'visible';
};

const shownWithin = (parent: HTMLElement): Area =>
  [parent, ...ancestors(parent)].filter(clipsOverflow).reduce<Area>((shown, clip) => overlap(shown, clip.getBoundingClientRect()), { left: 0, top: 0, width: window.innerWidth, height: window.innerHeight });

const PAINTS_NOTHING = 'rgba(0, 0, 0, 0)';

const coveredBelow = (canvas: HTMLCanvasElement, hole: Area): readonly HTMLElement[] =>
  [canvas, ...ancestors(canvas)].flatMap((node) => {
    const parent = node.parentElement;
    if (parent === null) return [];
    const shown = shownWithin(parent);
    return siblingsBelow(node, parent).filter((sibling) => covers(hole, overlap(sibling.getBoundingClientRect(), shown)));
  });

const clearBeneath = (canvases: readonly HTMLCanvasElement[]): readonly Kept[] => {
  const holes = canvases.map((canvas) => canvas.getBoundingClientRect());
  const painted = [...new Set(canvases.flatMap((canvas) => ancestors(canvas)))].filter((element) => getComputedStyle(element).backgroundColor !== PAINTS_NOTHING);
  const hidden = [...new Set(canvases.flatMap((canvas, index) => coveredBelow(canvas, holes[index] ?? canvas.getBoundingClientRect())))].filter(
    (element) => !canvases.some((canvas) => element.contains(canvas)),
  );
  const saved = [...painted.flatMap((element) => kept(element, BACKGROUND)), ...hidden.flatMap((element) => kept(element, ['visibility']))];
  const colors = painted.map((element) => getComputedStyle(element).backgroundColor);
  painted.forEach((element, index) => element.style.setProperty('background', holeBackground(colors[index] ?? PAINTS_NOTHING, holes, element.getBoundingClientRect())));
  hidden.forEach((element) => element.style.setProperty('visibility', 'hidden'));
  return saved;
};

const punched = { shown: new Set<HTMLCanvasElement>(), cleared: [] as readonly Kept[] };

const punch = (canvas: HTMLCanvasElement, shown: boolean) => {
  restore(punched.cleared);
  if (shown) punched.shown.add(canvas);
  else punched.shown.delete(canvas);
  punched.cleared = punched.shown.size === 0 ? [] : clearBeneath([...punched.shown]);
};

const placementNow = (canvas: HTMLCanvasElement): Placement =>
  placementOf(
    canvas.getBoundingClientRect(),
    { width: canvas.clientWidth, height: canvas.clientHeight },
    window.devicePixelRatio,
    window.innerHeight,
    canvas.isConnected && canvas.checkVisibility({ visibilityProperty: true, opacityProperty: true }),
  );

const showLayer = (canvas: HTMLCanvasElement, orbit: boolean): (() => void) => {
  canvas.className = 'block size-full';
  const state = { key: null as number | null, sent: null as Placement | null, sending: false, frame: 0, closed: false, fallback: null as (() => void) | null };
  const place = (key: number) => {
    const now = placementNow(canvas);
    if (!state.sending && !samePlacement(now, state.sent)) {
      punch(canvas, now.shown);
      state.sent = now;
      state.sending = true;
      void invoke('synthetic_layer_place', { key, placement: now })
        .catch(logged('place_refused'))
        .finally(() => {
          state.sending = false;
        });
    }
    state.frame = requestAnimationFrame(() => place(key));
  };
  const first = placementNow(canvas);
  void invoke<number>('synthetic_layer_open', { orbit, placement: first }).then(
    (key) => {
      if (state.closed) {
        void invoke('synthetic_layer_close', { key }).catch(logged('close_refused'));
        return;
      }
      state.key = key;
      state.sent = first;
      punch(canvas, first.shown);
      state.frame = requestAnimationFrame(() => place(key));
    },
    (error: unknown) => {
      logged('layer_refused')(error);
      if (!state.closed) state.fallback = showFrames(canvas, orbit, jpegHandoff);
    },
  );
  return () => {
    state.closed = true;
    cancelAnimationFrame(state.frame);
    punch(canvas, false);
    state.fallback?.();
    if (state.key !== null) void invoke('synthetic_layer_close', { key: state.key }).catch(logged('close_refused'));
  };
};

export const SyntheticSurface = ({ className, orbit = false }: { className?: string; orbit?: boolean }) => {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const view = host.current;
    if (view === null) return undefined;
    const canvas = document.createElement('canvas');
    view.append(canvas);
    const path = surfacePath(__DESKTOP__, onMac());
    const stop = path === 'layer' ? showLayer(canvas, orbit) : showFrames(canvas, orbit, path === 'jpeg' ? jpegHandoff : workerHandoff);
    return () => {
      stop();
      canvas.remove();
    };
  }, [orbit]);
  return <div ref={host} className={className} />;
};
