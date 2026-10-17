import { type ReactNode, useEffect, useRef, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray } from '@/bridge/Json';
import { md } from '@/theme/Colors';

const AXIS_INDICATOR_SIZE = 15;
const IMAGE_MARGIN = 5;
const FRAME_WIDTH = 6;
const COAX_DISTANCE = 0.03;
const SPIN_ARC_DEGREES = 50;
const IMAGE_ASPECT = 2;
const CLOCKWISE = 'rgb(21 158 31 / 0.784)';
const COUNTER_CLOCKWISE = 'rgb(78 195 232 / 0.784)';
const CLOCKWISE_ARROW = 'rgb(21 158 31)';
const COUNTER_CLOCKWISE_ARROW = 'rgb(78 195 232)';
const FRAME_ARROW = 'rgb(255 68 43 / 0.784)';
const FRAME = 'rgb(150 150 150)';

export type Offset = { x: number; y: number };

export type GeometryMotor = { index: number; label: number; x: number; y: number; counterClockwise: boolean };

export type DrawnMotor = { motor: GeometryMotor; center: Offset; textCenter: Offset; coax: boolean };

export type GeometryLayout = {
  origin: Offset;
  rotorDiameter: number;
  fontSize: number;
  extraYMargin: number;
  motors: readonly DrawnMotor[];
};

export const geometryMotors = (geometry: Json | null): readonly GeometryMotor[] =>
  jsonObjects(optJSONArray(geometry, 'motors')).map((it) => ({
    index: optInt(it, 'index'),
    label: optInt(it, 'label'),
    x: optDouble(it, 'x'),
    y: optDouble(it, 'y'),
    counterClockwise: optBoolean(it, 'counterClockwise'),
  }));

export const geometryLayout = (motors: readonly GeometryMotor[], width: number, height: number): GeometryLayout | null => {
  if (motors.length <= 1) return null;
  const xs = motors.map((it) => it.x);
  const ys = motors.map((it) => it.y);
  const [minX, maxX] = [Math.min(...xs), Math.max(...xs)];
  const [minY, maxY] = [Math.min(...ys), Math.max(...ys)];
  if (maxX - minX < 0.0001 || maxY - minY < 0.0001) return null;
  const coax = motors.map((motor, at) => motors.slice(0, at).some((before) => Math.hypot(before.x - motor.x, before.y - motor.y) < COAX_DISTANCE));
  const squeezed = width < height + AXIS_INDICATOR_SIZE * 4;
  const usableWidth = width - 2 * IMAGE_MARGIN - (squeezed ? AXIS_INDICATOR_SIZE : 0);
  const usableHeight = height - 2 * IMAGE_MARGIN - (squeezed ? AXIS_INDICATOR_SIZE : 0);
  const extraOffsetX = squeezed ? AXIS_INDICATOR_SIZE : 0;
  const rotorDiameter = Math.min(usableWidth, usableHeight) * (motors.length <= 6 ? 0.31 : 0.25);
  const fontSize = rotorDiameter * 0.4;
  const extraYMargin = coax.some((it) => it) ? fontSize * 1.1 : 0;
  const scaleX = (usableWidth - rotorDiameter) / (maxY - minY);
  const scaleY = (usableHeight - extraYMargin - rotorDiameter) / (maxX - minX);
  const scale = Math.min(scaleX, scaleY);
  const offsetX = IMAGE_MARGIN + extraOffsetX + usableWidth / 2 - ((maxY + minY) / 2) * scale;
  const offsetY = IMAGE_MARGIN + (usableHeight - extraYMargin) / 2 + ((maxX + minX) / 2) * scale;
  const drawn = motors.map((motor, at): DrawnMotor => {
    const isCoax = coax[at] === true;
    const base = { x: offsetX + motor.y * scale, y: offsetY - motor.x * scale };
    const center = isCoax ? { x: base.x, y: base.y + extraYMargin } : base;
    const textCenter = isCoax ? { x: center.x, y: center.y + rotorDiameter / 2 - extraYMargin / 2 } : center;
    return { motor, center, textCenter, coax: isCoax };
  });
  return { origin: { x: offsetX, y: offsetY }, rotorDiameter, fontSize, extraYMargin, motors: drawn };
};

export const motorAt = (layout: GeometryLayout, point: Offset, highlighted: ReadonlySet<number>): number | null => {
  const hits = layout.motors
    .filter((it) => highlighted.has(it.motor.index))
    .filter((drawn) => {
      const radius = drawn.coax ? layout.fontSize / 2 : layout.rotorDiameter / 2;
      return Math.hypot(drawn.textCenter.x - point.x, drawn.textCenter.y - point.y) < radius;
    });
  return hits.length === 1 ? (hits[0]?.motor.index ?? null) : null;
};

const radians = (degrees: number) => (degrees * Math.PI) / 180;

