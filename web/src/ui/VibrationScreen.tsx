import { useMemo } from 'react';
import { type Json, isNull, optBoolean, optDouble, optJSONArray, optString, optText, arrayJSONObject } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { format } from '@/kotlin/Std';
import { HorizontalDivider, Surface, Text } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';

export const VIBRATION_VIEW = 'view.vibration';
const SCALE_STEPS = 4;

const AXIS_WIDTH = 32;
const BAR_WIDTH = 56;

export type VibrationAxis = { axis: string; value: number | null; fraction: number; severity: string | null };

export type VibrationReading = {
  units: string;
  scaleMaximum: number;
  warningLevel: number;
  dangerLevel: number;
  axes: readonly VibrationAxis[];
  clipCounts: readonly number[];
};

const doubleOrNull = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

const stringOrNull = (json: Json, key: string): string | null => {
  if (isNull(json, key)) return null;
  const value = optString(json, key);
  return value.trim() === '' ? null : value;
};

export type SilentState = { title: string; body: string };

export const silentState = (view: Json | null): SilentState | null => {
  const token = optText(view, 'silentReason');
  if (view === null) return { title: 'No vehicle connected', body: CONNECT_PROMPT };
  if (token.trim() === '') return null;
  const silentText = optText(view, 'silentText');
  const title = silentText.trim() === '' ? 'Vibration is not being reported' : silentText;
  const body =
    token === 'noVehicle'
      ? CONNECT_PROMPT
      : token === 'notReported'
        ? 'The autopilot has not sent a VIBRATION message. Not all firmware and airframes publish one.'
        : 'Nothing has reported a vibration level on this vehicle.';
  return { title, body };
};

export const PARTIAL_TITLE = 'Only some vibration axes are reported';
export const PARTIAL_BODY = 'This screen draws all three axes together, and this vehicle is reporting some of them.';

const CONNECT_PROMPT = 'Connect a vehicle from the Fly view to see its vibration levels.';

const arrayInt = (value: unknown): number => {
  const number = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
  return Number.isFinite(number) ? Math.trunc(number) : 0;
};

export const vibrationReading = (view: Json | null): VibrationReading | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const axes = optJSONArray(view, 'axes');
  if (axes === null) return null;
  return {
    units: optText(view, 'units'),
    scaleMaximum: optDouble(view, 'scaleMaximum', 0.0),
    warningLevel: optDouble(view, 'warningLevel', 0.0),
    dangerLevel: optDouble(view, 'dangerLevel', 0.0),
    axes: axes.flatMap((_, index) => {
      const axis = arrayJSONObject(axes, index);
      if (axis === null) return [];
      const fraction = doubleOrNull(axis, 'fraction');
      return [
        {
          axis: stringOrNull(axis, 'label') ?? optText(axis, 'axis').toUpperCase(),
          value: doubleOrNull(axis, 'value'),
          fraction: fraction === null ? 0 : Math.fround(fraction),
          severity: stringOrNull(axis, 'severity'),
        },
      ];
    }),
    clipCounts: (optJSONArray(view, 'clipCounts') ?? []).map(arrayInt),
  };
};

export const vibrationGlance = (reading: VibrationReading): string | null => {
  const parts = reading.axes.flatMap((axis) => (axis.value !== null && Number.isFinite(axis.value) ? [`${axis.axis} ${Math.round(axis.value)}`] : []));
  if (parts.length === 0) return null;
  return parts.join(' · ') + (reading.units.trim() !== '' ? ` ${reading.units}` : '');
};

export const worstSeverity = (reading: VibrationReading): string | null => ['danger', 'warning', 'normal'].find((level) => reading.axes.some((it) => it.severity === level)) ?? null;

export const vibrationVerdict = (reading: VibrationReading): string => {
  const worst = worstSeverity(reading);
  const axes = reading.axes
    .filter((it) => it.severity === worst)
    .map((it) => it.axis)
    .join(' and ');
  const level =
    worst === 'danger'
      ? `Vibration on ${axes} is over the unsafe limit of ${Math.trunc(reading.dangerLevel)}.`
      : worst === 'warning'
        ? `Vibration on ${axes} is above ${Math.trunc(reading.warningLevel)}; watch it.`
        : 'Vibration is well under the limit.';
  const clips = reading.clipCounts.reduce((sum, it) => sum + it, 0);
  const clipping = clips === 0 ? 'No clipping.' : `The accelerometers clipped ${clips} times; expect zero in flight.`;
  return `${level} ${clipping}`;
};

export const severityLabel = (severity: string | null): string => {
  switch (severity) {
    case 'danger':
      return 'Unsafe';
    case 'warning':
      return 'Watch';
    case 'normal':
      return 'Healthy';
    default:
      return '';
  }
};

export const vibrationHeading = (units: string): string => (units.trim() === '' ? 'Vibration' : `Vibration (${units})`);

export const bandCaption = (warningLevel: number, dangerLevel: number): string => {
  const warn = Math.trunc(warningLevel);
  const danger = Math.trunc(dangerLevel);
  return `Under ${warn} healthy · ${warn}-${danger} watch · over ${danger} unsafe`;
};

export const scaleLabels = (scaleMaximum: number, warningLevel: number, dangerLevel: number): readonly string[] =>
  [scaleMaximum, dangerLevel, warningLevel, 0.0].map((it) => String(Math.trunc(it)));

const colorFor = (severity: string | null): string => {
  switch (severity) {
    case 'danger':
      return md('error');
    case 'warning':
      return aircast('warning');
    case 'normal':
      return md('primary');
    default:
      return md('surfaceVariant');
  }
};

