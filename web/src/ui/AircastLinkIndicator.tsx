import { useMemo, useState } from 'react';
import { type Json, type JsonArray, optBoolean, optJSONArray, optString } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { aircast, md } from '@/theme/Colors';

export const AIRCAST_LINK_VIEW = 'view.aircastLink';
const SIGNAL_MAXIMUM = 100.0;
const SPARKLINE_HEIGHT = 48;

export type AircastLink = {
  qualityText: string;
  signalText: string;
  network: string;
  modem: string;
  bitrateText: string;
  qualityHistory: readonly number[];
  bitrateHistory: readonly number[];
};

const numbers = (array: JsonArray | null): readonly number[] =>
  (array ?? []).map((value) => (typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN));

export const aircastLink = (view: Json | null): AircastLink | null =>
  view !== null && optBoolean(view, 'shown')
    ? {
        qualityText: optString(view, 'qualityText'),
        signalText: optString(view, 'signalText'),
        network: optString(view, 'network'),
        modem: optString(view, 'modem'),
        bitrateText: optString(view, 'bitrateText'),
        qualityHistory: numbers(optJSONArray(view, 'qualityHistory')),
        bitrateHistory: numbers(optJSONArray(view, 'bitrateHistory')),
      }
    : null;

type SparkPoint = readonly [number, number];

export const sparkRuns = (values: readonly number[], maximum: number): readonly (readonly SparkPoint[])[] => {
  if (values.length < 2) return [];
  const highest = maximum > 0 ? maximum : Math.max(...values);
  const peak = highest > 0 ? highest : 1;
  const step = 1 / (values.length - 1);
  return values
    .reduce<readonly (readonly SparkPoint[])[]>(
      (runs, value, index) => {
        const last = runs[runs.length - 1] ?? [];
        if (value < 0) return last.length === 0 ? runs : [...runs, []];
        return [...runs.slice(0, -1), [...last, [index * step, 1 - Math.min(value, peak) / peak] as const]];
      },
      [[]],
    )
    .filter((run) => run.length >= 2);
};

const Sparkline = ({ values, maximum }: { values: readonly number[]; maximum: number }) => (
  <svg className="w-full" height={SPARKLINE_HEIGHT} viewBox={`0 0 1 ${SPARKLINE_HEIGHT}`} preserveAspectRatio="none" aria-hidden>
    {sparkRuns(values, maximum).map((run, index) => (
      <path
        key={index}
        d={run.map(([x, y], at) => `${at === 0 ? 'M' : 'L'}${x} ${y * (SPARKLINE_HEIGHT - 1) + 0.5}`).join(' ')}
        fill="none"
        stroke={aircast('success')}
        strokeWidth={1.5}
        vectorEffect="non-scaling-stroke"
      />
    ))}
  </svg>
);

export const AircastLinkCell = () => {
  const json = useQgcPath(AIRCAST_LINK_VIEW);
  const link = useMemo(() => aircastLink(json), [json]);
  const [open, setOpen] = useState(false);
  if (link === null) return null;
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        <Text typography="labelMedium" color={md('onSurfaceVariant')} maxLines={1}>
          {`${link.qualityText} ${link.bitrateText}`}
        </Text>
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-1.5 px-sheet pb-6">
            <Text typography="titleSmall">Cellular link</Text>
            {(
              [
                ['Signal:', link.signalText],
                ['Network:', link.network],
                ['Modem:', link.modem],
                ['Video bitrate:', link.bitrateText],
              ] as const
            ).map(([label, value]) => (
              <Text key={label} typography="bodySmall">{`${label}  ${value}`}</Text>
            ))}
            <Text typography="labelSmall">Signal, up to the last hour</Text>
            <Sparkline values={link.qualityHistory} maximum={SIGNAL_MAXIMUM} />
            <Text typography="labelSmall">Video bitrate, up to the last hour</Text>
            <Sparkline values={link.bitrateHistory} maximum={0} />
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};
