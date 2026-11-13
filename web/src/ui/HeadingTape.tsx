import { useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Surface } from '@/m3/Surface';
import { Color, aircast, withAlpha } from '@/theme/Colors';
import { ATTITUDE_PATH, attitude } from '@/ui/AttitudeInstrument';
import { osdShadow, useOsdBackdrop } from '@/ui/Osd';

const TAPE_SPAN_DEGREES = 120;
const TICK_STEP = 10;
const LABEL_STEP = 30;
const READOUT_CLEARANCE_DEGREES = 15;
const EDGE_MARGIN_DEGREES = 5;
const EDGE_FADE = 0.6;
const EDGE_REACH = TAPE_SPAN_DEGREES / 2 - EDGE_MARGIN_DEGREES;
const TAPE_WIDTH = 320;
const TAPE_HEIGHT = 40;
const POINTER = 6;
const POINTER_WING = 5;
const LABEL_TICK = 10;
const MINOR_TICK = 5;
const GAP = 2;
const HOME_RADIUS = 6;
const LINE = POINTER + GAP;
const LABEL_TOP = LINE + LABEL_TICK + GAP;
const TAPE_BACKDROP = withAlpha(Color.Black, 0.35);
const CARDINALS: Readonly<Record<number, string>> = { 0: 'N', 90: 'E', 180: 'S', 270: 'W' };

const floorMod = (value: number, modulus: number): number => ((value % modulus) + modulus) % modulus;

export const relativeBearing = (bearing: number, heading: number): number => floorMod(bearing - heading + 180, 360) - 180;

export const tapeX = (relative: number, width: number): number => width / 2 + (relative / TAPE_SPAN_DEGREES) * width;

export const tapeLabel = (degrees: number): string => CARDINALS[floorMod(degrees, 360)] ?? String(floorMod(degrees, 360));

export const tapeTicks = (heading: number): number[] => {
  const half = TAPE_SPAN_DEGREES / 2;
  const first = Math.ceil((heading - half) / TICK_STEP) * TICK_STEP;
  const last = Math.floor((heading + half) / TICK_STEP) * TICK_STEP;
  return Array.from({ length: (last - first) / TICK_STEP + 1 }, (_, index) => first + index * TICK_STEP);
};

export const tapeLabelShown = (relative: number): boolean =>
  Math.abs(relative) > READOUT_CLEARANCE_DEGREES && Math.abs(relative) < EDGE_REACH;

export const headingReadout = (heading: number): string => `${floorMod(Math.round(heading), 360)}°`;

const clampedTapeX = (bearing: number, heading: number): number =>
  tapeX(Math.min(Math.max(relativeBearing(bearing, heading), -EDGE_REACH), EDGE_REACH), TAPE_WIDTH);

const pointer = (x: number): string => `M ${x - POINTER_WING} 0 L ${x + POINTER_WING} 0 L ${x} ${POINTER} Z`;

export const HeadingTape = ({ className }: { className?: string }) => {
  const view = useQgcPath(ATTITUDE_PATH);
  const reading = useMemo(() => attitude(view), [view]);
  const backdrop = useOsdBackdrop(TAPE_BACKDROP);
  if (reading === null) return null;
  const ink = Color.White;
  const heading = headingReadout(reading.heading);
  const homeBearing = reading.homeBearing ?? reading.headingToHome;
  return (
    <Surface color={backdrop} shape="small" className={`shrink-0 ${className ?? ''}`} style={{ width: TAPE_WIDTH, height: TAPE_HEIGHT }}>
      <svg width={TAPE_WIDTH} height={TAPE_HEIGHT} viewBox={`0 0 ${TAPE_WIDTH} ${TAPE_HEIGHT}`} role="img" aria-label={`Heading ${heading}`} className={`block ${osdShadow()}`}>
        {tapeTicks(reading.heading).map((degrees) => {
          const relative = relativeBearing(degrees, reading.heading);
          const x = tapeX(relative, TAPE_WIDTH);
          const shade = withAlpha(ink, 1 - (Math.abs(relative) / (TAPE_SPAN_DEGREES / 2)) * EDGE_FADE);
          const labelled = degrees % LABEL_STEP === 0;
          return (
            <g key={degrees}>
              <line x1={x} y1={LINE} x2={x} y2={LINE + (labelled ? LABEL_TICK : MINOR_TICK)} strokeWidth={1.5} style={{ stroke: shade }} />
              {labelled && tapeLabelShown(relative) ? (
                <text x={x} y={LABEL_TOP} textAnchor="middle" dominantBaseline="hanging" fontSize={11} fontWeight={600} style={{ fill: shade }}>
                  {tapeLabel(degrees)}
                </text>
              ) : null}
            </g>
          );
        })}
        {reading.headingToNextWaypoint !== null ? <path d={pointer(clampedTapeX(reading.headingToNextWaypoint, reading.heading))} style={{ fill: aircast('mission') }} /> : null}
        {homeBearing !== null ? (
          <g>
            <circle cx={clampedTapeX(homeBearing, reading.heading)} cy={LINE} r={HOME_RADIUS} style={{ fill: aircast('success') }} />
            <text x={clampedTapeX(homeBearing, reading.heading)} y={LINE} textAnchor="middle" dominantBaseline="central" fontSize={9} fontWeight="bold" style={{ fill: aircast('onSuccess') }}>
              H
            </text>
          </g>
        ) : null}
        <path d={pointer(TAPE_WIDTH / 2)} style={{ fill: ink }} />
        <text x={TAPE_WIDTH / 2} y={LABEL_TOP - GAP} textAnchor="middle" dominantBaseline="hanging" fontSize={15} fontWeight="bold" style={{ fill: ink, fontVariantNumeric: 'tabular-nums' }}>
          {heading}
        </text>
      </svg>
    </Surface>
  );
};
