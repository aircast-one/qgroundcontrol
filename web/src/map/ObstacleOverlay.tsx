import { useEffect, useMemo, useRef, useState } from 'react';
import { type Json, optArrayText, optDouble, optJSONArray, optJSONObject } from '@/bridge/Json';
import { useMapPath } from './MapBridge';

export const OBSTACLE_VIEW = 'view.obstacle';
const SEGMENTS = 16;
const LEVEL_METRES = 10.0;
const SEGMENT_GAP_RAD = 0.03;
const TEXT_STEP_METRES = 2.0;
const MAP_TEXT = 22;
const VIDEO_TEXT = 10;
export const TRUE_SCALE_PROBE = 100;
export const TRUE_SCALE_BELOW_METRES = 4.0;

export type Offset = { readonly x: number; readonly y: number };

export type DrawScope = { readonly context: CanvasRenderingContext2D; readonly width: number; readonly height: number };

export type ObstacleOverlay = {
  readonly ranges: readonly number[];
  readonly texts: readonly string[];
  readonly increment: number;
  readonly offset: number;
  readonly maxMetres: number;
};

const positive = (value: number): number | null => (Number.isFinite(value) && value > 0.0 ? value : null);

export const obstacleOverlay = (view: Json | null): ObstacleOverlay | null => {
  const drawn = optJSONObject(view, 'overlay');
  const ranges = optJSONArray(drawn, 'ranges');
  if (drawn === null || ranges === null) return null;
  const texts = optJSONArray(drawn, 'texts');
  const increment = positive(optDouble(view, 'ringIncrement', NaN));
  const maxMetres = positive(optDouble(drawn, 'maxMetres', NaN));
  if (increment === null || maxMetres === null) return null;
  const offset = optDouble(view, 'ringOffset', 0.0);
  const overlay: ObstacleOverlay = {
    ranges: ranges.map((range) => (typeof range === 'number' ? range : typeof range === 'string' && range.trim() !== '' ? Number(range) : NaN)),
    texts: ranges.map((_, index) => optArrayText(texts, index)),
    increment,
    offset: Number.isFinite(offset) ? offset : 0.0,
    maxMetres,
  };
  return overlay.ranges.length > 0 ? overlay : null;
};

const floorMod = (value: number, divisor: number): number => ((value % divisor) + divisor) % divisor;

export const rangeIndex = (degrees: number, overlay: ObstacleOverlay, heading: number): number => {
  const len = overlay.ranges.length;
  const wrapped = floorMod(degrees - overlay.offset - heading, 360.0);
  return floorMod(len + Math.trunc(Math.ceil(wrapped / overlay.increment)), len);
};

export type GradientStop = { readonly at: number; readonly colour: string };

const rgba = (red: number, green: number, blue: number, alpha: number): string => `rgba(${Math.round(red * 255)}, ${Math.round(green * 255)}, ${Math.round(blue * 255)}, ${alpha})`;

const MAP_STOPS: readonly GradientStop[] = [
  { at: 0, colour: rgba(1, 0, 0, 1) },
  { at: 0.1, colour: rgba(1, 0, 0, 0.7) },
  { at: 0.5, colour: rgba(1, 0.64, 0, 0.7) },
  { at: 0.65, colour: rgba(1, 0.64, 0, 0.3) },
  { at: 0.95, colour: rgba(0, 1, 0, 0.3) },
  { at: 1, colour: rgba(0, 1, 0, 0) },
];

const VIDEO_STOPS: readonly GradientStop[] = [
  { at: 0, colour: rgba(1, 0, 0, 0.9) },
  { at: 0.1, colour: rgba(1, 0, 0, 0.3) },
  { at: 0.5, colour: rgba(1, 0.64, 0, 0.3) },
  { at: 0.65, colour: rgba(1, 0.64, 0, 0.2) },
  { at: 0.95, colour: rgba(0, 1, 0, 0.1) },
  { at: 1, colour: rgba(0, 1, 0, 0) },
];

export const twoCircleStops = (from: number, to: number, stops: readonly GradientStop[]): readonly GradientStop[] =>
  stops.map((it) => ({ at: (from + it.at * (to - from)) / to, colour: it.colour }));

const radial = (context: CanvasRenderingContext2D, centre: Offset, from: number, to: number, stops: readonly GradientStop[]): CanvasGradient =>
  twoCircleStops(from, to, stops).reduce((gradient, stop) => {
    gradient.addColorStop(Math.min(1, Math.max(0, stop.at)), stop.colour);
    return gradient;
  }, context.createRadialGradient(centre.x, centre.y, 0, centre.x, centre.y, to));

export type OverlayPoint = { readonly outer: Offset; readonly inner: Offset; readonly range: number; readonly index: number };

export type MapOverlayShape = { readonly gradientFrom: number; readonly gradientTo: number; readonly points: readonly OverlayPoint[] };

