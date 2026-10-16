import { useId, useMemo } from 'react';
import { type Json, isNull, optBoolean, optDouble, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Color, aircast, argb, md, withAlpha } from '@/theme/Colors';

export const ATTITUDE_PATH = 'view.attitude';
const HORIZON_FRACTION = 0.72;
const PITCH_SPAN_DEGREES = 45;

export type Attitude = {
  roll: number;
  pitch: number;
  heading: number;
  headingText: string;
  courseOverGround: number | null;
  headingToHome: number | null;
  headingToNextWaypoint: number | null;
  noseUp: boolean;
  homeBearing: number | null;
  pilotBearing: number | null;
};

export const Attitude = (fields: Omit<Attitude, 'homeBearing' | 'pilotBearing'> & Partial<Pick<Attitude, 'homeBearing' | 'pilotBearing'>>): Attitude => ({
  homeBearing: null,
  pilotBearing: null,
  ...fields,
});

const optAngle = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

export const NO_VEHICLE_ATTITUDE = Attitude({ roll: 0, pitch: 0, heading: 0, headingText: '', courseOverGround: null, headingToHome: null, headingToNextWaypoint: null, noseUp: false });

export const attitude = (view: Json | null): Attitude | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : Attitude({
        roll: optAngle(view, 'roll') ?? 0,
        pitch: optAngle(view, 'pitch') ?? 0,
        heading: optAngle(view, 'heading') ?? 0,
        headingText: optText(view, 'headingText'),
        courseOverGround: optAngle(view, 'courseOverGround'),
        headingToHome: optAngle(view, 'headingToHome'),
        headingToNextWaypoint: optAngle(view, 'headingToNextWaypoint'),
        noseUp: optBoolean(view, 'noseUp'),
        homeBearing: optAngle(view, 'homeBearing'),
        pilotBearing: optAngle(view, 'pilotBearing'),
      });

export const pitchOffset = (pitch: number, horizonRadius: number): number => (pitch * (horizonRadius * 2)) / PITCH_SPAN_DEGREES;

type Offset = { x: number; y: number };

const pointOnRing = (center: Offset, radius: number, degrees: number): Offset => {
  const radians = (degrees * Math.PI) / 180;
  return { x: center.x + radius * Math.sin(radians), y: center.y - radius * Math.cos(radians) };
};

const OSD_DIAL_SCRIM = 0.4;
const OSD_DIAL_RING_ALPHA = 0.7;
const OSD_DIAL_HORIZON_ALPHA = 0.45;
const OSD_CHEVRON_TIP = 0.42;
const OSD_CHEVRON_WING = 0.3;
const OSD_CHEVRON_NOTCH = 0.14;
const OSD_NORTH = argb(0xffff4d4d);
const ROOMY_DIAL = 96;
const TICKS = Array.from({ length: 12 }, (_, index) => index * 30);

export const OsdCompassDial = ({ size, className }: { size: number; className?: string }) => {
  const view = useQgcPath(ATTITUDE_PATH);
  const reading = useMemo(() => attitude(view) ?? NO_VEHICLE_ATTITUDE, [view]);
  const clip = `dial-${useId().replace(/[^a-zA-Z0-9_-]/g, '')}`;
  const white = Color.White;
  const roomy = size >= ROOMY_DIAL;
  const radius = size / 2;
  const center = { x: radius, y: radius };
  const ring = withAlpha(white, OSD_DIAL_RING_ALPHA);
  const scrim = withAlpha(Color.Black, OSD_DIAL_SCRIM);
  const north = pointOnRing(center, radius - 15, -reading.heading);
  const homeBearing = reading.homeBearing ?? reading.headingToHome;
  const home = homeBearing === null ? null : pointOnRing(center, radius - 8, homeBearing - reading.heading);
  const pilot = reading.pilotBearing === null ? null : pointOnRing(center, radius - 8, reading.pilotBearing - reading.heading);
  const chevron = [
    `M ${center.x} ${center.y - radius * OSD_CHEVRON_TIP}`,
    `L ${center.x + radius * OSD_CHEVRON_WING} ${center.y + radius * OSD_CHEVRON_WING}`,
    `L ${center.x} ${center.y + radius * OSD_CHEVRON_NOTCH}`,
    `L ${center.x - radius * OSD_CHEVRON_WING} ${center.y + radius * OSD_CHEVRON_WING}`,
    'Z',
  ].join(' ');
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`Heading ${reading.headingText}`} className={`shrink-0 overflow-visible ${className ?? ''}`}>
      <defs>
        <clipPath id={clip}>
          <circle cx={center.x} cy={center.y} r={radius} />
        </clipPath>
      </defs>
      <circle cx={center.x} cy={center.y} r={radius} style={{ fill: scrim }} />
      <g clipPath={`url(#${clip})`}>
        <g transform={`rotate(${-reading.roll} ${center.x} ${center.y}) translate(0 ${pitchOffset(reading.pitch, radius * HORIZON_FRACTION)})`}>
          <line x1={center.x - radius} y1={center.y} x2={center.x + radius} y2={center.y} strokeWidth={1.5} style={{ stroke: withAlpha(white, OSD_DIAL_HORIZON_ALPHA) }} />
        </g>
      </g>
      <circle cx={center.x} cy={center.y} r={radius - 1} fill="none" strokeWidth={1.5} style={{ stroke: ring }} />
      {TICKS.map((degrees) => {
        const from = pointOnRing(center, radius - 2, degrees - reading.heading);
        const to = pointOnRing(center, radius - (degrees % 90 === 0 ? 8 : 5), degrees - reading.heading);
        return <line key={degrees} x1={from.x} y1={from.y} x2={to.x} y2={to.y} strokeWidth={1.5} style={{ stroke: ring }} />;
      })}
      {pilot !== null ? (
        <>
          <circle cx={pilot.x} cy={pilot.y} r={5} style={{ fill: white }} />
          <circle cx={pilot.x} cy={pilot.y} r={3.5} style={{ fill: md('primary') }} />
        </>
      ) : null}
      <text x={north.x} y={north.y} textAnchor="middle" dominantBaseline="central" fontSize={11} fontWeight="bold" style={{ fill: OSD_NORTH }}>
        N
      </text>
      {home !== null ? (
        <>
          <circle cx={home.x} cy={home.y} r={7} style={{ fill: aircast('success') }} />
          <text x={home.x} y={home.y} textAnchor="middle" dominantBaseline="central" fontSize={9} fontWeight="bold" style={{ fill: aircast('onSuccess') }}>
            H
          </text>
        </>
      ) : null}
      <path d={chevron} style={{ fill: white }} />
      <path d={chevron} fill="none" strokeWidth={1} style={{ stroke: scrim }} />
      {roomy && reading.headingText.trim() !== '' ? (
        <text x={center.x} y={center.y + radius * 0.5} textAnchor="middle" dominantBaseline="hanging" fontSize={11} fontWeight="bold" style={{ fill: white }}>
          {reading.headingText}
        </text>
      ) : null}
    </svg>
  );
};
