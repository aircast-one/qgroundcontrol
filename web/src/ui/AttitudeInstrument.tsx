import { useContext, useId, useMemo } from 'react';
import { type Json, isNull, optBoolean, optDouble, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { useValue } from '@/kotlin/State';
import { Checkbox, ListItem } from '@/m3';
import { Color, aircast, argb, md, withAlpha } from '@/theme/Colors';
import { LocalFlyScreenState } from '@/ui/FlyScreenState';
import { ATTITUDE_KEY, ATTITUDE_NAME } from '@/ui/OverlayLayout';
import { SectionHeader } from '@/ui/SetupRow';

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

const SKY = argb(0xff3a78c4);
const GROUND = argb(0xff4f8a3c);
const WINGS = argb(0xffff6b2c);
const LADDER_STEP_DEGREES = 10;
export const ATTITUDE_LADDER: readonly number[] = Array.from({ length: 19 }, (_, rung) => (rung - 9) * LADDER_STEP_DEGREES).filter((degrees) => degrees !== 0);
const WING_SPAN = 0.42;
const WING_SHOULDER = 0.35;
const WING_DIP = 0.25;
const LADDER_RUNG_HALF = 0.18;
const LADDER_LABEL_AT = 0.24;
const LADDER_FONT = 8;
const ROLL_POINTER_INSET = 3;
const ROLL_POINTER_RADIUS = 2.5;
const ROLL_TICK_MAJOR = 7;
const ROLL_TICK_MINOR = 4;
const ROLL_TICKS = [-60, -45, -30, -20, -10, 0, 10, 20, 30, 45, 60];

export const OsdAttitudeDial = ({ size, className }: { size: number; className?: string }) => {
  const view = useQgcPath(ATTITUDE_PATH);
  const reading = useMemo(() => attitude(view) ?? NO_VEHICLE_ATTITUDE, [view]);
  const clip = `attitude-${useId().replace(/[^a-zA-Z0-9_-]/g, '')}`;
  const white = Color.White;
  const radius = size / 2;
  const center = { x: radius, y: radius };
  const horizon = radius * HORIZON_FRACTION;
  const step = pitchOffset(LADDER_STEP_DEGREES, horizon);
  const ring = withAlpha(white, OSD_DIAL_RING_ALPHA);
  const wing = radius * WING_SPAN;
  const pointer = pointOnRing(center, radius - ROLL_POINTER_INSET, -reading.roll);
  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      role="img"
      aria-label={`Attitude, pitch ${Math.round(reading.pitch)}°, roll ${Math.round(reading.roll)}°`}
      className={`shrink-0 overflow-visible ${className ?? ''}`}
    >
      <defs>
        <clipPath id={clip}>
          <circle cx={center.x} cy={center.y} r={radius} />
        </clipPath>
      </defs>
      <g clipPath={`url(#${clip})`}>
        <g transform={`rotate(${-reading.roll} ${center.x} ${center.y}) translate(0 ${pitchOffset(reading.pitch, horizon)})`}>
          <rect x={-size} y={center.y - size * 2} width={size * 3} height={size * 2} style={{ fill: SKY }} />
          <rect x={-size} y={center.y} width={size * 3} height={size * 2} style={{ fill: GROUND }} />
          <line x1={-size} y1={center.y} x2={size * 2} y2={center.y} strokeWidth={1.5} style={{ stroke: white }} />
          {ATTITUDE_LADDER.map((degrees) => (
            <g key={degrees}>
              <line x1={center.x - radius * LADDER_RUNG_HALF} y1={center.y - (degrees / LADDER_STEP_DEGREES) * step} x2={center.x + radius * LADDER_RUNG_HALF} y2={center.y - (degrees / LADDER_STEP_DEGREES) * step} strokeWidth={1} style={{ stroke: white }} />
              <text x={center.x + radius * LADDER_LABEL_AT} y={center.y - (degrees / LADDER_STEP_DEGREES) * step} dominantBaseline="central" fontSize={LADDER_FONT} style={{ fill: white }}>
                {Math.abs(degrees)}
              </text>
            </g>
          ))}
        </g>
      </g>
      {ROLL_TICKS.map((degrees) => {
        const from = pointOnRing(center, radius - 1, degrees);
        const to = pointOnRing(center, radius - (degrees % 30 === 0 ? ROLL_TICK_MAJOR : ROLL_TICK_MINOR), degrees);
        return <line key={degrees} x1={from.x} y1={from.y} x2={to.x} y2={to.y} strokeWidth={1} style={{ stroke: white }} />;
      })}
      <circle cx={center.x} cy={center.y} r={radius - 0.75} fill="none" strokeWidth={1.5} style={{ stroke: ring }} />
      <circle cx={pointer.x} cy={pointer.y} r={ROLL_POINTER_RADIUS} style={{ fill: WINGS }} />
      <path
        d={`M ${center.x - wing} ${center.y} L ${center.x - wing * WING_SHOULDER} ${center.y} L ${center.x} ${center.y + wing * WING_DIP} L ${center.x + wing * WING_SHOULDER} ${center.y} L ${center.x + wing} ${center.y}`}
        fill="none"
        strokeWidth={3}
        strokeLinejoin="round"
        style={{ stroke: WINGS }}
      />
    </svg>
  );
};

export const ATTITUDE_GAUGE_SIZE = 104;

export const AttitudeGauges = () => (
  <div className="flex items-center gap-2">
    <OsdAttitudeDial size={ATTITUDE_GAUGE_SIZE} />
    <OsdCompassDial size={ATTITUDE_GAUGE_SIZE} />
  </div>
);

export const AttitudeChoice = () => {
  const layout = useContext(LocalFlyScreenState).layout;
  const shown = !useValue(layout.hidden).has(ATTITUDE_KEY);
  return (
    <>
      <SectionHeader text="Instruments" />
      <ListItem
        headlineContent={ATTITUDE_NAME}
        leadingContent={<Checkbox checked={shown} onCheckedChange={null} />}
        containerColor="transparent"
        onClick={() => layout.setHidden(ATTITUDE_KEY, shown)}
      />
    </>
  );
};
