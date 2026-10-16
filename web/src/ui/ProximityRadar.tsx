import { useEffect, useMemo, useRef, useState } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optString } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { Color } from '@/theme/Colors';

export const PROXIMITY_VIEW = 'view.proximityRadar';
const SECTOR_SWEEP_DEG = 45;
const SECTOR_GAP_DEG = 360 / 100;
const ARC_START_OFFSET_DEG = -90 - 22.5;
const RADAR_COLOUR = 'rgb(255 0 0 / 0.5)';

export type RadarSector = { bearing: number; meters: number | null; text: string };

export const RadarSector = (bearing: number, meters: number | null, text: string): RadarSector => ({ bearing, meters, text });

export type ProximityRadar = { range: number; sectors: readonly RadarSector[] };

export const proximityRadar = (view: Json | null): ProximityRadar | null =>
  view === null || !optBoolean(view, 'shown')
    ? null
    : {
        range: optDouble(view, 'rangeMeters', 6.0),
        sectors: jsonObjects(optJSONArray(view, 'sectors')).map((sector) => RadarSector(optInt(sector, 'bearing'), isNull(sector, 'meters') ? null : optDouble(sector, 'meters'), optString(sector, 'text'))),
      };

const ellipseArc = (cx: number, cy: number, rx: number, ry: number, startDegrees: number, sweepDegrees: number): string => {
  const at = (degrees: number) => {
    const radians = (degrees * Math.PI) / 180;
    return `${cx + rx * Math.cos(radians)} ${cy + ry * Math.sin(radians)}`;
  };
  return `M ${at(startDegrees)} A ${rx} ${ry} 0 ${sweepDegrees > 180 ? 1 : 0} 1 ${at(startDegrees + sweepDegrees)}`;
};

const useElementSize = () => {
  const ref = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const element = ref.current;
    if (element === null) return undefined;
    const observer = new ResizeObserver(([entry]) => entry && setSize({ width: entry.contentRect.width, height: entry.contentRect.height }));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return [ref, size] as const;
};

export const ProximityRadarOverlay = ({ className }: { className?: string }) => {
  const json = useQgcPath(PROXIMITY_VIEW);
  const radar = useMemo(() => proximityRadar(json), [json]);
  return radar === null ? null : <RadarRings radar={radar} className={className} />;
};

const RadarRings = ({ radar, className }: { radar: ProximityRadar; className?: string }) => {
  const [ref, { width, height }] = useElementSize();
  const shortest = Math.min(width, height);
  const ratio = shortest / 2 / radar.range;
  const scaleX = shortest === 0 ? 1 : width / shortest;
  const scaleY = shortest === 0 ? 1 : height / shortest;
  const center = { x: width / 2, y: height / 2 };
  const placed = radar.sectors.flatMap((sector) => (sector.meters === null ? [] : [{ sector, meters: sector.meters }]));
  return (
    <div ref={ref} className={`pointer-events-none relative ${className ?? ''}`}>
      {shortest > 0 ? (
        <>
          <svg width={width} height={height} className="absolute inset-0" aria-hidden>
            {placed.map(({ sector, meters }, index) => {
              const radius = meters * ratio;
              return (
                <path
                  key={index}
                  d={ellipseArc(center.x, center.y, radius * scaleX, radius * scaleY, ARC_START_OFFSET_DEG + sector.bearing + SECTOR_GAP_DEG, SECTOR_SWEEP_DEG - 2 * SECTOR_GAP_DEG)}
                  fill="none"
                  stroke={RADAR_COLOUR}
                  strokeWidth={width / 100}
                />
              );
            })}
          </svg>
          {placed.map(({ sector, meters }, index) => {
            const angle = -Math.PI / 2 + (Math.PI / 180) * sector.bearing;
            const x = Math.cos(angle) * meters * ratio * scaleX;
            const y = Math.sin(angle) * meters * ratio * scaleY;
            return (
              <Text
                key={index}
                typography="labelSmall"
                fontWeight="bold"
                color={Color.White}
                className="absolute left-1/2 top-1/2 whitespace-nowrap"
                style={{ transform: `translate(-50%, -50%) translate(${x}px, ${y}px)` }}
              >
                {sector.text}
              </Text>
            );
          })}
        </>
      ) : null}
    </div>
  );
};
