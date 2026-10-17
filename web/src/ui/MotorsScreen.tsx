import { useMemo, useState } from 'react';
import { type Json, isNull, optBoolean, optInt } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { FilledTonalButton, OutlinedButton, Slider, Surface, Switch, Text, stepsFor } from '@/m3';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { SETUP, setupReadiness } from '@/ui/SetupView';
import { TAP } from '@/kotlin/Platform';

const FRAME_VIEW = 'view.frame';

const TIMEOUT_SECONDS = 3;
const UNKNOWN_MOTOR_COUNT = 8;

export const reportedMotors = (view: Json | null): number | null => {
  if (view === null || isNull(view, 'motorCount')) return null;
  const count = optInt(view, 'motorCount');
  return count >= 1 ? count : null;
};

export const motorCount = (reported: number | null): number => reported ?? UNKNOWN_MOTOR_COUNT;

export const motorCountNotice = (reported: number | null): string | null => (reported === null ? 'Warning: Unable to determine motor count' : null);

export type MotorGate = { connected: boolean; armed: boolean; contactKnownLost: boolean };

export const motorGate = (view: Json | null): MotorGate => ({
  connected: optBoolean(view, 'connected'),
  armed: optBoolean(view, 'armed'),
  contactKnownLost: view !== null && !isNull(view, 'contactLost') && optBoolean(view, 'contactLost'),
});

export const canTest = (gate: MotorGate, propsOff: boolean): boolean => gate.connected && propsOff && !gate.armed && !gate.contactKnownLost;

export const canStop = (gate: MotorGate): boolean => gate.connected;

export const motorRefusal = (gate: MotorGate): string | null =>
  !gate.connected
    ? 'No vehicle is connected, so nothing will answer a motor test.'
    : gate.armed
      ? 'The vehicle is armed. Disarm it before testing a motor.'
      : gate.contactKnownLost
        ? 'The vehicle has stopped answering. Check the link before testing a motor.'
        : null;

export const spin = (motor: number, percent: number): void => {
  const seconds = percent === 0 ? 0 : TIMEOUT_SECONDS;
  void VehicleCommands.motorTest(motor, percent, seconds, true);
};

export const motorLabel = (motor: number, letters: boolean): string => (letters ? String.fromCharCode('A'.charCodeAt(0) + (motor - 1)) : String(motor));

const motorNumbers = (count: number): number[] => Array.from({ length: count }, (_, index) => index + 1);

const chunked = <T,>(items: readonly T[], size: number): T[][] =>
  Array.from({ length: Math.ceil(items.length / size) }, (_, index) => items.slice(index * size, index * size + size));

export const MotorsScreen = ({ className }: { className?: string }) => {
  const frameJson = useQgcPath(FRAME_VIEW);
  const reported = useMemo(() => reportedMotors(frameJson), [frameJson]);
  const gate = useMemo(() => motorGate(frameJson), [frameJson]);
  const motors = motorCount(reported);
  const [propsOff, setPropsOff] = useState(false);
  const [throttle, setThrottle] = useState(0);
  const setupJson = useQgcPath(SETUP);
  const letters = useMemo(() => setupReadiness(setupJson)?.firmware === 'apm', [setupJson]);
  const testable = canTest(gate, propsOff);
  const percent = Math.trunc(throttle);
  const countNotice = motorCountNotice(reported);
  const refusal = motorRefusal(gate);
  const switchText = propsOff ? 'Careful: motors are enabled' : 'Propellers are removed - enable slider and motors';

  return (
    <div className={`flex flex-col gap-3 overflow-y-auto p-4 ${className ?? ''}`}>
      <Surface color={aircast('warningContainer')} contentColor={aircast('warning')} shape="full">
        <div className="flex w-full items-center gap-3 px-4 py-2.5">
          <Drawable name="ic_warning" />
          <Text typography="labelLarge">Make sure you remove all props.</Text>
        </div>
      </Surface>
      {countNotice !== null ? (
        <Text typography="bodyMedium" color={aircast('warning')}>
          {countNotice}
        </Text>
      ) : null}
      {refusal !== null ? (
        <Text typography="bodyMedium" color={md('error')}>
          {refusal}
        </Text>
      ) : null}

      <div className="flex items-center gap-3">
        <Switch
          checked={propsOff}
          label={switchText}
          onCheckedChange={(it) => {
            setPropsOff(it);
            if (!it) setThrottle(0);
          }}
        />
        <Text typography="bodyMedium" color={aircast('warning')}>
          {switchText}
        </Text>
      </div>

      <div className="flex w-full items-center">
        <Text typography="bodyLarge" className="flex-1">
          Throttle
        </Text>
        <Text typography="labelMedium" color={md('primary')}>
          {`${percent}%`}
        </Text>
      </div>
      <Slider value={throttle} onValueChange={setThrottle} valueRange={[0, 100]} steps={stepsFor(0, 100, 1)} enabled={testable} label="Throttle" />

      {chunked(motorNumbers(motors), 2).map((pair) => (
        <div key={pair[0]} className="flex gap-3">
          {pair.map((motor) => (
            <Surface key={motor} onClick={() => spin(motor, percent)} enabled={testable} shape="large" color={md('surfaceContainer')} className="flex-1">
              <span className="flex w-full flex-col items-center py-5">
                <Text typography="bodyLarge">{`Motor ${motorLabel(motor, letters)}`}</Text>
                <Text typography="labelMedium" color={md('onSurfaceVariant')}>
                  {testable ? `${TAP} to spin` : 'Locked'}
                </Text>
              </span>
            </Surface>
          ))}
          {pair.length === 1 ? <span className="flex-1" /> : null}
        </div>
      ))}

      <div className="flex flex-wrap gap-2">
        <FilledTonalButton onClick={() => motorNumbers(motors).map((it) => spin(it, percent))} enabled={testable}>
          All
        </FilledTonalButton>
        <OutlinedButton onClick={() => motorNumbers(motors).map((it) => spin(it, 0))} enabled={canStop(gate)}>
          Stop
        </OutlinedButton>
      </div>
    </div>
  );
};
