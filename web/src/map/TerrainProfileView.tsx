import { type MouseEvent, useEffect, useMemo, useRef, useState } from 'react';
import { type Json, has, isNull, optBoolean } from '@/bridge/Json';
import { coerceAtLeast, coerceIn } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { Color, argb, md, withAlpha } from '@/theme/Colors';
import { type ProfilePoint, type TerrainProfile, terrainWarning } from './TerrainProfile';

export type Offset = { readonly x: number; readonly y: number };

const TERRAIN_COLOUR = argb(0xff8d6e63);
const PLANNED_COLOUR = argb(0xff4fc3f7);
const COLLISION_COLOUR = Color.Red;
const MISSING_COLOUR = Color.Yellow;
const PATTERN_COLOUR = withAlpha(Color.Green, 0.5);
const MARKER_TAP_SLOP_PX = 48;
const FULL_TERRAIN = 0.98;
const CHART_HEIGHT = 110;
const SURFACE_HEIGHT = 126;
const CHART_TOP = 8;
const CHART_BOTTOM = 16;
const CHART_END = 8;
const LABEL_FONT = '500 11px "Roboto Variable", Roboto, system-ui, sans-serif';
const LABEL_HEIGHT = 16;

const pairs = <T,>(list: readonly T[]): readonly (readonly [T, T])[] =>
  list.flatMap((from, index) => {
    const to = list[index + 1];
    return to === undefined ? [] : [[from, to] as const];
  });

export const terrainRuns = (profile: TerrainProfile, width: number, height: number): readonly (readonly Offset[])[] => profileRuns(profile, width, height, (it) => it.terrain);

export const plannedRuns = (profile: TerrainProfile, width: number, height: number): readonly (readonly Offset[])[] =>
  profileRuns(profile, width, height, (it) => (Number.isFinite(it.planned) ? it.planned : null));

const profileRuns = (profile: TerrainProfile, width: number, height: number, value: (point: ProfilePoint) => number | null): readonly (readonly Offset[])[] => {
  const distance = profile.distance;
  if (!(distance > 0.0 && profile.drawable)) return [];
  return profile.points
    .reduce<readonly (readonly Offset[])[]>(
      (runs, point) => {
        const altitude = value(point);
        const last = runs.at(-1) ?? [];
        if (altitude === null) return last.length === 0 ? runs : [...runs, []];
        const at = { x: (point.distance / distance) * width, y: height - ((altitude - profile.lowest) / profile.span) * height };
        return [...runs.slice(0, -1), [...last, at]];
      },
      [[]],
    )
    .filter((it) => it.length >= 2);
};

export const missingSpans = (profile: TerrainProfile, width: number): readonly (readonly [number, number])[] => {
  const distance = profile.distance;
  if (!(distance > 0.0)) return [];
  return pairs(profile.points)
    .filter(([from, to]) => from.terrain === null || to.terrain === null)
    .map(([from, to]) => [(from.distance / distance) * width, (to.distance / distance) * width] as const);
};

export const collisionSegments = (profile: TerrainProfile, width: number, height: number): readonly (readonly [Offset, Offset])[] => {
  const distance = profile.distance;
  if (!(distance > 0.0)) return [];
  const at = (point: ProfilePoint): Offset => ({ x: (point.distance / distance) * width, y: height - ((point.planned - profile.lowest) / profile.span) * height });
  return pairs(profile.points)
    .filter(([from, to]) => from.collision && to.collision && Number.isFinite(from.planned) && Number.isFinite(to.planned))
    .map(([from, to]) => [at(from), at(to)] as const);
};

export const markerX = (distance: number, profile: TerrainProfile, width: number): number => (profile.distance > 0.0 ? (distance / profile.distance) * width : 0);

const gap = (x: number, tapX: number): number => {
  const apart = Math.abs(x - tapX);
  return Number.isNaN(apart) ? Infinity : apart;
};

export const tappedSequence = (profile: TerrainProfile, width: number, tapX: number): number | null => {
  const nearest = profile.markers
    .flatMap((marker) => [marker.distance, ...(marker.endDistance === null ? [] : [marker.endDistance])].map((it) => [markerX(it, profile, width), marker.sequence] as const))
    .reduce<readonly [number, number] | null>((best, candidate) => (best === null || gap(candidate[0], tapX) < gap(best[0], tapX) ? candidate : best), null);
  if (nearest !== null && gap(nearest[0], tapX) <= MARKER_TAP_SLOP_PX) return nearest[1];
  return (
    profile.markers.find((marker) => marker.endDistance !== null && tapX >= markerX(marker.distance, profile, width) && tapX <= markerX(marker.endDistance, profile, width))?.sequence ?? null
  );
};

