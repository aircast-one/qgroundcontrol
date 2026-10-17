import { useMemo, useRef, useState } from 'react';
import { optDouble } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay, format } from '@/kotlin/Std';
import { Checkbox, OutlinedButton, Text } from '@/m3';
import { useKeyedState } from '@/map/SettingStepper';
import { argb, md } from '@/theme/Colors';
import { FLY_STATE, flyState } from '@/ui/FlyState';
import type { TuningAxis, TuningModes } from '@/ui/Px4TuningScreen';

const SAMPLE_MS = 10;
export const FLIGHT_MODE_PATH = 'vehicle.flightMode';
const HISTORY_SECONDS = 180.0;
const SERIES_COLOURS = [argb(0xff2196f3), argb(0xffff9800)];

export type Sample = { seconds: number; value: number };

export const Sample = (seconds: number, value: number): Sample => ({ seconds, value });

export const withSample = (series: readonly Sample[], sample: Sample): readonly Sample[] => {
  const joined = [...series, sample];
  const first = joined.findIndex((it) => !(it.seconds < sample.seconds - HISTORY_SECONDS));
  return first < 0 ? [] : joined.slice(first);
};

const TICK_SEPARATION = 5.0;

const grownLow = (current: number, value: number): number => {
  const low = Math.min(current, value);
  return low % TICK_SEPARATION !== 0.0 ? Math.floor((low - TICK_SEPARATION) / TICK_SEPARATION) * TICK_SEPARATION : low;
};

const grownHigh = (current: number, value: number): number => {
  const high = Math.max(current, value);
  return high % TICK_SEPARATION !== 0.0 ? Math.floor((high + TICK_SEPARATION) / TICK_SEPARATION) * TICK_SEPARATION : high;
};

export type Range = readonly [number, number];

export const grownRange = (range: Range | null, values: readonly number[]): Range | null =>
  values.reduce<Range | null>((held, value) => (held === null ? [value, value] : [grownLow(held[0], value), grownHigh(held[1], value)]), range);

const factValue = async (path: string): Promise<number | null> => {
  const value = optDouble(await Qgc.get(path), 'value');
  return Number.isNaN(value) ? null : value;
};

type Trace = { series: readonly (readonly Sample[])[]; now: number; range: Range | null };

const CHART_HEIGHT = 180;
const CHART_WIDTH = 1000;

const seriesPath = (shown: readonly Sample[], from: number, windowSeconds: number, low: number, span: number): string =>
  shown
    .map((sample, at) => {
      const x = ((sample.seconds - from) / windowSeconds) * CHART_WIDTH;
      const y = CHART_HEIGHT - ((sample.value - low) / span) * CHART_HEIGHT;
      return `${at === 0 ? 'M' : 'L'}${x},${y}`;
    })
    .join(' ');

export const TuningChart = ({ axis, unit, windowSeconds, modes }: { axis: TuningAxis; unit: string; windowSeconds: number; modes: TuningModes | null }) => {
  const axisKey = useMemo(() => JSON.stringify(axis), [axis]);
  const [running, setRunning] = useKeyedState([axisKey], () => true);
  const [autoModeChange, setAutoModeChange] = useState(false);
  const [cleared, setCleared] = useKeyedState([axisKey], () => 0);
  const [trace, setTraceState] = useKeyedState<Trace>([axisKey, cleared], () => ({ series: axis.plot.map(() => []), now: 0.0, range: null }));
  const traceRef = useRef(trace);
  traceRef.current = trace;
  const armed = flyState(useQgcPath(FLY_STATE))?.armed === true;

  useLaunchedEffect([armed], () => {
    if (armed && !running) setRunning(true);
  });

  useLaunchedEffect([axisKey, cleared, running], async (scope) => {
    if (!running) return;
    const started = performance.now() / 1000 - traceRef.current.now;
    const loop = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const values = await Promise.all(axis.plot.map((it) => factValue(it.path)));
      if (!scope.isActive()) return;
      const held = traceRef.current;
      const now = performance.now() / 1000 - started;
      const next: Trace = {
        now,
        series: held.series.map((list, index) => {
          const value = values[index] ?? null;
          return value === null ? list : withSample(list, Sample(now, value));
        }),
        range: grownRange(held.range, values.filter((it): it is number => it !== null)),
      };
      traceRef.current = next;
      setTraceState(next);
      await delay(SAMPLE_MS);
      return loop();
    };
    await loop();
  });

  const from = trace.now - windowSeconds;
  const grid = md('outlineVariant');
  const range = trace.range;
  const span = range === null ? 1.0 : range[1] - range[0] > 0 ? range[1] - range[0] : 1.0;
  return (
    <div className="flex flex-col gap-1.5">
      <Text typography="titleSmall">{axis.chartTitle}</Text>
      <svg width="100%" height={CHART_HEIGHT} viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`} preserveAspectRatio="none" className="block w-full" style={{ background: md('surfaceContainer') }} aria-hidden>
        <line x1={0} y1={CHART_HEIGHT / 2} x2={CHART_WIDTH} y2={CHART_HEIGHT / 2} stroke={grid} strokeWidth={1} vectorEffect="non-scaling-stroke" />
        {range !== null
          ? trace.series.map((list, index) => {
              const shown = list.filter((it) => it.seconds >= from);
              return shown.length < 2 ? null : (
                <path
                  key={index}
                  d={seriesPath(shown, from, windowSeconds, range[0], span)}
                  fill="none"
                  stroke={SERIES_COLOURS[index % SERIES_COLOURS.length]}
                  strokeWidth={2}
                  vectorEffect="non-scaling-stroke"
                />
              );
            })
          : null}
      </svg>
      <div className="flex flex-wrap items-center gap-3">
        {axis.plot.map((plot, index) => (
          <div key={index} className="flex items-center gap-1">
            <span className="block size-2.5" style={{ background: SERIES_COLOURS[index % SERIES_COLOURS.length] }} />
            <Text typography="labelSmall">{plot.name}</Text>
          </div>
        ))}
        <Text typography="labelSmall">{range === null ? '' : format('%.2f … %.2f %s', range[0], range[1], unit)}</Text>
      </div>
      <div className="flex gap-2">
        <OutlinedButton onClick={() => setCleared(cleared + 1)}>Clear</OutlinedButton>
        <OutlinedButton
          onClick={() => {
            const next = !running;
            setRunning(next);
            if (modes !== null && autoModeChange) {
              const mode = next ? modes.stabilized : modes.pause;
              void Qgc.writeRefusal(FLIGHT_MODE_PATH, mode);
            }
          }}
        >
          {running ? 'Stop' : 'Start'}
        </OutlinedButton>
      </div>
      {modes !== null ? (
        <>
          <div className="flex items-center gap-3">
            <Checkbox
              checked={autoModeChange}
              onCheckedChange={(checked) => {
                setAutoModeChange(checked);
                if (checked) setRunning(false);
              }}
              label="Automatic flight mode switching"
            />
            <Text>Automatic flight mode switching</Text>
          </div>
          {autoModeChange ? (
            <>
              <Text typography="bodySmall">{"Switches to 'Stabilized' when you click Start."}</Text>
              <Text typography="bodySmall">{`Switches to '${modes.pause}' when you click Stop.`}</Text>
            </>
          ) : null}
        </>
      ) : null}
    </div>
  );
};
