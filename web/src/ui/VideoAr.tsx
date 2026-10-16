import { useQgcDerived } from '@/bridge/QgcState';
import { type Json, arrayJSONObject, has, jsonObjects, optBoolean, optDouble, optJSONArray, optJSONObject, optString } from '@/bridge/Json';
import { Color, argb, withAlpha } from '@/theme/Colors';
import { type Offset, useElementSize } from '@/ui/FlyShell';

export const VIDEO_AR_VIEW = 'view.videoAr';
const ROUTE_COLOUR = argb(0xffffe9a6);
const HOME_COLOUR = argb(0xff4caf50);
const EDGE_COLOUR = withAlpha(Color.Black, 0.6);
const ROUTE_WIDTH = 3;
const EDGE_WIDTH = 2;
const MARK_RADIUS = 7;
const LABEL_GAP = 4;

export type ArSpot = { x: number; up: number };

export type ArMark = { at: ArSpot; label: string; text: string };

export type ArReading = { home: ArMark | null; waypoints: readonly ArMark[]; route: readonly (readonly [ArSpot, ArSpot])[] };

const spot = (json: Json | null): ArSpot | null => (json !== null && has(json, 'x') && has(json, 'up') ? { x: optDouble(json, 'x'), up: optDouble(json, 'up') } : null);

const mark = (json: Json | null): ArMark | null => {
  const at = spot(json);
  return at === null ? null : { at, label: optString(json, 'label'), text: optString(json, 'text') };
};

export const arReading = (view: Json | null): ArReading | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const route = optJSONArray(view, 'route');
  return {
    home: mark(optJSONObject(view, 'home')),
    waypoints: jsonObjects(optJSONArray(view, 'waypoints')).flatMap((it) => mark(it) ?? []),
    route: (route ?? []).flatMap((_, index) => {
      const leg = arrayJSONObject(route, index);
      const start = spot(optJSONObject(leg, 'from'));
      const end = spot(optJSONObject(leg, 'to'));
      return start !== null && end !== null ? [[start, end] as const] : [];
    }),
  };
};

export const arOffset = (spot: ArSpot, width: number, height: number): Offset => ({ x: spot.x * width, y: height / 2 - spot.up * width });

const Marked = ({ mark, fill, width, height }: { mark: ArMark; fill: string; width: number; height: number }) => {
  const at = arOffset(mark.at, width, height);
  const caption = [mark.label, mark.text].filter((it) => it.trim() !== '').join(' · ');
  return (
    <>
      <circle cx={at.x} cy={at.y} r={MARK_RADIUS + EDGE_WIDTH} fill={EDGE_COLOUR} />
      <circle cx={at.x} cy={at.y} r={MARK_RADIUS} fill={fill} />
      <text
        x={at.x}
        y={at.y - MARK_RADIUS - LABEL_GAP}
        textAnchor="middle"
        dominantBaseline="text-after-edge"
        fill={Color.White}
        className="text-label-medium"
        style={{ filter: `drop-shadow(0 0 2px ${Color.Black})` }}
      >
        {caption}
      </text>
    </>
  );
};

export const VideoArOverlay = ({ className }: { className?: string }) => {
  const reading = useQgcDerived(VIDEO_AR_VIEW, arReading);
  const { ref, size } = useElementSize();
  if (reading === null) return null;
  return (
    <div ref={ref} className={`pointer-events-none overflow-hidden ${className ?? ''}`}>
      <svg width={size.width} height={size.height} className="block overflow-hidden">
        {reading.route.map(([from, to], index) => {
          const start = arOffset(from, size.width, size.height);
          const end = arOffset(to, size.width, size.height);
          return (
            <g key={index} strokeLinecap="round">
              <line x1={start.x} y1={start.y} x2={end.x} y2={end.y} stroke={EDGE_COLOUR} strokeWidth={ROUTE_WIDTH + EDGE_WIDTH * 2} />
              <line x1={start.x} y1={start.y} x2={end.x} y2={end.y} stroke={ROUTE_COLOUR} strokeWidth={ROUTE_WIDTH} />
            </g>
          );
        })}
        {reading.waypoints.map((it, index) => (
          <Marked key={index} mark={it} fill={Color.White} width={size.width} height={size.height} />
        ))}
        {reading.home !== null ? <Marked mark={reading.home} fill={HOME_COLOUR} width={size.width} height={size.height} /> : null}
      </svg>
    </div>
  );
};