export const groundOutline = (terrain: readonly Offset[], height: number): readonly Offset[] => {
  const first = terrain[0];
  const last = terrain.at(-1);
  return terrain.length < 2 || first === undefined || last === undefined ? [] : [...terrain, { x: last.x, y: height }, { x: first.x, y: height }];
};

const pathOf = (points: readonly Offset[], closed = false): string => points.map((it, index) => `${index === 0 ? 'M' : 'L'}${it.x} ${it.y}`).join(' ') + (closed ? ' Z' : '');

export const heightRange = (profile: TerrainProfile): string => (profile.flat ? `${profile.lowestText} AMSL` : `${profile.bandText} AMSL`);

const coverageNote = (profile: TerrainProfile): string => {
  if (!profile.hasTerrain) return ' · ground height unknown';
  if (profile.terrainCoverage <= 0.0) return ' · ground height at points, none along the route';
  if (profile.terrainCoverage < FULL_TERRAIN) return ` · ground height for ${coerceAtLeast(Math.trunc(profile.terrainCoverage * 100), 1)}% of the route`;
  return '';
};

export const profileLabel = (profile: TerrainProfile): string => {
  const warning = terrainWarning(profile.clearance);
  return `${warning === null ? '' : `${warning} `}${heightRange(profile)} · ${profile.distanceText}${coverageNote(profile)}`;
};

export const ELEVATION_PROVIDER = 'settings.flightMapSettings.elevationMapProvider.rawValue';
export const SHOW_MISSION_ITEM_STATUS = 'settings.planViewSettings.showMissionItemStatus';

export const missionItemStatusShown = (setting: Json | null): boolean => (setting !== null && has(setting, 'value') && !isNull(setting, 'value') ? optBoolean(setting, 'value', true) : true);

export const elevationCredit = (notice: string): string | null => (notice.trim() === '' ? null : `Powered by ${notice}`);

type TextWidth = (text: string) => number;

const useTextWidth = (): TextWidth =>
  useMemo(() => {
    const context = document.createElement('canvas').getContext('2d');
    return (text: string) => {
      if (context === null) return text.length * 6;
      context.font = LABEL_FONT;
      return context.measureText(text).width;
    };
  }, []);

const useWidth = () => {
  const box = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const element = box.current;
    if (element === null) return undefined;
    const observer = new ResizeObserver(([entry]) => setWidth(entry?.contentRect.width ?? 0));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return [box, width] as const;
};

const Label = ({ x, y, text, fill, anchor = 'start', baseline = 'hanging' }: { x: number; y: number; text: string; fill: string; anchor?: 'start' | 'middle' | 'end'; baseline?: 'hanging' | 'central' }) => (
  <text x={x} y={y} textAnchor={anchor} dominantBaseline={baseline} style={{ font: LABEL_FONT, fill }}>
    {text}
  </text>
);