const ScaleAxis = ({ labels, className }: { labels: readonly string[]; className?: string }) => (
  <div className={`flex shrink-0 flex-col items-end justify-between ${className ?? ''}`} style={{ width: AXIS_WIDTH }}>
    {labels.map((label, index) => (
      <Text key={index} typography="labelSmall">
        {label}
      </Text>
    ))}
  </div>
);

const VibrationBarGraphic = ({ axis, className }: { axis: VibrationAxis; className?: string }) => (
  <div className={`flex items-end justify-center ${className ?? ''}`}>
    <div className="relative h-full overflow-hidden rounded-[6px]" style={{ width: BAR_WIDTH, background: md('surfaceVariant') }}>
      <div className="absolute inset-x-0 bottom-0" style={{ height: `${Math.min(1, Math.max(0, axis.fraction)) * 100}%`, background: colorFor(axis.severity) }} />
      <div className="absolute inset-0 flex flex-col justify-between">
        {Array.from({ length: SCALE_STEPS }, (_, index) => (
          <HorizontalDivider key={index} color={md('onSurfaceVariant')} />
        ))}
      </div>
    </div>
  </div>
);

const VibrationReadout = ({ axis, className }: { axis: VibrationAxis; className?: string }) => (
  <div className={`flex flex-col items-center ${className ?? ''}`}>
    <Text typography="titleMedium">{axis.value !== null ? format('%.1f', axis.value) : '--'}</Text>
    <Text typography="labelLarge">{axis.axis}</Text>
    <Text typography="labelMedium">{severityLabel(axis.severity)}</Text>
  </div>
);

const EmptyState = ({ message, detail, className }: { message: string; detail: string; className?: string }) => (
  <div className={`flex h-full w-full flex-col items-center gap-2 p-6 ${className ?? ''}`}>
    <Text typography="titleMedium" textAlign="center">
      {sentenceCase(message)}
    </Text>
    <Text typography="bodyMedium" textAlign="center">
      {detail}
    </Text>
  </div>
);

export const VibrationScreen = ({ className }: { className?: string }) => {
  const view = useQgcPath(VIBRATION_VIEW);
  const reading = useMemo(() => vibrationReading(view), [view]);
  const empty = useMemo(() => vibrationEmptyState(view, reading), [view, reading]);
  if (empty !== null) return <EmptyState message={empty.title} detail={empty.body} className={className} />;
  if (reading !== null) return <VibrationBody reading={reading} className={className} />;
  return null;
};

export const vibrationCaveat = (view: Json | null): string | null => {
  if (view === null || optBoolean(view, 'available')) return null;
  if (optText(view, 'silentReason') === 'notReported') return 'This vehicle is not reporting vibration';
  if (optText(view, 'silentReason').trim() === '') return 'This vehicle is reporting only some vibration axes';
  return null;
};

export const vibrationEmptyState = (view: Json | null, reading: VibrationReading | null): SilentState | null => silentState(view) ?? (reading === null ? partlyReported(view) : null);

const partlyReported = (view: Json | null): SilentState =>
  optBoolean(view, 'connected') ? { title: PARTIAL_TITLE, body: PARTIAL_BODY } : { title: 'No vehicle connected', body: CONNECT_PROMPT };

const verdictLook = (worst: string | null): { color: string; contentColor: string; icon: 'ic_error' | 'ic_warning' | 'ic_check_circle' } => {
  if (worst === 'danger') return { color: md('errorContainer'), contentColor: md('onErrorContainer'), icon: 'ic_error' };
  if (worst === 'warning') return { color: aircast('warningContainer'), contentColor: aircast('warning'), icon: 'ic_warning' };
  return { color: aircast('successContainer'), contentColor: aircast('success'), icon: 'ic_check_circle' };
};

const VibrationBody = ({ reading, className }: { reading: VibrationReading; className?: string }) => {
  const look = verdictLook(worstSeverity(reading));
  return (
    <div className={`flex h-full w-full flex-col gap-3 p-4 ${className ?? ''}`}>
      <Surface className="flex min-h-0 w-full flex-1 flex-col" color={md('surfaceContainer')} shape="medium">
        <div className="flex min-h-0 flex-1 flex-col gap-3 p-4">
          <Text typography="titleSmall">{vibrationHeading(reading.units)}</Text>
          <div className="flex min-h-0 w-full flex-1">
            <ScaleAxis labels={scaleLabels(reading.scaleMaximum, reading.warningLevel, reading.dangerLevel)} className="h-full" />
            {reading.axes.map((axis, index) => (
              <VibrationBarGraphic key={index} axis={axis} className="h-full flex-1" />
            ))}
          </div>
          <div className="flex w-full">
            <span className="shrink-0" style={{ width: AXIS_WIDTH }} />
            {reading.axes.map((axis, index) => (
              <VibrationReadout key={index} axis={axis} className="flex-1" />
            ))}
          </div>
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {bandCaption(reading.warningLevel, reading.dangerLevel)}
          </Text>
        </div>
      </Surface>
      <Surface className="w-full" shape="medium" color={look.color} contentColor={look.contentColor}>
        <div className="flex items-center gap-3 p-4">
          <Drawable name={look.icon} className="shrink-0" />
          <Text typography="bodyMedium">{vibrationVerdict(reading)}</Text>
        </div>
      </Surface>
      <div className="flex w-full items-center">
        <Text typography="bodyLarge" className="flex-1">
          Clipping events
        </Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {clipText(reading.clipCounts)}
        </Text>
      </div>
    </div>
  );
};

export const clipText = (counts: readonly number[]): string => counts.map((count, index) => `Accel ${index + 1}: ${count}`).join(' · ');
