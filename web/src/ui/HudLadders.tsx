import { useMemo } from 'react';
import { useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { aircast } from '@/theme/Colors';
import { ATTITUDE_PATH, attitude } from '@/ui/AttitudeInstrument';
import { osdShadow } from '@/ui/Osd';
import { LayoutWidget } from '@/ui/OverlayLayout';

export const THROTTLE_PATH = 'vehicle.throttlePct';
const LADDER_MIN = 110;
const LADDER_MAX = 260;
const LADDER_SHARE = 0.3;
const LADDER_PAD = 8;
const LONG_TICK = 22;
const MID_TICK = 16;
const SHORT_TICK = 10;
const TICK_EDGE = 26;
const LADDER_WIDTH = 52;
const CROSSHAIR = 44;
const LADDER_REACH = 0.38;
const COMPACT_HEIGHT = 500;
const COMPACT_GAP = 10;

export type LadderScale = {
  readonly top: number;
  readonly bottom: number;
  readonly step: number;
  readonly labelEvery: number;
  readonly major: number | null;
  readonly label: (value: number) => string | null;
  readonly readout: (value: number) => string;
};

const whole = (value: number): number => (Math.round(value) === 0 ? 0 : Math.round(value));

export const PITCH_SCALE: LadderScale = {
  top: 30,
  bottom: -30,
  step: 5,
  labelEvery: 10,
  major: 0,
  label: (value) => (value === 0 ? null : String(Math.abs(value))),
  readout: (value) => `${whole(value)}°`,
};

export const THROTTLE_SCALE: LadderScale = {
  top: 100,
  bottom: 0,
  step: 10,
  labelEvery: 20,
  major: null,
  label: (value) => String(value),
  readout: (value) => `${whole(value)}%`,
};

export const ladderMarks = (scale: LadderScale): readonly number[] =>
  Array.from({ length: (scale.top - scale.bottom) / scale.step + 1 }, (_, index) => scale.top - index * scale.step);

export const ladderHeight = (shellHeight: number): number => Math.round(Math.min(LADDER_MAX, Math.max(LADDER_MIN, shellHeight * LADDER_SHARE)));

export const ladderReach = (shellHeight: number): number => Math.round(shellHeight * LADDER_REACH);

export const compactHud = (shellHeight: number): boolean => shellHeight < COMPACT_HEIGHT;

export const ladderY = (value: number, scale: LadderScale, height: number): number =>
  LADDER_PAD + ((scale.top - Math.min(scale.top, Math.max(scale.bottom, value))) / (scale.top - scale.bottom)) * (height - 2 * LADDER_PAD);

const labelled = (value: number, scale: LadderScale): boolean => (value - scale.bottom) % scale.labelEvery === 0;

export const ladderTick = (value: number, scale: LadderScale): number => (value === scale.major ? LONG_TICK : labelled(value, scale) ? MID_TICK : SHORT_TICK);

export const ladderLabel = (value: number, scale: LadderScale): string | null => (labelled(value, scale) ? scale.label(value) : null);

type Side = 'start' | 'end';

const Ladder = ({ side, scale, value, height, icon, label }: { side: Side; scale: LadderScale; value: number | null; height: number; icon: DrawableName; label: string }) => {
  const ink = aircast('outdoorForeground');
  const start = side === 'start';
  const edge = start ? TICK_EDGE : LADDER_WIDTH - TICK_EDGE;
  return (
    <div className="relative" style={{ width: LADDER_WIDTH, height }}>
      <svg width={LADDER_WIDTH} height={height} viewBox={`0 0 ${LADDER_WIDTH} ${height}`} aria-hidden className="block overflow-visible">
        {ladderMarks(scale).map((mark) => {
          const y = ladderY(mark, scale, height);
          const tick = ladderTick(mark, scale);
          const text = ladderLabel(mark, scale);
          return (
            <g key={mark}>
              <line x1={start ? edge - tick : edge} y1={y} x2={start ? edge : edge + tick} y2={y} strokeWidth={2} strokeLinecap="round" style={{ stroke: ink }} />
              {text !== null ? (
                <text x={start ? edge + 5 : edge - 5} y={y} textAnchor={start ? 'start' : 'end'} dominantBaseline="central" fontSize={12} fontWeight={600} style={{ fill: ink, fontVariantNumeric: 'tabular-nums' }}>
                  {text}
                </text>
              ) : null}
            </g>
          );
        })}
      </svg>
      {value !== null ? (
        <div
          role="img"
          aria-label={`${label} ${scale.readout(value)}`}
          className={`absolute flex items-center gap-1 whitespace-nowrap ${start ? 'flex-row' : 'flex-row-reverse'}`}
          style={{ top: ladderY(value, scale, height), transform: 'translateY(-50%)', ...(start ? { right: LADDER_WIDTH - edge + LONG_TICK + 2 } : { left: edge + LONG_TICK + 2 }), color: ink }}
        >
          <Drawable name={icon} size={16} style={{ fill: ink }} />
          <span className="text-title-medium font-bold tabular-nums">{scale.readout(value)}</span>
          <svg width={10} height={12} viewBox="0 0 10 12" aria-hidden className="block">
            <path d={start ? 'M 0 6 L 10 6 M 10 0 L 10 12' : 'M 10 6 L 0 6 M 0 0 L 0 12'} strokeWidth={2} style={{ stroke: ink }} />
          </svg>
        </div>
      ) : null}
    </div>
  );
};

export const Crosshair = () => {
  const ink = aircast('outdoorForeground');
  const half = CROSSHAIR / 2;
  return (
    <svg width={CROSSHAIR} height={CROSSHAIR} viewBox={`0 0 ${CROSSHAIR} ${CROSSHAIR}`} aria-hidden className={`block ${osdShadow()}`}>
      <path d={`M 0 ${half} H ${CROSSHAIR} M ${half} 0 V ${CROSSHAIR}`} strokeWidth={2} style={{ stroke: ink }} />
    </svg>
  );
};

const Readout = ({ scale, value, icon, label }: { scale: LadderScale; value: number; icon: DrawableName; label: string }) => {
  const ink = aircast('outdoorForeground');
  return (
    <div role="img" aria-label={`${label} ${scale.readout(value)}`} className="flex items-center gap-1 whitespace-nowrap" style={{ color: ink }}>
      <Drawable name={icon} size={16} style={{ fill: ink }} />
      <span className="text-title-medium font-bold tabular-nums">{scale.readout(value)}</span>
    </div>
  );
};

const CompactSight = ({ pitch, throttle }: { pitch: number; throttle: number | null }) => (
  <div role="group" aria-label="Sight" className="pointer-events-none absolute inset-0">
    <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2">
      <Crosshair />
    </div>
    <div className="absolute top-1/2 -translate-y-1/2" style={{ right: `calc(50% + ${CROSSHAIR / 2 + COMPACT_GAP}px)` }}>
      <LayoutWidget widgetKey="pitchLadder" movable={false}>
        <Readout scale={PITCH_SCALE} value={pitch} icon="ic_height" label="Pitch" />
      </LayoutWidget>
    </div>
    {throttle !== null ? (
      <div className="absolute top-1/2 -translate-y-1/2" style={{ left: `calc(50% + ${CROSSHAIR / 2 + COMPACT_GAP}px)` }}>
        <LayoutWidget widgetKey="throttleLadder" movable={false}>
          <Readout scale={THROTTLE_SCALE} value={throttle} icon="ic_speed" label="Throttle" />
        </LayoutWidget>
      </div>
    ) : null}
  </div>
);

export const HudLadders = ({ shellHeight }: { shellHeight: number }) => {
  const view = useQgcPath(ATTITUDE_PATH);
  const reading = useMemo(() => attitude(view), [view]);
  const throttle = useQgcDouble(THROTTLE_PATH);
  if (reading === null) return null;
  if (compactHud(shellHeight)) return <CompactSight pitch={reading.pitch} throttle={Number.isNaN(throttle) ? null : throttle} />;
  const height = ladderHeight(shellHeight);
  const reach = `calc(50% - ${ladderReach(shellHeight)}px)`;
  return (
    <div role="group" aria-label="Sight" className="pointer-events-none absolute inset-0">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2">
        <Crosshair />
      </div>
      <div className="absolute top-1/2 -translate-x-1/2 -translate-y-1/2" style={{ left: reach }}>
        <LayoutWidget widgetKey="pitchLadder" movable={false}>
          <Ladder side="start" scale={PITCH_SCALE} value={reading.pitch} height={height} icon="ic_height" label="Pitch" />
        </LayoutWidget>
      </div>
      <div className="absolute top-1/2 translate-x-1/2 -translate-y-1/2" style={{ right: reach }}>
        <LayoutWidget widgetKey="throttleLadder" movable={false}>
          <Ladder side="end" scale={THROTTLE_SCALE} value={Number.isNaN(throttle) ? null : throttle} height={height} icon="ic_speed" label="Throttle" />
        </LayoutWidget>
      </div>
    </div>
  );
};
