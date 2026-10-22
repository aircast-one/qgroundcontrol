import { useQgcDerived } from '@/bridge/QgcState';
import { type Json, has, jsonObjects, optBoolean, optDouble, optJSONArray, optJSONObject, optString } from '@/bridge/Json';
import { Color, argb, withAlpha } from '@/theme/Colors';
import { type Offset, useElementSize } from '@/ui/FlyShell';

export const VIDEO_AR_VIEW = 'view.videoAr';
const ROUTE_COLOUR = argb(0xffffe9a6);
const HOME_COLOUR = argb(0xff4caf50);
const AIM_COLOUR = argb(0xffff3b30);
const IMPACT_COLOUR = argb(0xffffd60a);
const PIPPER_RADIUS = 14;
const PIPPER_WIDTH = 3;
const PIPPER_DOT = 2;
const LEAD_RADIUS = 9;
const LEAD_WIDTH = 2;
const EDGE_COLOUR = withAlpha(Color.Black, 0.6);
const ROUTE_WIDTH = 3;
const EDGE_WIDTH = 2;
const MARK_RADIUS = 7;
const LABEL_GAP = 4;

export type ArSpot = { x: number; up: number };

export type ArMark = { at: ArSpot; label: string; text: string };

type ArLeg = readonly [ArSpot, ArSpot];

export type ArReading = {
  home: ArMark | null;
  aim: ArMark | null;
  impact: ArMark | null;
  waypoints: readonly ArMark[];
  route: readonly ArLeg[];
  lead: ArSpot | null;
  leadLine: ArLeg | null;
};

const spot = (json: Json | null): ArSpot | null => (json !== null && has(json, 'x') && has(json, 'up') ? { x: optDouble(json, 'x'), up: optDouble(json, 'up') } : null);

const leg = (json: Json | null): ArLeg | null => {
  const start = spot(optJSONObject(json, 'from'));
  const end = spot(optJSONObject(json, 'to'));
  return start !== null && end !== null ? [start, end] : null;
};

const mark = (json: Json | null): ArMark | null => {
  const at = spot(json);
  return at === null ? null : { at, label: optString(json, 'label'), text: optString(json, 'text') };
};

export const arReading = (view: Json | null): ArReading | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  return {
    home: mark(optJSONObject(view, 'home')),
    aim: mark(optJSONObject(view, 'aim')),
    impact: mark(optJSONObject(view, 'impact')),
    waypoints: jsonObjects(optJSONArray(view, 'waypoints')).flatMap((it) => mark(it) ?? []),
    route: jsonObjects(optJSONArray(view, 'route'))
      .map(leg)
      .filter((it): it is ArLeg => it !== null),
    lead: spot(optJSONObject(view, 'lead')),
    leadLine: leg(optJSONObject(view, 'leadLine')),
  };
};

export const arOffset = (spot: ArSpot, width: number, height: number): Offset => ({ x: spot.x * width, y: height / 2 - spot.up * width });

const Marked = ({ mark, fill, width, height, ring = false }: { mark: ArMark; fill: string; width: number; height: number; ring?: boolean }) => {
  const at = arOffset(mark.at, width, height);
  const caption = [mark.label, mark.text].filter((it) => it.trim() !== '').join(' · ');
  const radius = ring ? PIPPER_RADIUS : MARK_RADIUS;
  return (
    <>
      {ring ? (
        <>
          <circle cx={at.x} cy={at.y} r={radius} fill="none" stroke={EDGE_COLOUR} strokeWidth={PIPPER_WIDTH + EDGE_WIDTH * 2} />
          <circle cx={at.x} cy={at.y} r={radius} fill="none" stroke={fill} strokeWidth={PIPPER_WIDTH} />
          <circle cx={at.x} cy={at.y} r={PIPPER_DOT} fill={fill} />
        </>
      ) : (
        <>
          <circle cx={at.x} cy={at.y} r={radius + EDGE_WIDTH} fill={EDGE_COLOUR} />
          <circle cx={at.x} cy={at.y} r={radius} fill={fill} />
        </>
      )}
      <text
        x={at.x}
        y={at.y - radius - LABEL_GAP}
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
        {reading.leadLine !== null
          ? (() => {
              const start = arOffset(reading.leadLine[0], size.width, size.height);
              const end = arOffset(reading.leadLine[1], size.width, size.height);
              return (
                <g strokeLinecap="round">
                  <line x1={start.x} y1={start.y} x2={end.x} y2={end.y} stroke={EDGE_COLOUR} strokeWidth={LEAD_WIDTH + EDGE_WIDTH} />
                  <line x1={start.x} y1={start.y} x2={end.x} y2={end.y} stroke={AIM_COLOUR} strokeWidth={LEAD_WIDTH} />
                </g>
              );
            })()
          : null}
        {reading.lead !== null
          ? (() => {
              const at = arOffset(reading.lead, size.width, size.height);
              return (
                <>
                  <circle cx={at.x} cy={at.y} r={LEAD_RADIUS} fill="none" stroke={EDGE_COLOUR} strokeWidth={LEAD_WIDTH + EDGE_WIDTH} />
                  <circle cx={at.x} cy={at.y} r={LEAD_RADIUS} fill="none" stroke={AIM_COLOUR} strokeWidth={LEAD_WIDTH} />
                </>
              );
            })()
          : null}
        {reading.aim !== null ? <Marked mark={reading.aim} fill={AIM_COLOUR} width={size.width} height={size.height} /> : null}
        {reading.impact !== null ? <Marked mark={reading.impact} fill={IMPACT_COLOUR} width={size.width} height={size.height} ring /> : null}
      </svg>
    </div>
  );
};
