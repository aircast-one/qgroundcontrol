import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useKeyedState, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { DropdownMenu, DropdownMenuItem, OutlinedButton, Slider, Switch, Text } from '@/m3';
import { aircast } from '@/theme/Colors';

export const ACTUATOR_TEST_ACTIVE = 'actuatorTest.setActive';
export const ACTUATOR_TEST_SET = 'actuatorTest.setChannelTo';
export const ACTUATOR_TEST_STOP = 'actuatorTest.stopControl';
const SEND_MS = 50;
const SNAP_FRACTION = 0.15;

export type TestChannel = {
  readonly label: string;
  readonly function: number;
  readonly min: number;
  readonly max: number;
  readonly default: number | null;
  readonly isMotor: boolean;
  readonly snap: boolean;
  readonly snapRange: number;
  readonly from: number;
  readonly rest: number;
};

export const TestChannel = (label: string, fn: number, min: number, max: number, fallback: number | null, isMotor: boolean): TestChannel => {
  const snap = fallback === null;
  const snapRange = (max - min) * SNAP_FRACTION;
  const from = snap ? min - snapRange : min;
  return { label, function: fn, min, max, default: fallback, isMotor, snap, snapRange, from, rest: fallback ?? from };
};

export type ActuatorTesting = { actuators: readonly TestChannel[]; allMotors: TestChannel | null; hadFailure: boolean };
export type ActuatorActionChoice = { label: string; function: number };
export type ActuatorActionGroup = { label: string; type: number; actions: readonly ActuatorActionChoice[] };

export const ACTUATOR_ACTION_TRIGGER = 'actuatorAction.trigger';

export const actuatorActions = (view: Json | null): readonly ActuatorActionGroup[] => {
  const groups = optJSONArray(view, 'actions');
  if (groups === null) return [];
  return jsonObjects(groups).map((group) => ({
    label: optText(group, 'label'),
    type: optInt(group, 'type'),
    actions: jsonObjects(optJSONArray(group, 'actions')).map((it) => ({ label: optText(it, 'label'), function: optInt(it, 'function') })),
  }));
};

const testChannel = (json: Json | null): TestChannel | null =>
  json === null
    ? null
    : TestChannel(optText(json, 'label'), optInt(json, 'function'), optDouble(json, 'min'), optDouble(json, 'max'), isNull(json, 'default') ? null : optDouble(json, 'default'), optBoolean(json, 'isMotor'));

export const actuatorTesting = (view: Json | null): ActuatorTesting | null => {
  const testing = optJSONObject(view, 'testing');
  return testing === null
    ? null
    : {
        actuators: jsonObjects(optJSONArray(testing, 'actuators')).flatMap((it) => {
          const channel = testChannel(it);
          return channel === null ? [] : [channel];
        }),
        allMotors: testChannel(optJSONObject(testing, 'allMotors')),
        hadFailure: optBoolean(testing, 'hadFailure'),
      };
};

export const snapped = (channel: TestChannel, value: number): number => {
  if (!channel.snap || value >= channel.min) return value;
  if (value < channel.min - channel.snapRange / 2) return channel.min - channel.snapRange;
  return channel.min;
};

export const sentValue = (channel: TestChannel, value: number): number | null => (value < channel.min - channel.snapRange / 2 ? channel.default : value);

const restingValues = (actuators: readonly TestChannel[]): ReadonlyMap<number, number> => new Map(actuators.map((it) => [it.function, it.rest]));

