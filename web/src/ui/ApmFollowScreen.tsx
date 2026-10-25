import { type MouseEvent, useState } from 'react';
import { type Json, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { delay, format } from '@/kotlin/Std';
import { Button, Checkbox, DropdownMenu, DropdownMenuItem, OutlinedButton, OutlinedTextField, Text } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { md, withAlpha } from '@/theme/Colors';
import { type DistanceUnit, transformUnit } from '@/ui/PlanTransform';
import { SectionHeader } from '@/ui/SetupRow';

export const APM_FOLLOW_VIEW = 'view.apmFollow';
export const APM_FOLLOW_SCREEN = 'apmFollow';
export const APM_FOLLOW_ENABLE = 'apmFollow.enable';
export const APM_FOLLOW_RESET = 'apmFollow.reset';
export const APM_FOLLOW_LEAD = 'apmFollow.lead';
export const APM_FOLLOW_POSITION = 'apmFollow.position';
export const APM_FOLLOW_POINT = 'apmFollow.point';
export const APM_FOLLOW_OFFSETS = 'apmFollow.offsets';
export const APM_FOLLOW_HEIGHT = 'apmFollow.height';
const WAITING_POLL_MS = 500;

export type FollowLead = { id: number; name: string };

export type ApmFollow = {
  available: boolean;
  enabled: boolean;
  waiting: boolean;
  supported: boolean;
  unsupportedText: string;
  showSettings: boolean;
  rover: boolean;
  leads: readonly FollowLead[];
  leadIndex: number;
  positionOptions: readonly string[];
  positionIndex: number;
  pointOptions: readonly string[];
  pointIndex: number;
  angle: number;
  distance: number;
  height: number;
  horizontal: DistanceUnit;
  vertical: DistanceUnit;
};

const shown = (unit: DistanceUnit, metres: number): number => metres / unit.metresPerUnit;

const metres = (unit: DistanceUnit, value: number): number => value * unit.metresPerUnit;

export const text = (unit: DistanceUnit, value: number): string => `${oneDecimal(shown(unit, value))} ${unit.name}`;

export const apmFollow = (view: Json | null): ApmFollow | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : {
        available: true,
        enabled: optBoolean(view, 'enabled'),
        waiting: optBoolean(view, 'waiting'),
        supported: optBoolean(view, 'supported', true),
        unsupportedText: optText(view, 'unsupportedText'),
        showSettings: optBoolean(view, 'showSettings'),
        rover: optBoolean(view, 'rover'),
        leads: jsonObjects(optJSONArray(view, 'leads')).map((lead) => ({ id: optInt(lead, 'id'), name: optText(lead, 'name') })),
        leadIndex: optInt(view, 'leadIndex', -1),
        positionOptions: jsonStrings(optJSONArray(view, 'positionOptions')),
        positionIndex: optInt(view, 'positionIndex'),
        pointOptions: jsonStrings(optJSONArray(view, 'pointOptions')),
        pointIndex: optInt(view, 'pointIndex', -1),
        angle: optDouble(view, 'angle'),
        distance: optDouble(view, 'distance'),
        height: optDouble(view, 'height'),
        horizontal: transformUnit(view, 'horizontal'),
        vertical: transformUnit(view, 'vertical'),
      };

export const oneDecimal = (value: number): string => format('%.1f', value);

export const ApmFollowScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<ApmFollow | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const bump = () => setRevision((it) => it + 1);

  useLaunchedEffect([revision], async (scope) => {
    const next = apmFollow(await Qgc.get(APM_FOLLOW_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    setLoaded(true);
  });
  const waiting = read?.waiting === true;
  useLaunchedEffect([waiting], async (scope) => {
    const loop = async (): Promise<void> => {
      if (!waiting || !scope.isActive()) return;
      await delay(WAITING_POLL_MS);
      if (!scope.isActive()) return;
      bump();
      return loop();
    };
    await loop();
  });
  if (!loaded) return null;
  const follow = read;
  if (follow === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>This vehicle has no follow me parameters.</Text>;
  }

  const act = (path: string, ...args: unknown[]) =>
    void (async () => {
      setRefusal(await Qgc.refusalOf(path, ...args));
      bump();
    })();

  return (
    <div className={`flex w-full flex-col overflow-y-auto px-5 py-3 ${className ?? ''}`}>
      <div className="flex items-center gap-3">
        <Checkbox checked={follow.enabled} onCheckedChange={(it) => act(APM_FOLLOW_ENABLE, it)} label="Enable follow me" />
        <Text>Enable follow me</Text>
      </div>
      {follow.waiting ? <Text typography="bodyMedium">Waiting for Vehicle to update</Text> : null}
      {!follow.supported ? (
        <>
          <Text color={md('error')} className="block py-2">
            {follow.unsupportedText}
          </Text>
          <Button onClick={() => act(APM_FOLLOW_RESET)} className="self-start">
            Reset to supported settings
          </Button>
        </>
      ) : null}
      {follow.showSettings ? (
        <>
          <SectionHeader text="Follow me settings" />
          {follow.leads.length > 1 ? (
            <Choice label="Follow" options={follow.leads.map((lead) => lead.name)} index={follow.leadIndex} onPick={(it) => act(APM_FOLLOW_LEAD, follow.leads[it]?.id)} />
          ) : null}
          <Choice label="Vehicle position" options={follow.positionOptions} index={follow.positionIndex} onPick={(it) => act(APM_FOLLOW_POSITION, it)} />
          {!follow.rover ? <Choice label="Point vehicle" options={follow.pointOptions} index={follow.pointIndex} onPick={(it) => act(APM_FOLLOW_POINT, it)} /> : null}
          {follow.positionIndex === 1 ? (
            <>
              <Text typography="titleSmall" className="block pt-3">
                Vehicle offsets
              </Text>
              <div className="flex">
                <OffsetGraphic follow={follow} onAngle={(it) => act(APM_FOLLOW_OFFSETS, it, follow.distance)} />
                {!follow.rover ? <HeightGraphic heightText={text(follow.vertical, follow.height)} /> : null}
              </div>
              <NumberEntry label="Angle" units="deg" value={follow.angle} onDone={(it) => act(APM_FOLLOW_OFFSETS, it, follow.distance)} />
              <NumberEntry label="Distance" units={follow.horizontal.name} value={shown(follow.horizontal, follow.distance)} onDone={(it) => act(APM_FOLLOW_OFFSETS, follow.angle, metres(follow.horizontal, it))} />
              {!follow.rover ? <NumberEntry label="Height" units={follow.vertical.name} value={shown(follow.vertical, follow.height)} onDone={(it) => act(APM_FOLLOW_HEIGHT, metres(follow.vertical, it))} /> : null}
            </>
          ) : null}
        </>
      ) : null}
      {refusal !== null ? (
        <Text color={md('error')} className="block pt-2">
          {refusal}
        </Text>
      ) : null}
    </div>
  );
};

const Choice = ({ label, options, index, onPick }: { label: string; options: readonly string[]; index: number; onPick: (index: number) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex w-full items-center py-1">
      <Text className="flex-1">{label}</Text>
      <div className="relative">
        <OutlinedButton onClick={() => setOpen(true)}>{options[index] ?? ''}</OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {options.map((option, at) => (
            <DropdownMenuItem
              key={at}
              text={option}
              onClick={() => {
                setOpen(false);
                onPick(at);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};

const NumberEntry = ({ label, units, value, onDone }: { label: string; units: string; value: number; onDone: (value: number) => void }) => {
  const [typed, setTyped] = useKeyedState([value], () => oneDecimal(value));
  return (
    <div className="flex w-full items-center py-1">
      <Text className="flex-1">{label}</Text>
      <OutlinedTextField
        value={typed}
        onValueChange={setTyped}
        singleLine
        suffix={units}
        keyboardType="decimal"
        onDone={() => {
          const number = toDoubleOrNull(typed);
          if (number !== null) onDone(number);
        }}
        className="flex-1"
      />
    </div>
  );
};

export const headingOfTap = (x: number, y: number): number => {
  const geometric = (Math.atan2(y, x) * 180) / Math.PI;
  const heading = 90 - geometric;
  return heading < 0 ? heading + 360 : heading > 360 ? heading - 360 : heading;
};

export const vehicleYaw = (follow: ApmFollow): number => {
  if (follow.rover || follow.pointIndex === 0) return 0.0;
  if (follow.pointIndex === 1) return 180.0;
  return -follow.angle;
};

const OFFSET_GRAPHIC = 240;
const DISTANCE_LABEL_RADIUS = (OFFSET_GRAPHIC / 2 - 16) / 2;

const triangle = (points: readonly (readonly [number, number])[]): string => points.map(([x, y]) => `${x},${y}`).join(' ');

const OffsetGraphic = ({ follow, onAngle }: { follow: ApmFollow; onAngle: (angle: number) => void }) => {
  const shade = md('outlineVariant');
  const ink = md('onSurface');
  const accent = md('primary');
  const center = OFFSET_GRAPHIC / 2;
  const arrow = 14;
  const at = { x: center, y: 20 };
  const labelRadians = (follow.angle * Math.PI) / 180;
  const tap = (event: MouseEvent<SVGSVGElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    const x = event.clientX - box.left;
    const y = event.clientY - box.top;
    onAngle(headingOfTap(x - box.width / 2, box.height / 2 - y));
  };
  return (
    <>
      <Text typography="labelSmall" color={md('onSurfaceVariant')}>
        Click in the graphic to change angle
      </Text>
      <div className="relative flex shrink-0 items-center justify-center" style={{ width: OFFSET_GRAPHIC, height: OFFSET_GRAPHIC }}>
        <svg width={OFFSET_GRAPHIC} height={OFFSET_GRAPHIC} className="cursor-pointer" onClick={tap}>
          <line x1={center} y1={0} x2={center} y2={OFFSET_GRAPHIC} stroke={shade} strokeWidth={3} />
          <line x1={0} y1={center} x2={OFFSET_GRAPHIC} y2={center} stroke={shade} strokeWidth={3} />
          <polygon
            points={triangle([
              [center, center - arrow],
              [center - arrow * 0.7, center + arrow * 0.7],
              [center + arrow * 0.7, center + arrow * 0.7],
            ])}
            fill={accent}
          />
          <g transform={`rotate(${follow.angle} ${center} ${center})`}>
            <line x1={center} y1={at.y + 14} x2={center} y2={center - arrow - 4} stroke={withAlpha(ink, 0.4)} strokeWidth={2} />
            <polygon
              transform={`rotate(${vehicleYaw(follow)} ${at.x} ${at.y})`}
              points={triangle([
                [at.x, at.y - 12],
                [at.x - 9, at.y + 10],
                [at.x + 9, at.y + 10],
              ])}
              fill={ink}
            />
          </g>
        </svg>
        <span
          className="pointer-events-none absolute"
          style={{
            background: md('surface'),
            left: '50%',
            top: '50%',
            transform: `translate(-50%, -50%) translate(${DISTANCE_LABEL_RADIUS * Math.sin(labelRadians)}px, ${-DISTANCE_LABEL_RADIUS * Math.cos(labelRadians)}px)`,
          }}
        >
          <Text typography="labelSmall">{text(follow.horizontal, follow.distance)}</Text>
        </span>
      </div>
    </>
  );
};

const HEIGHT_GRAPHIC_WIDTH = 56;

const HeightGraphic = ({ heightText }: { heightText: string }) => {
  const ink = md('onSurface');
  const center = HEIGHT_GRAPHIC_WIDTH / 2;
  const tick = 8;
  return (
    <div className="relative flex shrink-0 items-center justify-center" style={{ width: HEIGHT_GRAPHIC_WIDTH, height: OFFSET_GRAPHIC }}>
      <svg width={HEIGHT_GRAPHIC_WIDTH} height={OFFSET_GRAPHIC} className="absolute inset-0" aria-hidden>
        <line x1={center} y1={0} x2={center} y2={OFFSET_GRAPHIC} stroke={withAlpha(ink, 0.4)} strokeWidth={2} />
        <line x1={center - tick} y1={1} x2={center + tick} y2={1} stroke={ink} strokeWidth={2} />
        <line x1={center - tick} y1={OFFSET_GRAPHIC - 1} x2={center + tick} y2={OFFSET_GRAPHIC - 1} stroke={ink} strokeWidth={2} />
      </svg>
      <span className="relative" style={{ background: md('surface') }}>
        <Text typography="labelSmall">{heightText}</Text>
      </span>
    </div>
  );
};
