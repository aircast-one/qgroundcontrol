import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcDerived } from '@/bridge/QgcState';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { ESC_PATH } from '@/ui/EscIndicator';

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

const SIZE = 132;
const CENTRE = SIZE / 2;
const ARM = 44;
const RING = 16;
const RING_WIDTH = 3.5;
const BODY = 9;
const CIRCUMFERENCE = 2 * Math.PI * RING;
const ROTOR_FILL = '#141414';

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

const ringColour = (load: Level): string => (load === 'critical' ? md('error') : load === 'elevated' ? aircast('warning') : withAlpha(aircast('success'), 0.75));

const labelColour = (load: Level): string => (load === 'critical' ? md('error') : load === 'elevated' ? aircast('warning') : Color.White);

const place = (angle: number): readonly [number, number] => {
  const radians = (angle * Math.PI) / 180;
  return [CENTRE + ARM * Math.sin(radians), CENTRE - ARM * Math.cos(radians)];
};

const Rotor = ({ load }: { load: MotorLoad }) => {
  const [x, y] = place(load.angle);
  return (
    <g>
      <title>{load.reporting ? `Motor ${load.motor}: ${load.current} A, ${load.rpm} rpm, ${load.temperature}` : `Motor ${load.motor}: no ESC data`}</title>
      <circle cx={x} cy={y} r={RING + RING_WIDTH} fill={ROTOR_FILL} />
      <circle cx={x} cy={y} r={RING} fill="none" stroke={withAlpha(Color.White, 0.18)} strokeWidth={RING_WIDTH} strokeDasharray={load.reporting ? undefined : '3 3'} />
      {load.reporting ? (
        <circle
          cx={x}
          cy={y}
          r={RING}
          fill="none"
          stroke={ringColour(load.temperatureLevel)}
          strokeWidth={RING_WIDTH}
          strokeLinecap="round"
          strokeDasharray={`${load.fill * CIRCUMFERENCE} ${CIRCUMFERENCE}`}
          transform={`rotate(-90 ${x} ${y})`}
        />
      ) : null}
      {load.reporting ? (
        <>
          <text x={x} y={y} dy="-0.05em" textAnchor="middle" fontSize={11} fontWeight={600} fill={labelColour(load.currentLevel)}>
            {load.current}
          </text>
          <text x={x} y={y} dy="1.05em" textAnchor="middle" fontSize={7.5} fill={withAlpha(Color.White, 0.7)}>
            {load.rpm}
          </text>
        </>
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
  if (loads.length === 0) return null;
  return (
    <svg width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`} role="img" aria-label="Motor load">
      {loads.map((load) => {
        const [x, y] = place(load.angle);
        return <line key={load.motor} x1={CENTRE} y1={CENTRE} x2={x} y2={y} stroke={withAlpha(Color.White, 0.35)} strokeWidth={3} />;
      })}
      {loads.map((load) => (
        <Rotor key={load.motor} load={load} />
      ))}
      <circle cx={CENTRE} cy={CENTRE} r={BODY} fill={withAlpha(Color.Black, 0.7)} stroke={withAlpha(Color.White, 0.6)} strokeWidth={2} />
      <path d={`M ${CENTRE} ${CENTRE - BODY - 9} l -5 7 h 10 z`} fill={withAlpha(Color.White, 0.8)} />
    </svg>
  );
};