const Chart = ({
  profile,
  width,
  labelMargin,
  textWidth,
  selectedSequence,
  onSelect,
}: {
  profile: TerrainProfile;
  width: number;
  labelMargin: number;
  textWidth: TextWidth;
  selectedSequence: number | null;
  onSelect: (sequence: number) => void;
}) => {
  const w = coerceAtLeast(width - labelMargin - CHART_END, 0);
  const h = CHART_HEIGHT - CHART_TOP - CHART_BOTTOM;
  const ink = md('onSurface');
  const accent = md('primary');
  const onAccent = md('onPrimary');
  const paper = md('surface');
  const tap = (event: MouseEvent<SVGRectElement>) => {
    const picked = tappedSequence(profile, w, event.clientX - event.currentTarget.getBoundingClientRect().left);
    if (picked !== null) onSelect(picked);
  };
  return (
    <g transform={`translate(${labelMargin} ${CHART_TOP})`}>
      {terrainRuns(profile, w, h).map((run, index) => (
        <g key={`terrain${index}`}>
          <path d={pathOf(groundOutline(run, h), true)} style={{ fill: withAlpha(TERRAIN_COLOUR, 0.45) }} />
          <path d={pathOf(run)} style={{ fill: 'none', stroke: TERRAIN_COLOUR, strokeWidth: 3 }} />
        </g>
      ))}
      {missingSpans(profile, w).map(([from, to], index) => (
        <line key={`missing${index}`} x1={from} y1={h} x2={to} y2={h} style={{ stroke: MISSING_COLOUR, strokeWidth: 9 }} />
      ))}
      {profile.heightTicks.map((tick, index) => {
        const y = h - (h * index) / coerceAtLeast(profile.heightTicks.length - 1, 1);
        return (
          <g key={`height${index}`}>
            <line x1={0} y1={y} x2={w} y2={y} style={{ stroke: withAlpha(ink, 0.3), strokeWidth: 1 }} />
            <Label x={-4} y={y} text={tick} fill={ink} anchor="end" baseline="central" />
          </g>
        );
      })}
      {profile.distanceTicks.map((tick, index) => {
        const measured = textWidth(tick);
        const x = (w * index) / coerceAtLeast(profile.distanceTicks.length - 1, 1);
        return <Label key={`distance${index}`} x={coerceIn(x - measured / 2, 0, coerceAtLeast(w - measured, 0))} y={h} text={tick} fill={ink} />;
      })}
      {plannedRuns(profile, w, h).map((run, index) => (
        <path key={`planned${index}`} d={pathOf(run)} style={{ fill: 'none', stroke: PLANNED_COLOUR, strokeWidth: 3 }} />
      ))}
      {collisionSegments(profile, w, h).map(([from, to], index) => (
        <line key={`collision${index}`} x1={from.x} y1={from.y} x2={to.x} y2={to.y} style={{ stroke: COLLISION_COLOUR, strokeWidth: 9 }} />
      ))}
      {profile.markers.map((marker, index) => {
        const start = markerX(marker.distance, profile, w);
        const end = marker.endDistance === null ? null : markerX(marker.endDistance, profile, w);
        const labels = [[start, marker.label] as const, ...(end === null ? [] : [[end, String(marker.lastSequence ?? marker.sequence)] as const])];
        const chosen = marker.sequence === selectedSequence;
        return (
          <g key={`marker${index}`}>
            {end === null ? null : (
              <>
                <rect x={start} y={h - LABEL_HEIGHT} width={coerceAtLeast(end - start, 0)} height={LABEL_HEIGHT} style={{ fill: PATTERN_COLOUR }} />
                <Label x={(start + end - textWidth(marker.pattern)) / 2} y={h - LABEL_HEIGHT} text={marker.pattern} fill={ink} />
              </>
            )}
            {labels.map(([x, text], at) => {
              const radius = Math.max(textWidth(text), LABEL_HEIGHT) / 2 + 3;
              return (
                <g key={at}>
                  <line x1={x} y1={0} x2={x} y2={h} style={{ stroke: ink, strokeWidth: 1 }} />
                  <circle cx={x} cy={h - radius} r={radius} style={{ fill: chosen ? accent : withAlpha(ink, 0.8) }} />
                  <Label x={x} y={h - radius} text={text} fill={chosen ? onAccent : paper} anchor="middle" baseline="central" />
                </g>
              );
            })}
          </g>
        );
      })}
      <rect x={0} y={0} width={w} height={h} fill="transparent" onClick={tap} />
    </g>
  );
};

export const TerrainProfileView = ({
  profile,
  notice,
  className,
  selectedSequence = null,
  onSelect = () => undefined,
}: {
  profile: TerrainProfile;
  notice: string;
  className?: string;
  selectedSequence?: number | null;
  onSelect?: (sequence: number) => void;
}) => {
  const [box, width] = useWidth();
  const textWidth = useTextWidth();
  if (profile.points.length === 0) return null;
  if (!profile.drawable) {
    return (
      <Text typography="bodySmall" color={md('onSurfaceVariant')} className={`block w-full py-1 ${className ?? ''}`}>
        Plan a route with altitudes to see a profile.
      </Text>
    );
  }
  const credit = elevationCredit(notice);
  const labelMargin = profile.heightTicks.reduce((widest, tick) => Math.max(widest, textWidth(tick)), 0) + 8;
  return (
    <>
      <Text typography="labelSmall" className="block w-full px-2">
        {profileLabel(profile)}
      </Text>
      {profile.heightHeader.trim() !== '' ? (
        <div className="flex w-full px-2">
          <Text typography="labelSmall" className="flex-1">
            Elevation
          </Text>
          <Text typography="labelSmall">{profile.heightHeader}</Text>
        </div>
      ) : null}
      {credit === null ? null : (
        <Text typography="labelSmall" textAlign="center" className="w-full">
          {credit}
        </Text>
      )}
      <Surface color={withAlpha(md('surface'), 0.88)} className={`w-full ${className ?? ''}`} style={{ height: SURFACE_HEIGHT }}>
        <div ref={box} className="relative size-full">
          {width > 0 ? (
            <svg width={width} height={CHART_HEIGHT} className="block overflow-visible" role="img" aria-label={profileLabel(profile)}>
              <Chart profile={profile} width={width} labelMargin={labelMargin} textWidth={textWidth} selectedSequence={selectedSequence} onSelect={onSelect} />
            </svg>
          ) : null}
        </div>
      </Surface>
    </>
  );
};
