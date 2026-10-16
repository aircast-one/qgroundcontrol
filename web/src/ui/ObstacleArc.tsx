import { useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { coerceAtLeast, coerceIn } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { md, withAlpha } from '@/theme/Colors';
import { obstacleRing } from './ObstacleDistance';

const OBSTACLE_PATH = 'view.obstacle';
const ARC_SIZE = 48;
const SWEEP_PADDING = 0.85;

export const arcSweep = (increment: number): number => increment * SWEEP_PADDING;

export const NEAREST_VISIBLE_FRACTION = 0.12;

export const arcRadiusFraction = (metres: number, ceiling: number): number =>
  ceiling <= 0.0 ? 0 : coerceAtLeast(Math.sqrt(coerceIn(metres / ceiling, 0.0, 1.0)), NEAREST_VISIBLE_FRACTION);

export const sampleIsClose = (metres: number, floorMetres: number): boolean => metres < floorMetres * 2.0;

export type ArcTone = { readonly name: 'Alarm' | 'Calm' | 'Stale'; readonly ordinal: number };

export const ArcTone = {
  Alarm: { name: 'Alarm', ordinal: 0 },
  Calm: { name: 'Calm', ordinal: 1 },
  Stale: { name: 'Stale', ordinal: 2 },
} as const satisfies Record<string, ArcTone>;

export const ArcToneEntries: readonly ArcTone[] = [ArcTone.Alarm, ArcTone.Calm, ArcTone.Stale];

export const arcTone = (near: boolean, stale: boolean): ArcTone => (stale ? ArcTone.Stale : near ? ArcTone.Alarm : ArcTone.Calm);

const devicePixels = (pixels: number): number => pixels / (typeof window === 'undefined' ? 1 : window.devicePixelRatio || 1);

const arcPath = (cx: number, cy: number, radius: number, startDegrees: number, sweepDegrees: number): string => {
  const at = (degrees: number) => {
    const radians = (degrees * Math.PI) / 180;
    return `${cx + radius * Math.cos(radians)} ${cy + radius * Math.sin(radians)}`;
  };
  return `M ${at(startDegrees)} A ${radius} ${radius} 0 ${sweepDegrees > 180 ? 1 : 0} 1 ${at(startDegrees + sweepDegrees)}`;
};

export const ObstacleArc = ({ className }: { className?: string }) => {
  const view = useQgcPath(OBSTACLE_PATH);
  const ring = useMemo(() => obstacleRing(view), [view]);

  if (ring === null) return null;

  const alarm = md('error');
  const calm = md('onSurfaceVariant');
  const faded = withAlpha(calm, 0.45);
  const full = ARC_SIZE / 2;
  const sweep = arcSweep(ring.incrementDegrees);

  return (
    <Surface className={className} color={withAlpha(md('surface'), 0.8)} shape="small">
      <div className="flex flex-col items-center gap-0.5">
        <svg width={ARC_SIZE} height={ARC_SIZE} viewBox={`0 0 ${ARC_SIZE} ${ARC_SIZE}`} className="overflow-visible" aria-hidden>
          <circle cx={full} cy={full} r={full} fill="none" strokeWidth={devicePixels(2)} style={{ stroke: withAlpha(calm, 0.25) }} />
          <circle cx={full} cy={full} r={devicePixels(3)} style={{ fill: withAlpha(calm, 0.6) }} />
          {ring.samples.map((sample, index) => {
            const reach = full * arcRadiusFraction(sample.metres, ring.maxMetres);
            const near = sampleIsClose(sample.metres, ring.floorMetres);
            const tone = arcTone(near, ring.stale);
            const colour = tone === ArcTone.Alarm ? alarm : tone === ArcTone.Calm ? calm : faded;
            return (
              <path
                key={index}
                d={arcPath(full, full, reach, sample.bearingDegrees - 90.0 - sweep / 2, sweep)}
                fill="none"
                strokeWidth={devicePixels(near ? 10 : 6)}
                style={{ stroke: colour }}
              />
            );
          })}
        </svg>
        {ring.stale ? (
          <Text typography="labelSmall" color={md('onSurfaceVariant')}>
            Last seen
          </Text>
        ) : null}
      </div>
    </Surface>
  );
};