export const ActuatorTestSection = ({
  testing,
  actions = [],
  enabled,
  assigning = false,
  onEnabled,
}: {
  testing: ActuatorTesting;
  actions?: readonly ActuatorActionGroup[];
  enabled: boolean;
  assigning?: boolean;
  onEnabled: (on: boolean) => void;
}) => {
  const actuatorsKey = useMemo(() => JSON.stringify(testing.actuators), [testing.actuators]);
  const allMotorsKey = useMemo(() => JSON.stringify(testing.allMotors), [testing.allMotors]);
  const [values, setValuesState] = useKeyedState<ReadonlyMap<number, number>>([actuatorsKey], () => restingValues(testing.actuators));
  const [moved, setMovedState] = useKeyedState<ReadonlySet<number>>([actuatorsKey], () => new Set());
  const [allMotors, setAllMotors] = useKeyedState([allMotorsKey], () => testing.allMotors?.rest ?? 0.0);
  const latestValues = useRef(values);
  const latestMoved = useRef(moved);
  latestValues.current = values;
  latestMoved.current = moved;
  const setValues = (next: ReadonlyMap<number, number>) => {
    latestValues.current = next;
    setValuesState(next);
  };
  const setMoved = (next: ReadonlySet<number>) => {
    latestMoved.current = next;
    setMovedState(next);
  };

  useEffect(
    () => () => {
      void Qgc.invoke(ACTUATOR_TEST_ACTIVE, false);
    },
    [],
  );

  const setEnabled = (on: boolean) => {
    onEnabled(on);
    if (!on) {
      setValues(restingValues(testing.actuators));
      setMoved(new Set());
      setAllMotors(testing.allMotors?.rest ?? 0.0);
    }
    void Qgc.invoke(ACTUATOR_TEST_ACTIVE, on);
  };

  return (
    <div className="flex flex-col gap-2">
      <Text typography="titleMedium">Actuator testing</Text>
      {testing.actuators.length === 0 ? (
        <Text>Configure some outputs in order to test them.</Text>
      ) : (
        <>
          <div className="flex items-center gap-2">
            <Switch checked={enabled && !testing.hadFailure} onCheckedChange={setEnabled} enabled={!testing.hadFailure && !assigning} label="Enable sliders" />
            <Text color={aircast('warning')}>{enabled ? 'Careful: actuator sliders are enabled' : 'Propellers are removed - enable sliders'}</Text>
          </div>
          {testing.allMotors !== null ? (
            <TestSlider
              channel={testing.allMotors}
              value={allMotors}
              enabled={enabled}
              onChange={(value) => {
                setAllMotors(value);
                const motorFunctions = testing.actuators.filter((it) => it.isMotor).map((it) => it.function);
                setValues(new Map([...latestValues.current, ...motorFunctions.map((fn) => [fn, value] as const)]));
                setMoved(new Set([...latestMoved.current, ...motorFunctions]));
              }}
            />
          ) : null}
          {testing.actuators.map((channel) => (
            <Fragment key={channel.function}>
              <TestSlider
                channel={channel}
                value={values.get(channel.function) ?? channel.rest}
                enabled={enabled}
                onChange={(value) => {
                  setValues(new Map(latestValues.current).set(channel.function, value));
                  setMoved(new Set([...latestMoved.current, channel.function]));
                }}
              />
              {enabled && moved.has(channel.function) ? (
                <ChannelSender
                  channel={channel}
                  value={values.get(channel.function) ?? channel.rest}
                  onStopped={() => {
                    setValues(new Map(latestValues.current).set(channel.function, channel.rest));
                    setMoved(new Set([...latestMoved.current].filter((it) => it !== channel.function)));
                  }}
                />
              ) : null}
            </Fragment>
          ))}
        </>
      )}
      {actions.length > 0 ? (
        <div className="flex gap-2 overflow-x-auto">
          {actions.map((group, index) => (
            <ActionGroupButton key={index} group={group} enabled={!enabled && !assigning} />
          ))}
        </div>
      ) : null}
    </div>
  );
};

const ActionGroupButton = ({ group, enabled }: { group: ActuatorActionGroup; enabled: boolean }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative shrink-0">
      <OutlinedButton onClick={() => setOpen(true)} enabled={enabled}>
        {group.label}
      </OutlinedButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {group.actions.map((action, index) => (
          <DropdownMenuItem
            key={index}
            text={action.label}
            onClick={() => {
              setOpen(false);
              void Qgc.invoke(ACTUATOR_ACTION_TRIGGER, group.type, action.function);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};

const ChannelSender = ({ channel, value, onStopped }: { channel: TestChannel; value: number; onStopped: () => void }) => {
  const latest = useUpdatedRef(value);
  const stopped = useUpdatedRef(onStopped);
  useLaunchedEffect([channel.function], async (scope) => {
    const loop = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const send = sentValue(channel, latest.current);
      await (send === null ? Qgc.invoke(ACTUATOR_TEST_STOP, channel.function) : Qgc.invoke(ACTUATOR_TEST_SET, channel.function, send));
      if (send === null) {
        if (scope.isActive()) stopped.current();
        return;
      }
      await delay(SEND_MS);
      return loop();
    };
    await loop();
  });
  return null;
};

const TestSlider = ({ channel, value, enabled, onChange }: { channel: TestChannel; value: number; enabled: boolean; onChange: (value: number) => void }) => (
  <div className="flex w-full flex-col">
    <Text typography="labelMedium">{channel.label}</Text>
    <Slider value={value} onValueChange={(it) => onChange(snapped(channel, it))} valueRange={[channel.from, channel.max]} enabled={enabled} label={channel.label} />
  </div>
);
