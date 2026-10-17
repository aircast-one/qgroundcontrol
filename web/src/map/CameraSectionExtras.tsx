import { useRef } from 'react';
import { FilterChip, OutlinedTextField, Slider, Switch, Text } from '@/m3';
import { coerceIn } from '@/kotlin/Std';
import { typedNumber } from './AltitudeEntry';
import type { CameraExtras } from './ItemCamera';
import { type ClosedRange, useKeyedState } from './SettingStepper';

export const CAMERA_MODES: readonly string[] = ['Photo', 'Video', 'Survey'];

const NumberEntry = ({ label, value, enabled = true, onDone }: { label: string; value: number; enabled?: boolean; onDone: (value: number) => void }) => {
  const [typed, setTyped] = useKeyedState([value], () => trimmedNumber(value));
  return (
    <OutlinedTextField
      value={typed}
      onValueChange={setTyped}
      label={label}
      enabled={enabled}
      singleLine
      keyboardType="decimal"
      onDone={() => {
        const number = typedNumber(typed);
        if (number !== null) onDone(number);
      }}
      className="w-[110px]"
    />
  );
};

export const trimmedNumber = (value: number): string => (value === Math.floor(value) ? Math.trunc(value).toString() : value.toString());

const within = (value: number, range: ClosedRange | null): number => (range === null ? value : coerceIn(value, range.start, range.endInclusive));

export const CameraSectionExtras = ({ extras, write }: { extras: CameraExtras; write: (member: string, value: unknown) => void }) => (
  <div className="flex flex-col gap-1">
    {extras.intervalTime !== null ? <NumberEntry label="Time (s)" value={extras.intervalTime} onDone={(it) => write('cameraPhotoIntervalTime', it)} /> : null}
    {extras.intervalDistance !== null ? <NumberEntry label={`Distance (${extras.distanceUnits})`} value={extras.intervalDistance} onDone={(it) => write('cameraPhotoIntervalDistance', it)} /> : null}
    {extras.modeSupported ? (
      <div className="flex flex-wrap items-center gap-1.5">
        <Text typography="labelMedium">Mode</Text>
        <Switch checked={extras.commandsMode} onCheckedChange={(it) => write('specifyCameraMode', it)} label="Mode" />
        {CAMERA_MODES.map((label, at) => (
          <FilterChip key={label} selected={extras.mode === at} enabled={extras.commandsMode} onClick={() => write('cameraMode', at)} label={label} />
        ))}
      </div>
    ) : null}
    <div className="flex flex-wrap items-center gap-1.5">
      <Text typography="labelMedium">Gimbal</Text>
      <Switch checked={extras.commandsGimbal} onCheckedChange={(it) => write('specifyGimbal', it)} label="Gimbal" />
      <NumberEntry label="Pitch" value={extras.pitch} enabled={extras.commandsGimbal} onDone={(it) => write('gimbalPitch', within(it, extras.pitchRange))} />
      <NumberEntry label="Yaw" value={extras.yaw} enabled={extras.commandsGimbal} onDone={(it) => write('gimbalYaw', within(it, extras.yawRange))} />
    </div>
    {extras.pitchRange !== null ? <AngleSlider value={extras.pitch} range={extras.pitchRange} enabled={extras.commandsGimbal} label="Pitch" onDone={(it) => write('gimbalPitch', it)} /> : null}
    {extras.yawRange !== null ? <AngleSlider value={extras.yaw} range={extras.yawRange} enabled={extras.commandsGimbal} label="Yaw" onDone={(it) => write('gimbalYaw', it)} /> : null}
  </div>
);

const AngleSlider = ({ value, range, enabled, label, onDone }: { value: number; range: ClosedRange; enabled: boolean; label: string; onDone: (value: number) => void }) => {
  const [shown, setShown] = useKeyedState([value, range.start, range.endInclusive], () => coerceIn(value, range.start, range.endInclusive));
  const latest = useRef(shown);
  latest.current = shown;
  return (
    <Slider
      value={shown}
      onValueChange={(it) => {
        latest.current = it;
        setShown(it);
      }}
      valueRange={[range.start, range.endInclusive]}
      enabled={enabled}
      label={label}
      onValueChangeFinished={() => onDone(Math.round(latest.current))}
    />
  );
};