const drawMotor = (drawn: DrawnMotor, layout: GeometryLayout, ink: string, highlight: boolean, key: number): ReactNode => {
  const fill = drawn.motor.counterClockwise ? COUNTER_CLOCKWISE : CLOCKWISE;
  const arrowColor = drawn.motor.counterClockwise ? COUNTER_CLOCKWISE_ARROW : CLOCKWISE_ARROW;
  const radius = layout.rotorDiameter / 2;
  const { x, y } = drawn.center;
  const offsets = drawn.coax ? [30, 150] : [0, 180];
  const ySign = drawn.motor.counterClockwise ? 1 : -1;
  const sweep = ySign * SPIN_ARC_DEGREES;
  const head = FRAME_WIDTH * 1.25;
  return (
    <g key={key}>
      <circle cx={x} cy={y} r={radius} fill={fill} />
      {highlight ? <circle cx={drawn.textCenter.x} cy={drawn.textCenter.y} r={drawn.coax ? layout.fontSize / 2 : radius} fill={FRAME_ARROW} /> : null}
      <text x={drawn.textCenter.x} y={drawn.textCenter.y} fill={ink} fontSize={layout.fontSize} textAnchor="middle" dominantBaseline="central">
        {drawn.motor.label}
      </text>
      {offsets.map((offset) => {
        const turn = drawn.motor.counterClockwise ? -SPIN_ARC_DEGREES / 2 + offset : SPIN_ARC_DEGREES / 2 + offset;
        const endX = x + radius * Math.cos(radians(sweep));
        const endY = y + radius * Math.sin(radians(sweep));
        return (
          <g key={offset} transform={`rotate(${turn} ${x} ${y})`}>
            <path d={`M ${x + radius} ${y} A ${radius} ${radius} 0 0 ${sweep > 0 ? 1 : 0} ${endX} ${endY}`} fill="none" stroke={arrowColor} strokeWidth={2.5} />
            <path
              d={`M ${x + radius - FRAME_WIDTH / 2} ${y + (ySign * head) / 2} L ${x + radius} ${y - (ySign * head) / 2} L ${x + radius + FRAME_WIDTH / 2} ${y + (ySign * head) / 2} Z`}
              fill={arrowColor}
            />
          </g>
        );
      })}
    </g>
  );
};

const drawAxisIndicator = (origin: Offset, ink: string): ReactNode => {
  const length = AXIS_INDICATOR_SIZE * 2;
  const up = { x: origin.x, y: origin.y - length };
  const right = { x: origin.x + length, y: origin.y };
  return (
    <g>
      <line x1={origin.x} y1={origin.y} x2={up.x} y2={up.y} stroke={ink} strokeWidth={1.5} />
      <line x1={origin.x} y1={origin.y} x2={right.x} y2={right.y} stroke={ink} strokeWidth={1.5} />
      <text x={up.x} y={up.y} fill={ink} fontSize={AXIS_INDICATOR_SIZE} textAnchor="middle" dominantBaseline="text-after-edge">
        x
      </text>
      <text x={right.x} y={right.y} fill={ink} fontSize={AXIS_INDICATOR_SIZE} dominantBaseline="central" style={{ whiteSpace: 'pre' }}>
        {' y'}
      </text>
    </g>
  );
};

export const GeometryImage = ({
  motors,
  className,
  highlighted = new Set<number>(),
  onMotor = () => undefined,
}: {
  motors: readonly GeometryMotor[];
  className?: string;
  highlighted?: ReadonlySet<number>;
  onMotor?: (index: number) => void;
}) => {
  const host = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const element = host.current;
    if (element === null) return undefined;
    const observer = new ResizeObserver(([entry]) => setWidth(entry?.contentRect.width ?? 0));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const height = width / IMAGE_ASPECT;
  const ink = md('onSurface');
  const layout = width > 0 ? geometryLayout(motors, width, height) : null;
  const tap = (clientX: number, clientY: number) => {
    const element = host.current;
    if (element === null || layout === null) return;
    const bounds = element.getBoundingClientRect();
    const hit = motorAt(layout, { x: clientX - bounds.left, y: clientY - bounds.top }, highlighted);
    if (hit !== null) onMotor(hit);
  };
  return (
    <div ref={host} className={`w-full ${className ?? ''}`} style={{ aspectRatio: IMAGE_ASPECT }} onClick={(event) => tap(event.clientX, event.clientY)}>
      {layout !== null ? (
        <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className="block select-none">
          {layout.motors
            .filter((it) => !it.coax)
            .map((it) => (
              <line key={it.motor.index} x1={layout.origin.x} y1={layout.origin.y} x2={it.center.x} y2={it.center.y} stroke={FRAME} strokeWidth={FRAME_WIDTH} />
            ))}
          <rect
            x={layout.origin.x - (layout.rotorDiameter * 0.8) / 2}
            y={layout.origin.y - (layout.rotorDiameter * 0.8) / 2}
            width={layout.rotorDiameter * 0.8}
            height={layout.rotorDiameter * 0.8}
            rx={FRAME_WIDTH}
            fill={FRAME}
          />
          <path
            d={`M ${layout.origin.x - layout.rotorDiameter / 8} ${layout.origin.y + layout.rotorDiameter / 4} L ${layout.origin.x} ${layout.origin.y - layout.rotorDiameter / 4} L ${layout.origin.x + layout.rotorDiameter / 8} ${layout.origin.y + layout.rotorDiameter / 4} Z`}
            fill={FRAME_ARROW}
          />
          {drawAxisIndicator({ x: AXIS_INDICATOR_SIZE / 2, y: height - AXIS_INDICATOR_SIZE / 2 }, ink)}
          {[...layout.motors.filter((it) => it.coax), ...layout.motors.filter((it) => !it.coax)].map((it, at) => drawMotor(it, layout, ink, highlighted.has(it.motor.index), at))}
        </svg>
      ) : null}
    </div>
  );
};
