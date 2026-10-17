import { useState } from 'react';
import { type Json, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { SetupCommands } from '@/bridge/BridgeCommands';
import { useLaunchedEffect, useProduceState, useUpdatedRef } from '@/kotlin/Compose';
import { mutableStateMapOf, useValue } from '@/kotlin/State';
import { delay } from '@/kotlin/Std';
import { Button, Checkbox, Slider, Switch, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { APM_SUB_FRAME_VIEW, type SubFrames, frameImage, subFrames } from '@/ui/ApmSubFrameScreen';

export const APM_SUB_MOTORS_SCREEN = 'apmSubMotors';
export const APM_SUB_MOTORS_VIEW = 'view.apmSubMotors';
const SUB_MOTORS_POLL_MS = 500;
const SUB_MOTOR_TEST_MS = 50;
const SUB_COOL_DOWN_MS = 11_000;
export const SUB_NEUTRAL = 50;

export const MOTOR_DISPLAY_FRAMES: ReadonlySet<number> = new Set([0, 1, 2, 4, 5]);

export const motorDisplayFrame = (frames: SubFrames | null): number | null => {
  const selected = frames?.selected ?? null;
  return selected !== null && MOTOR_DISPLAY_FRAMES.has(selected) ? selected : null;
};

export type SubMotor = { motor: number; reversed: boolean };

export type SubMotors = {
  armed: boolean;
  detecting: boolean;
  canRunManualTest: boolean;
  motors: readonly SubMotor[];
  warning: string;
  offersAutoDetect: boolean;
  autoDetectHelp: string;
  detectionMessages: string;
};

export const subMotors = (view: Json | null): SubMotors | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : {
        armed: optBoolean(view, 'armed'),
        detecting: optBoolean(view, 'detecting'),
        canRunManualTest: optBoolean(view, 'canRunManualTest'),
        motors: jsonObjects(optJSONArray(view, 'motors')).map((m) => ({ motor: optInt(m, 'motor'), reversed: optBoolean(m, 'reversed') })),
        warning: optText(view, 'warning'),
        offersAutoDetect: optBoolean(view, 'offersAutoDetect'),
        autoDetectHelp: optText(view, 'autoDetectHelp'),
        detectionMessages: optText(view, 'detectionMessages'),
      };

export const ApmSubMotorsScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<SubMotors | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [shouldRunManualTest, setShouldRunManualTest] = useState(false);
  const [coolingDown, setCoolingDown] = useState(false);
  const [lastIndex, setLastIndex] = useState(0);
  const [sliders] = useState(() => mutableStateMapOf<number, number>());
  const sliderValues = useValue(sliders);
  const lastIndexRef = useUpdatedRef(lastIndex);
  const frameDisplay = useProduceState<string | null>(null, [], async (set) => {
    const frame = motorDisplayFrame(subFrames(await Qgc.get(APM_SUB_FRAME_VIEW)));
    set(frame === null ? null : await frameImage(frame));
  });
  useLaunchedEffect([], async () => {
    await SetupCommands.openSubMotors();
  });
  useLaunchedEffect([revision], async (scope) => {
    const next = subMotors(await Qgc.get(APM_SUB_MOTORS_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    await delay(SUB_MOTORS_POLL_MS);
    if (scope.isActive()) setRevision((it) => it + 1);
  });
  const armed = read?.armed === true;
  useLaunchedEffect([armed], async (scope) => {
    sliders.clear();
    setShouldRunManualTest(armed);
    if (!armed && read !== null) {
      setCoolingDown(true);
      await delay(SUB_COOL_DOWN_MS);
      if (scope.isActive()) setCoolingDown(false);
    }
  });
  const testing = read?.canRunManualTest === true && shouldRunManualTest;
  useLaunchedEffect([testing], async (scope) => {
    const loop = async (): Promise<void> => {
      if (!testing || !scope.isActive()) return;
      const index = lastIndexRef.current;
      await SetupCommands.testSubMotor(index, sliders.get(index) ?? SUB_NEUTRAL);
      await delay(SUB_MOTOR_TEST_MS);
      return loop();
    };
    await loop();
  });
  const act = (path: string, ...args: unknown[]) =>
    void (async () => {
      setRefusal(await Qgc.refusalOf(path, ...args));
    })();
  const state = read;
  if (state === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>This page is for an ArduSub vehicle.</Text>;
  }
  return (
    <div className={`flex w-full flex-col gap-2 overflow-y-auto p-4 ${className ?? ''}`}>
      {frameDisplay !== null ? <img src={frameDisplay} alt="Frame" className="max-h-60 w-full object-contain" /> : null}
      {state.motors.map((motor, index) => (
        <div key={index} className="flex w-full items-center">
          <Text className="w-6 shrink-0">{`${motor.motor}`}</Text>
          <Slider
            value={sliderValues.get(index) ?? SUB_NEUTRAL}
            onValueChange={(it) => {
              setLastIndex(index);
              sliders.set(index, it);
            }}
            onValueChangeFinished={() => sliders.set(index, SUB_NEUTRAL)}
            valueRange={[0, 100]}
            enabled={testing}
            label={`Motor ${motor.motor}`}
            className="flex-1"
          />
          <Checkbox
            checked={motor.reversed}
            enabled={testing}
            onCheckedChange={(it) => {
              sliders.set(index, SUB_NEUTRAL);
              act('apmSubMotors.reverse', motor.motor, it);
            }}
            label={`Reverse motor ${motor.motor}`}
          />
        </div>
      ))}
      <Text typography="bodySmall">Reverse motor direction</Text>
      <Text typography="bodySmall">{state.warning}</Text>
      <div className="flex items-center gap-2">
        <Switch checked={state.armed} enabled={!coolingDown} onCheckedChange={(it) => act('apmSubMotors.arm', it)} label="Arm the vehicle" />
        <Text color={md('error')}>
          {coolingDown ? 'A 10 second coooldown is required before testing again, please stand by...' : 'Slide this switch to arm the vehicle and enable the motor test (CAUTION!)'}
        </Text>
      </div>
      {state.offersAutoDetect ? (
        <>
          <Text typography="titleMedium">Automatic motor direction detection</Text>
          <Text typography="bodySmall">{state.autoDetectHelp}</Text>
          <Button enabled={!state.detecting} onClick={() => act('apmSubMotors.autoDetect')} className="self-start">
            Auto-Detect Directions
          </Button>
          {state.detectionMessages.trim() !== '' ? (
            <Text typography="bodySmall" className="whitespace-pre-line">
              {state.detectionMessages}
            </Text>
          ) : null}
        </>
      ) : null}
      {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
    </div>
  );
};