export const mapOverlayShape = (
  overlay: ObstacleOverlay,
  centre: Offset,
  heightPx: number,
  metresInProbe: number,
  probePx: number,
  heading: number,
  mapBearing: number,
): MapOverlayShape => {
  const maxRadius = (0.9 * heightPx) / 2;
  const minRadius = maxRadius * 0.2;
  const pixelsPerMetre = probePx / metresInProbe;
  const trueScale = metresInProbe < TRUE_SCALE_BELOW_METRES;
  const gradientFrom = trueScale ? 0 : minRadius;
  const gradientTo = trueScale ? overlay.maxMetres * pixelsPerMetre : maxRadius;
  const metresToPixels = trueScale ? pixelsPerMetre : (maxRadius - minRadius) / overlay.maxMetres;
  const height = minRadius / 8;
  const points = overlay.ranges.map((_, i) => {
    const degrees = i * overlay.increment;
    const rad = ((degrees - mapBearing) * Math.PI) / 180.0;
    const index = rangeIndex(degrees, overlay, heading);
    const metres = overlay.ranges[index] ?? NaN;
    const pixels = gradientFrom + metres * metresToPixels;
    return {
      outer: { x: centre.x + pixels * Math.sin(rad), y: centre.y - pixels * Math.cos(rad) },
      inner: { x: centre.x + (pixels - height) * Math.sin(rad), y: centre.y - (pixels - height) * Math.cos(rad) },
      range: metres,
      index,
    };
  });
  return { gradientFrom, gradientTo, points };
};

export const mapOverlayLabels = (overlay: ObstacleOverlay, points: readonly OverlayPoint[]): readonly OverlayPoint[] =>
  points
    .flatMap((point, i) => {
      if (i % 3 !== 0) return [];
      const next = points[(i + 1) % points.length];
      return [next !== undefined && next.range < point.range ? next : point];
    })
    .reduce<{ shown: readonly OverlayPoint[]; previous: number }>(
      ({ shown, previous }, point) =>
        point.range < overlay.maxMetres && Math.abs(point.range - previous) > TEXT_STEP_METRES ? { shown: [...shown, point], previous: point.range } : { shown, previous },
      { shown: [], previous: -1.0 },
    ).shown;

export type VideoSegment = { readonly from: number; readonly to: number; readonly radFrom: number; readonly radTo: number; readonly label: string | null };

export const videoOverlaySegments = (overlay: ObstacleOverlay, heightPx: number, showText: boolean): readonly [readonly [number, number], readonly VideoSegment[]] | null => {
  const maxRadius = (0.9 * heightPx) / 2;
  const segmentHeight = (maxRadius * 0.2) / 8;
  const levels = overlay.maxMetres / LEVEL_METRES;
  const gradientFrom = maxRadius - segmentHeight * levels * 2;
  if (maxRadius <= 0 || gradientFrom < 0) return null;
  const len = overlay.ranges.length;
  const step = 360.0 / SEGMENTS;
  const rangeAt = (index: number): number => overlay.ranges[index] ?? NaN;
  const segments = Array.from({ length: SEGMENTS }, (_, s) => s).flatMap((s) => {
    const degrees = s * step;
    const first = rangeIndex(degrees, overlay, 0.0);
    const next = rangeIndex(degrees + step, overlay, 0.0);
    const end = first < next ? next : len + next;
    const nearest = Array.from({ length: Math.max(0, end - first) }, (_, offset) => (first + offset) % len)
      .filter((it) => rangeAt(it) < overlay.maxMetres)
      .reduce<number | null>((best, it) => (best === null || rangeAt(it) < rangeAt(best) ? it : best), null);
    const rangeMin = nearest === null ? overlay.maxMetres : rangeAt(nearest);
    const radFrom = (degrees * Math.PI) / 180.0;
    const radTo = radFrom + (step * Math.PI) / 180.0 - SEGMENT_GAP_RAD;
    const drawn = Array.from({ length: Math.ceil(levels) }, (_, ii) => {
      const from = maxRadius - ii * segmentHeight * 2;
      const rangeInLevel = overlay.maxMetres - (ii + 1) * LEVEL_METRES;
      const reached = rangeMin > rangeInLevel || ii >= levels - 1;
      const range = reached ? rangeMin : overlay.maxMetres;
      const label = nearest !== null && showText && reached && range < overlay.maxMetres ? (overlay.texts[nearest] ?? '') : null;
      return { segment: { from, to: from - segmentHeight, radFrom, radTo, label }, last: range === rangeMin };
    });
    const hit = drawn.findIndex((it) => it.last);
    return drawn.slice(0, hit < 0 ? drawn.length : hit + 1).map((it) => it.segment);
  });
  const negative = segments.findIndex((it) => it.to < 0);
  return [[gradientFrom, maxRadius], segments.slice(0, negative < 0 ? segments.length : negative)];
};

