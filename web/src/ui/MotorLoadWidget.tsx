import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcDerived } from '@/bridge/QgcState';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { ESC_PATH } from '@/ui/EscIndicator';
import { useScreenSize } from '@/ui/FlyShell';
import { compactHud } from '@/ui/HudLadders';
import { osdShadow } from '@/ui/Osd';

export type Level = 'normal' | 'elevated' | 'critical';

export type MotorLoad = {
  motor: number;
  angle: number;
  reporting: boolean;
  current: string;
  rpm: string;
  temperature: string;
  temperatureLevel: Level;
  currentLevel: Level;
  fill: number;
};

type Frame = { readonly size: number; readonly arm: number; readonly ring: number; readonly rpm: boolean };

const FULL_FRAME: Frame = { size: 120, arm: 42, ring: 16, rpm: true };
const COMPACT_FRAME: Frame = { size: 88, arm: 30, ring: 12, rpm: false };
const RING_WIDTH = 2;
const BODY = 6;

const level = (json: Json, key: string): Level => {
  const text = optText(json, key);
  return text === 'critical' || text === 'elevated' ? text : 'normal';
};

export const motorLoads = (view: Json | null): readonly MotorLoad[] =>
  view !== null && optBoolean(view, 'shown')
    ? jsonObjects(optJSONArray(view, 'frame')).map((motor) => ({
        motor: optInt(motor, 'motor'),
        angle: optDouble(motor, 'angle', 0),
        reporting: optBoolean(motor, 'reporting'),
        current: optText(motor, 'current'),
        rpm: optText(motor, 'rpm'),
        temperature: optText(motor, 'temperature'),
        temperatureLevel: level(motor, 'temperatureLevel'),
        currentLevel: level(motor, 'currentLevel'),
        fill: optDouble(motor, 'fill', 0),
      }))
    : [];

const ringColour = (load: Level): string => (load === 'critical' ? md('error') : load === 'elevated' ? aircast('warning') : withAlpha(Color.White, 0.85));

const labelColour = (load: Level): string => (load === 'critical' ? md('error') : load === 'elevated' ? aircast('warning') : Color.White);

const place = (frame: Frame, angle: number, reach: number = frame.arm): readonly [number, number] => {
  const radians = (angle * Math.PI) / 180;
  const centre = frame.size / 2;
  return [centre + reach * Math.sin(radians), centre - reach * Math.cos(radians)];
};

const Rotor = ({ frame, load }: { frame: Frame; load: MotorLoad }) => {
  const [x, y] = place(frame, load.angle);
  const circumference = 2 * Math.PI * frame.ring;
  return (
    <g>
      <title>{load.reporting ? `Motor ${load.motor}: ${load.current} A, ${load.rpm} rpm, ${load.temperature}` : `Motor ${load.motor}: no ESC data`}</title>
      <circle cx={x} cy={y} r={frame.ring} fill="none" stroke={withAlpha(Color.White, 0.3)} strokeWidth={RING_WIDTH} strokeDasharray={load.reporting ? undefined : '3 3'} />
      {load.reporting ? (
        <circle
          cx={x}
          cy={y}
          r={frame.ring}
          fill="none"
          stroke={ringColour(load.temperatureLevel)}
          strokeWidth={RING_WIDTH}
          strokeLinecap="round"
          strokeDasharray={`${load.fill * circumference} ${circumference}`}
          transform={`rotate(-90 ${x} ${y})`}
        />
      ) : null}
      {load.reporting && frame.rpm ? (
        <>
          <text x={x} y={y} dy="-0.05em" textAnchor="middle" fontSize={11} fontWeight={600} fill={labelColour(load.currentLevel)}>
            {load.current}
          </text>
          <text x={x} y={y} dy="1.05em" textAnchor="middle" fontSize={7.5} fill={withAlpha(Color.White, 0.7)}>
            {load.rpm}
          </text>
        </>
      ) : load.reporting ? (
        <text x={x} y={y} dy="0.35em" textAnchor="middle" fontSize={10} fontWeight={600} fill={labelColour(load.currentLevel)}>
          {load.current}
        </text>
      ) : (
        <text x={x} y={y} dy="0.35em" textAnchor="middle" fontSize={11} fontWeight={600} fill={withAlpha(Color.White, 0.5)}>
          –
        </text>
      )}
    </g>
  );
};

export const MotorLoadWidget = () => {
  const loads = useQgcDerived(ESC_PATH, motorLoads);
  const frame = compactHud(useScreenSize().height) ? COMPACT_FRAME : FULL_FRAME;
  if (loads.length === 0) return null;
  const centre = frame.size / 2;
  return (
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
  );
};
