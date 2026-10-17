import { useState } from 'react';
import { type Json, type JsonArray, isNull, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optLong, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useTicker } from '@/kotlin/Compose';
import { coerceAtLeast, coerceIn, format } from '@/kotlin/Std';
import { DropdownMenu, DropdownMenuItem, Surface, Text, TextButton } from '@/m3';
import { aircast, md } from '@/theme/Colors';
import { useElementSize } from '@/ui/FlyShell';

export const INSPECTOR_CHARTS_VIEW = 'view.inspectorCharts';
const CHART_ADD = 'mavlinkInspector.chart.add';
const CHART_REMOVE = 'mavlinkInspector.chart.remove';
const CHART_RANGE_X = 'mavlinkInspector.chart.rangeX';
const CHART_RANGE_Y = 'mavlinkInspector.chart.rangeY';

export type ChartPlot = { label: string; field: string; colour: number; points: readonly (readonly [number, number])[] };

export type ChartModel = { rangeX: number; rangeY: number; windowMs: number; yMin: number | null; yMax: number | null; room: boolean; plots: readonly ChartPlot[] };

export type InspectorCharts = { timeScales: readonly string[]; ranges: readonly string[]; charts: readonly ChartModel[]; charted: ReadonlyMap<string, number> };

const number = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

const numeric = (value: unknown): number => (typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN);

const sample = (point: JsonArray): readonly [number, number] => {
  const age = numeric(point[0]);
  return [Number.isFinite(age) ? Math.trunc(age) : 0, numeric(point[1])];
};

export const inspectorCharts = (view: Json | null): InspectorCharts | null => {
  const charts = optJSONArray(view, 'charts');
  if (view === null || charts === null) return null;
  return {
    timeScales: jsonStrings(optJSONArray(view, 'timeScales')),
    ranges: jsonStrings(optJSONArray(view, 'ranges')),
    charts: jsonObjects(charts).map((chart) => ({
      rangeX: optInt(chart, 'rangeX'),
      rangeY: optInt(chart, 'rangeY'),
      windowMs: optLong(chart, 'windowMs', 5000),
      yMin: number(chart, 'yMin'),
      yMax: number(chart, 'yMax'),
      room: optBoolean(chart, 'room', true),
      plots: jsonObjects(optJSONArray(chart, 'plots')).map((plot) => ({
        label: optText(plot, 'label'),
        field: optText(plot, 'field'),
        colour: optInt(plot, 'colour'),
        points: (optJSONArray(plot, 'points') ?? []).filter((it): it is JsonArray => Array.isArray(it)).map(sample),
      })),
    })),
    charted: new Map(jsonObjects(optJSONArray(view, 'selectedCharted')).map((it) => [optText(it, 'field'), optInt(it, 'chart')] as const)),
  };
};

const CHART_HEIGHT = 220;
const TIME_TICKS = 3;
const CLOCK_TICK_MS = 1000;
const VALUE_TICKS = 4;
const TICK_FONT_SIZE = 11;
const TICK_LINE_HEIGHT = 16;
const MONOSPACE_ADVANCE = 0.6;

const pad2 = (value: number): string => format('%02d', value);

export const timeTicks = (windowMs: number, nowMs: number, zone: (instant: number) => number = (instant) => -new Date(instant).getTimezoneOffset() * 60_000): readonly (readonly [number, string])[] =>
  Array.from({ length: TIME_TICKS + 1 }, (_, step) => {
    const at = step / TIME_TICKS;
    const instant = nowMs - Math.trunc((1 - at) * windowMs);
    const wallClock = instant + zone(instant);
    return [at, `${pad2(Math.trunc(wallClock / 60_000) % 60)}:${pad2(Math.trunc(wallClock / 1000) % 60)}`] as const;
  });

const formatG = (value: number, precision: number): string => {
  if (!Number.isFinite(value)) return Number.isNaN(value) ? 'NaN' : value > 0 ? 'Infinity' : '-Infinity';
  const magnitude = Math.abs(Number(value.toPrecision(precision)));
  return magnitude !== 0 && (magnitude < 1e-4 || magnitude >= 10 ** precision)
    ? value.toExponential(precision - 1).replace(/e([+-])(\d)$/, 'e$10$2')
    : value.toPrecision(precision);
};

export const valueTicks = (low: number, high: number): readonly (readonly [number, string])[] =>
  Array.from({ length: VALUE_TICKS + 1 }, (_, step) => {
    const fraction = step / VALUE_TICKS;
    return [1 - fraction, formatG(low + (high - low) * fraction, 4)] as const;
  });

export const latestValue = (points: readonly (readonly [number, number])[]): string | null => {
  const newest = points.reduce<readonly [number, number] | null>((best, it) => (best === null || it[0] < best[0] ? it : best), null);
  return newest === null ? null : formatG(newest[1], 4);
};

const useSeriesColours = (): readonly string[] => [md('primary'), aircast('mission'), aircast('success'), md('tertiary'), md('error'), aircast('warning')];

export const chartPoint = (ageMs: number, value: number, windowMs: number, yMin: number, yMax: number): readonly [number, number] => {
  const span = yMax - yMin > 0 ? yMax - yMin : 1.0;
  return [1 - ageMs / coerceAtLeast(windowMs, 1), 1 - coerceIn((value - yMin) / span, 0, 1)] as const;
};

export const fieldChartable = (type: string): boolean => !type.startsWith('char');

