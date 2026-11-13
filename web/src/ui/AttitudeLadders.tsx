import { useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { aircast } from '@/theme/Colors';
import { ATTITUDE_PATH, attitude } from '@/ui/AttitudeInstrument';
import { osdShadow } from '@/ui/Osd';

const LADDER_SPAN = 30;
const LADDER_STEP = 5;
const LABEL_STEP = 10;
const LADDER_MIN = 110;
const LADDER_MAX = 260;
const LADDER_SHARE = 0.32;
const LADDER_PAD = 8;
const LONG_TICK = 22;
const MID_TICK = 16;
const SHORT_TICK = 10;
const TICK_EDGE = 26;
const LADDER_WIDTH = 52;
const CROSSHAIR = 44;
const LADDER_OFFSET = '28%';

export const LADDER_MARKS: readonly number[] = Array.from({ length: (2 * LADDER_SPAN) / LADDER_STEP + 1 }, (_, index) => LADDER_SPAN - index * LADDER_STEP);

export const ladderHeight = (shellHeight: number): number => Math.round(Math.min(LADDER_MAX, Math.max(LADDER_MIN, shellHeight * LADDER_SHARE)));

export const ladderY = (degrees: number, height: number): number =>
  height / 2 - (Math.min(LADDER_SPAN, Math.max(-LADDER_SPAN, degrees)) / LADDER_SPAN) * (height / 2 - LADDER_PAD);

export const ladderTick = (degrees: number): number => (degrees === 0 ? LONG_TICK : degrees % LABEL_STEP === 0 ? MID_TICK : SHORT_TICK);

export const ladderLabel = (degrees: number): string | null => (degrees !== 0 && degrees % LABEL_STEP === 0 ? String(Math.abs(degrees)) : null);

export const ladderReadout = (degrees: number): string => `${Math.round(degrees) === 0 ? 0 : Math.round(degrees)}°`;

type Side = 'start' | 'end';

const Ladder = ({ side, value, height, icon, label }: { side: Side; value: number; height: number; icon: DrawableName; label: string }) => {
  const ink = aircast('osdInk');
  const start = side === 'start';
  const edge = start ? TICK_EDGE : LADDER_WIDTH - TICK_EDGE;
  const readout = ladderReadout(value);
  return (
    <div className="relative" style={{ width: LADDER_WIDTH, height }}>
      <svg width={LADDER_WIDTH} height={height} viewBox={`0 0 ${LADDER_WIDTH} ${height}`} aria-hidden className={`block overflow-visible ${osdShadow()}`}>
        {LADDER_MARKS.map((degrees) => {
          const y = ladderY(degrees, height);
          const tick = ladderTick(degrees);
          const text = ladderLabel(degrees);
          return (
            <g key={degrees}>
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
      <div
        role="img"
        aria-label={`${label} ${readout}`}
        className={`absolute flex items-center gap-1 whitespace-nowrap ${start ? 'flex-row' : 'flex-row-reverse'} ${osdShadow()}`}
        style={{ top: ladderY(value, height), transform: 'translateY(-50%)', ...(start ? { right: LADDER_WIDTH - edge + LONG_TICK + 2 } : { left: edge + LONG_TICK + 2 }), color: ink }}
      >
        <Drawable name={icon} size={16} style={{ fill: ink }} />
        <span className="text-label-large tabular-nums">{readout}</span>
        <svg width={10} height={12} viewBox="0 0 10 12" aria-hidden className="block">
          <path d={start ? 'M 0 6 L 10 6 M 10 0 L 10 12' : 'M 10 6 L 0 6 M 0 0 L 0 12'} strokeWidth={2} style={{ stroke: ink }} />
        </svg>
      </div>
    </div>
  );
};

export const Crosshair = () => {
  const ink = aircast('osdInk');
  const half = CROSSHAIR / 2;
  return (
    <svg width={CROSSHAIR} height={CROSSHAIR} viewBox={`0 0 ${CROSSHAIR} ${CROSSHAIR}`} aria-hidden className={`block ${osdShadow()}`}>
      <path d={`M 0 ${half} H ${CROSSHAIR} M ${half} 0 V ${CROSSHAIR}`} strokeWidth={2} style={{ stroke: ink }} />
    </svg>
  );
};

export const AttitudeLadders = ({ shellHeight }: { shellHeight: number }) => {
  const view = useQgcPath(ATTITUDE_PATH);
  const reading = useMemo(() => attitude(view), [view]);
  if (reading === null) return null;
  const height = ladderHeight(shellHeight);
  return (
    <div role="group" aria-label="Attitude" className="pointer-events-none absolute inset-0">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2">
        <Crosshair />
      </div>
      <div className="absolute top-1/2 -translate-x-1/2 -translate-y-1/2" style={{ left: LADDER_OFFSET }}>
        <Ladder side="start" value={reading.pitch} height={height} icon="ic_height" label="Pitch" />
      </div>
      <div className="absolute top-1/2 translate-x-1/2 -translate-y-1/2" style={{ right: LADDER_OFFSET }}>
        <Ladder side="end" value={reading.roll} height={height} icon="ic_redo" label="Roll" />
      </div>
    </div>
  );
};
