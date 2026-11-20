import { useContext, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcDerived } from '@/bridge/QgcState';
import type { SheetAnchor } from '@/m3';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { ESC_PATH, EscStatusPanel, type Level, level } from '@/ui/EscIndicator';
import { STALE_OPACITY, useStaleNotice } from '@/ui/FlyState';
import { LocalCompactHud, OSD_SCRIM, osdShadow, useOsdBackdrop } from '@/ui/Osd';

export type RotorState = 'reporting' | 'lost' | 'none';

export type MotorLoad = {
  motor: number;
  angle: number;
  state: RotorState;
  current: string;
  rpm: string;
  temperature: string;
  temperatureLevel: Level;
  currentLevel: Level;
};

type Frame = { readonly size: number; readonly arm: number; readonly ring: number; readonly rpm: boolean };

const FULL_FRAME: Frame = { size: 140, arm: 48, ring: 20, rpm: true };
const COMPACT_FRAME: Frame = { size: 88, arm: 30, ring: 12, rpm: false };
const RING_WIDTH = 2;
const BODY = 6;
const PANEL_WIDTH = 320;
const PANEL_GAP = 8;

const rotorState = (json: Json): RotorState => {
  const text = optText(json, 'state');
  return text === 'reporting' || text === 'lost' ? text : 'none';
};

export const motorLoads = (view: Json | null): readonly MotorLoad[] =>
  view !== null && optBoolean(view, 'shown')
    ? jsonObjects(optJSONArray(view, 'frame')).map((motor) => ({
        motor: optInt(motor, 'motor'),
        angle: optDouble(motor, 'angle', 0),
        state: rotorState(motor),
        current: optText(motor, 'current'),
        rpm: optText(motor, 'rpm'),
        temperature: optText(motor, 'temperature'),
        temperatureLevel: level(motor, 'temperatureLevel'),
        currentLevel: level(motor, 'currentLevel'),
      }))
    : [];

export const besideWidget = (rect: { left: number; right: number; top: number }, viewportWidth: number): SheetAnchor => ({
  x: rect.right + PANEL_GAP + PANEL_WIDTH <= viewportWidth ? rect.right + PANEL_GAP : rect.left - PANEL_GAP - PANEL_WIDTH,
  y: rect.top,
});

const ringColour = (load: Level): string => (load === 'critical' ? md('error') : aircast('warning'));

const labelColour = (load: Level): string => (load === 'critical' ? md('error') : load === 'elevated' ? aircast('warning') : Color.White);

const place = (frame: Frame, angle: number, reach: number = frame.arm): readonly [number, number] => {
  const radians = (angle * Math.PI) / 180;
  const centre = frame.size / 2;
  return [centre + reach * Math.sin(radians), centre - reach * Math.cos(radians)];
};

const compactAmps = (current: string): string => (Number(current) >= 10 ? Math.round(Number(current)).toString() : current);

const rotorTitle = (load: MotorLoad): string =>
  load.state === 'reporting' ? `Motor ${load.motor}: ${load.current} A, ${load.rpm} rpm, ${load.temperature}` : load.state === 'lost' ? `Motor ${load.motor}: ESC telemetry lost` : `Motor ${load.motor}: no ESC data`;

const Unit = ({ children }: { children: string }) => (
  <tspan fontSize="0.75em" fontWeight={400}>
    {children}
  </tspan>
);

const Rotor = ({ frame, load }: { frame: Frame; load: MotorLoad }) => {
  const [x, y] = place(frame, load.angle);
  const lost = load.state === 'lost';
  return (
    <g data-motor={load.motor}>
      <title>{rotorTitle(load)}</title>
      <circle
        cx={x}
        cy={y}
        r={frame.ring}
        fill={useOsdBackdrop(OSD_SCRIM)}
        stroke={lost ? md('error') : withAlpha(Color.White, 0.3)}
        strokeWidth={RING_WIDTH}
        strokeDasharray={load.state === 'reporting' ? undefined : '3 3'}
      />
      {load.state === 'reporting' && load.temperatureLevel !== 'normal' ? (
        <circle cx={x} cy={y} r={frame.ring} fill="none" stroke={ringColour(load.temperatureLevel)} strokeWidth={RING_WIDTH} />
      ) : null}
      {load.state === 'reporting' && frame.rpm ? (
        <>
          <text x={x} y={y} dy="-0.1em" textAnchor="middle" fontSize={12} fontWeight={600} fill={labelColour(load.currentLevel)}>
            {load.current}
            <Unit>A</Unit>
          </text>
          <text x={x} y={y} dy="1.15em" textAnchor="middle" fontSize={10} fill={withAlpha(Color.White, 0.85)}>
            {load.rpm}
            <Unit>{' rpm'}</Unit>
          </text>
        </>
      ) : load.state === 'reporting' ? (
        <text x={x} y={y} dy="0.35em" textAnchor="middle" fontSize={11} fontWeight={600} fill={labelColour(load.currentLevel)}>
          {compactAmps(load.current)}
          <Unit>A</Unit>
        </text>
      ) : (
        <text x={x} y={y} dy="0.35em" textAnchor="middle" fontSize={11} fontWeight={600} fill={lost ? md('error') : withAlpha(Color.White, 0.5)}>
          –
        </text>
      )}
    </g>
  );
};

type Opened = { readonly anchor: SheetAnchor; readonly focus: number | null };

export const MotorLoadWidget = () => {
  const loads = useQgcDerived(ESC_PATH, motorLoads);
  const stale = useStaleNotice() !== '';
  const frame = useContext(LocalCompactHud) ? COMPACT_FRAME : FULL_FRAME;
  const [opened, setOpened] = useState<Opened | null>(null);
  if (loads.length === 0) return null;
  const centre = frame.size / 2;
  return (
    <>
      <button
        type="button"
        aria-label="Motor load: open ESC status"
        aria-expanded={opened !== null}
        style={{ opacity: stale ? STALE_OPACITY : 1 }}
        onClick={(event) => {
          const tapped = (event.target as Element).closest('[data-motor]')?.getAttribute('data-motor') ?? null;
          const next = { anchor: besideWidget(event.currentTarget.getBoundingClientRect(), window.innerWidth), focus: tapped === null ? null : Number(tapped) };
          setOpened((was) => (was !== null && was.focus === next.focus ? null : next));
        }}
      >
        <svg width={frame.size} height={frame.size} viewBox={`0 0 ${frame.size} ${frame.size}`} role="img" aria-label="Motor load" className={osdShadow()}>
          {loads.map((load) => {
            const [x1, y1] = place(frame, load.angle, BODY);
            const [x2, y2] = place(frame, load.angle, frame.arm - frame.ring - RING_WIDTH / 2);
            return <line key={load.motor} x1={x1} y1={y1} x2={x2} y2={y2} stroke={withAlpha(Color.White, 0.5)} strokeWidth={1.5} />;
          })}
          {loads.map((load) => (
            <Rotor key={load.motor} frame={frame} load={load} />
          ))}
          <circle cx={centre} cy={centre} r={BODY} fill="none" stroke={withAlpha(Color.White, 0.8)} strokeWidth={1.5} />
          <path d={`M ${centre} ${centre - BODY - 8} l -4 6 h 8 z`} fill={withAlpha(Color.White, 0.8)} />
        </svg>
      </button>
      {opened !== null ? <EscStatusPanel anchor={opened.anchor} focus={opened.focus} onClose={() => setOpened(null)} /> : null}
    </>
  );
};