export const chartToggleEnabled = (charts: InspectorCharts | null, field: string, type: string, chart: number): boolean => {
  const on = charts?.charted.get(field) ?? null;
  if (on === chart) return true;
  if (!fieldChartable(type) || on !== null) return false;
  return charts?.charts[chart]?.room ?? true;
};

export const toggleChartField = (chart: number, field: string, on: boolean): void => {
  void Qgc.invoke(on ? CHART_ADD : CHART_REMOVE, chart, field);
};

const ChartChoice = ({ title, options, chosen, onChosen }: { title: string; options: readonly string[]; chosen: number; onChosen: (index: number) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <TextButton onClick={() => setOpen(true)}>{`${title}: ${options[chosen] ?? ''}`}</TextButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {options.map((option, index) => (
          <DropdownMenuItem
            key={index}
            text={option}
            onClick={() => {
              setOpen(false);
              onChosen(index);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};

const textWidth = (label: string): number => label.length * TICK_FONT_SIZE * MONOSPACE_ADVANCE;

const ChartCanvas = ({ chart, colours }: { chart: ChartModel; colours: readonly string[] }) => {
  const { ref, size } = useElementSize();
  const [now, setNow] = useState(() => Date.now());
  useTicker(CLOCK_TICK_MS, [], () => setNow(Date.now()));
  const width = size.width;
  const low = chart.yMin;
  const high = chart.yMax;
  const drawn = (() => {
    if (low === null || high === null || width <= 0) return null;
    const values = valueTicks(low, high);
    const times = timeTicks(chart.windowMs, now);
    const gap = 4;
    const left = values.reduce((widest, [, label]) => Math.max(widest, textWidth(label)), 0) + gap;
    const plotHeight = CHART_HEIGHT - (times.length > 0 ? TICK_LINE_HEIGHT : 0) - gap;
    const plotWidth = width - left;
    return (
      <>
        {values.map(([at, label], index) => {
          const y = at * plotHeight;
          const top = coerceIn(y - TICK_LINE_HEIGHT / 2, 0, plotHeight - TICK_LINE_HEIGHT);
          return (
            <g key={`v${index}`}>
              <line x1={left} y1={y} x2={width} y2={y} stroke={md('outlineVariant')} strokeWidth={1} />
              <text x={left - gap} y={top + TICK_LINE_HEIGHT / 2} textAnchor="end" dominantBaseline="central" fill={md('onSurfaceVariant')} fontSize={TICK_FONT_SIZE} fontFamily="ui-monospace, monospace">
                {label}
              </text>
            </g>
          );
        })}
        {times.map(([at, label], index) => {
          const x = left + at * plotWidth;
          const labelWidth = textWidth(label);
          const start = coerceIn(x - labelWidth / 2, left, width - labelWidth);
          return (
            <text key={`t${index}`} x={start} y={plotHeight + gap + TICK_LINE_HEIGHT / 2} dominantBaseline="central" fill={md('onSurfaceVariant')} fontSize={TICK_FONT_SIZE} fontFamily="ui-monospace, monospace">
              {label}
            </text>
          );
        })}
        {chart.plots.map((plot, index) => (
          <path
            key={`p${index}`}
            d={plot.points
              .map(([age, value], at) => {
                const [x, y] = chartPoint(age, value, chart.windowMs, low, high);
                return `${at === 0 ? 'M' : 'L'}${left + x * plotWidth},${y * plotHeight}`;
              })
              .join(' ')}
            fill="none"
            stroke={colours[plot.colour % colours.length]}
            strokeWidth={2}
          />
        ))}
      </>
    );
  })();
  return (
    <div ref={ref} className="w-full" style={{ height: CHART_HEIGHT }}>
      <svg width={width} height={CHART_HEIGHT} className="block overflow-visible" aria-hidden>
        {drawn}
      </svg>
    </div>
  );
};

export const InspectorChartPanel = ({ index, charts, className }: { index: number; charts: InspectorCharts; className?: string }) => {
  const colours = useSeriesColours();
  const chart = charts.charts[index];
  if (chart === undefined || chart.plots.length === 0) return null;
  return (
    <div className={`flex w-full flex-col px-3 ${className ?? ''}`}>
      <div className="flex gap-1">
        <Text typography="titleSmall" className="pt-3">{`Chart ${index + 1}`}</Text>
        <ChartChoice title="Scale" options={charts.timeScales} chosen={chart.rangeX} onChosen={(chosen) => void Qgc.invoke(CHART_RANGE_X, index, chosen)} />
        <ChartChoice title="Range" options={charts.ranges} chosen={chart.rangeY} onChosen={(chosen) => void Qgc.invoke(CHART_RANGE_Y, index, chosen)} />
      </div>
      <Surface className="mt-2 w-full" shape="large" color={md('surfaceContainer')}>
        <div className="flex flex-col gap-2 p-3">
          <ChartCanvas chart={chart} colours={colours} />
          <div className="flex w-full flex-wrap justify-center gap-x-4">
            {chart.plots.map((plot, at) => (
              <div key={at} className="flex items-center gap-1.5">
                <span className="block size-2 rounded-full" style={{ background: colours[plot.colour % colours.length] }} />
                <Text typography="labelMedium">{[plot.label, latestValue(plot.points)].filter((it) => it !== null).join('  ')}</Text>
              </div>
            ))}
          </div>
        </div>
      </Surface>
    </div>
  );
};