const outlinedText = (context: CanvasRenderingContext2D, text: string, at: Offset, textSize: number, fill: string, strokeWidth: number) => {
  context.font = `bold ${textSize}px sans-serif`;
  context.textBaseline = 'alphabetic';
  context.textAlign = 'left';
  context.lineWidth = strokeWidth;
  context.strokeStyle = 'rgba(0, 0, 0, 0.8)';
  context.strokeText(text, at.x, at.y);
  context.fillStyle = fill;
  context.fillText(text, at.x, at.y);
};

export const drawMapObstacleOverlay = (scope: DrawScope, overlay: ObstacleOverlay, shape: MapOverlayShape, centre: Offset, showText: boolean): void => {
  const points = shape.points;
  if (points.length === 0 || shape.gradientTo <= 0) return;
  const { context } = scope;
  const brush = radial(context, centre, shape.gradientFrom, shape.gradientTo, MAP_STOPS);
  const at = (index: number): OverlayPoint => points[index % points.length] as OverlayPoint;
  const wedges = points.flatMap((_, i) => (i % 3 === 0 ? [i] : [])).map((i) => {
    const [a, b, c, d] = [at(i), at(i + 1), at(i + 2), at(i + 3)];
    const wedge = new Path2D();
    wedge.moveTo(a.inner.x, a.inner.y);
    wedge.lineTo(a.outer.x, a.outer.y);
    wedge.bezierCurveTo(b.outer.x, b.outer.y, c.outer.x, c.outer.y, d.outer.x, d.outer.y);
    wedge.lineTo(d.inner.x, d.inner.y);
    wedge.bezierCurveTo(d.inner.x, d.inner.y, c.inner.x, c.inner.y, b.inner.x, b.inner.y);
    wedge.closePath();
    return wedge;
  });
  context.fillStyle = brush;
  wedges.forEach((wedge) => context.fill(wedge));
  if (showText) {
    mapOverlayLabels(overlay, points).forEach((it) => outlinedText(context, overlay.texts[it.index] ?? '', it.inner, MAP_TEXT, 'rgba(255, 255, 255, 0.9)', 2));
  }
};

export const drawVideoObstacleOverlay = (scope: DrawScope, overlay: ObstacleOverlay, showText: boolean): void => {
  const placed = videoOverlaySegments(overlay, scope.height, showText);
  if (placed === null) return;
  const [[radiusFrom, radiusTo], segments] = placed;
  const { context } = scope;
  const centre = { x: scope.width / 2, y: scope.height / 2 };
  const brush = radial(context, centre, radiusFrom, radiusTo, VIDEO_STOPS);
  segments.forEach((segment) => {
    const radFrom = segment.radFrom - Math.PI / 2;
    const radTo = segment.radTo - Math.PI / 2;
    const top = { x: centre.x + segment.from * Math.cos(radFrom), y: centre.y + segment.from * Math.sin(radFrom) };
    const topEnd = { x: centre.x + segment.from * Math.cos(radTo), y: centre.y + segment.from * Math.sin(radTo) };
    const ring = new Path2D();
    ring.moveTo(top.x, top.y);
    ring.arc(centre.x, centre.y, segment.from, radFrom, radTo, false);
    ring.arc(centre.x, centre.y, segment.to, radTo, radFrom, true);
    ring.lineTo(top.x, top.y);
    ring.closePath();
    context.save();
    context.translate(centre.x, 0);
    context.scale(2, 1);
    context.translate(-centre.x, 0);
    context.fillStyle = brush;
    context.fill(ring);
    context.restore();
    if (segment.label !== null) {
      const middle = { x: (top.x + topEnd.x) / 2, y: (top.y + topEnd.y) / 2 };
      outlinedText(context, segment.label, { x: 2 * middle.x - centre.x, y: middle.y }, VIDEO_TEXT * 2, 'rgba(255, 255, 255, 0.8)', 4);
    }
  });
};

export const Canvas = ({ className, onDraw }: { className?: string; onDraw: (scope: DrawScope) => void }) => {
  const ref = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const canvas = ref.current;
    if (canvas === null) return undefined;
    const observer = new ResizeObserver(([entry]) => entry && setSize({ width: entry.contentRect.width, height: entry.contentRect.height }));
    observer.observe(canvas);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    const canvas = ref.current;
    const context = canvas?.getContext('2d') ?? null;
    if (canvas === null || context === null) return;
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.round(size.width * ratio);
    canvas.height = Math.round(size.height * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, size.width, size.height);
    onDraw({ context, width: size.width, height: size.height });
  });
  return <canvas ref={ref} className={`pointer-events-none ${className ?? ''}`} />;
};

export const ObstacleVideoOverlay = ({ className, showText }: { className?: string; showText: boolean }) => {
  const json = useMapPath(OBSTACLE_VIEW);
  const overlay = useMemo(() => obstacleOverlay(json), [json]);
  return overlay === null ? null : <Canvas className={className} onDraw={(scope) => drawVideoObstacleOverlay(scope, overlay, showText)} />;
};
