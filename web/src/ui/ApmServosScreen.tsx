import { useEffect, useRef, useState } from 'react';
import { type Json, isNull, jsonObjects, optDouble, optInt, optJSONArray, optJSONObject } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useKeyedState, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { LinearProgressIndicator, OutlinedTextField, Text } from '@/m3';
import { toDoubleOrNull, typedNumber } from '@/map/AltitudeEntry';
import { useRepeatWhileHeld } from '@/map/SettingStepper';
import { md } from '@/theme/Colors';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow } from '@/ui/SettingsScreen';
import { SectionHeader } from '@/ui/SetupRow';

export const APM_SERVOS_VIEW = 'view.apmServos';
export const APM_SERVOS_SCREEN = 'apmServos';
const POLL_MS = 200;
const REPEAT_DELAY_MS = 350;
const REPEAT_MS = 80;

export type ApmServo = {
  index: number;
  pwm: number | null;
  position: number | null;
  function: Fact | null;
  min: Fact | null;
  trim: Fact | null;
  max: Fact | null;
  reversed: Fact | null;
};

export const apmServos = (view: Json | null): readonly ApmServo[] => {
  const servos = optJSONArray(view, 'servos');
  if (servos === null) return [];
  return jsonObjects(servos).map((servo) => {
    const fact = (key: string): Fact | null => {
      const control = optJSONObject(servo, key);
      return control === null ? null : factFromControl(control);
    };
    return {
      index: optInt(servo, 'index'),
      pwm: isNull(servo, 'pwm') ? null : optInt(servo, 'pwm'),
      position: isNull(servo, 'position') ? null : Math.fround(optDouble(servo, 'position')),
      function: fact('function'),
      min: fact('min'),
      trim: fact('trim'),
      max: fact('max'),
      reversed: fact('reversed'),
    };
  });
};

export const stepped = (fact: Fact, direction: number): number | null => {
  const current = typeof fact.value === 'number' ? fact.value : toDoubleOrNull(fact.valueString);
  return current === null ? null : current + direction;
};

export const ApmServosScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [servos, setServos] = useState<readonly ApmServo[] | null>(null);

  useLaunchedEffect([revision], async (scope) => {
    const next = apmServos(await Qgc.get(APM_SERVOS_VIEW));
    if (!scope.isActive()) return;
    setServos(next);
    await delay(POLL_MS);
    if (scope.isActive()) setRevision((it) => it + 1);
  });
  const read = servos;
  if (read === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>Reading servo outputs.</Text>;
  }
  if (read.length === 0) {
    return <Text className={`block p-4 ${className ?? ''}`}>This vehicle exposes no servo outputs.</Text>;
  }
  return (
    <div className={`flex h-full flex-col overflow-y-auto ${className ?? ''}`}>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block px-5 py-3">
        Configure ArduPilot servo outputs.
      </Text>
      {read.map((servo) => (
        <ServoCard key={servo.index} servo={servo} />
      ))}
    </div>
  );
};

const ServoCard = ({ servo }: { servo: ApmServo }) => (
  <div className="flex w-full flex-col py-1">
    <SectionHeader text={`Servo ${servo.index}`} />
    <div className="flex w-full items-center gap-3 px-5 py-1">
      <Text typography="bodyMedium">Position</Text>
      <LinearProgressIndicator progress={servo.position ?? 0} className="flex-1" label={`Servo ${servo.index} position`} />
      <Text typography="bodyMedium" className="w-12 shrink-0">
        {servo.pwm?.toString() ?? '-'}
      </Text>
    </div>
    {servo.function !== null ? <FactRow fact={servo.function} title="Function" /> : null}
    {servo.min !== null ? <Stepper title="Min" fact={servo.min} /> : null}
    {servo.trim !== null ? <Stepper title="Trim" fact={servo.trim} /> : null}
    {servo.max !== null ? <Stepper title="Max" fact={servo.max} /> : null}
    {servo.reversed !== null ? <FactRow fact={servo.reversed} title="Reversed" /> : null}
  </div>
);

const Stepper = ({ title, fact }: { title: string; fact: Fact }) => {
  const [refusal, setRefusal] = useKeyedState<string | null>([fact.path], () => null);
  const latest = useUpdatedRef(fact);
  const [pending, setPendingState] = useKeyedState<number | null>([fact.path], () => null);
  const pendingRef = useRef(pending);
  pendingRef.current = pending;
  const setPending = (next: number | null) => {
    pendingRef.current = next;
    setPendingState(next);
  };

  const write = (next: number) => {
    setPending(next);
    void (async () => {
      const refused = await Qgc.writeRefusal(fact.path, next);
      setRefusal(refused);
      if (refused !== null) setPending(null);
    })();
  };

  const step = (direction: number) => {
    const base = pendingRef.current ?? stepped(latest.current, 0);
    if (base === null) return;
    write(base + direction);
  };

  const shown = pending !== null ? String(Math.trunc(pending)) : fact.valueString;
  const [typed, setTyped] = useKeyedState([shown], () => shown);
  const commit = () => {
    const parsed = typedNumber(typed);
    const number = parsed !== null && parsed === Math.floor(parsed) ? parsed : null;
    if (number === null) {
      setRefusal('Enter a whole number.');
      setTyped(shown);
      return;
    }
    if (number !== typedNumber(shown)) write(number);
  };

  useEffect(() => setPending(null), [fact.valueString]);

  return (
    <div className="flex flex-col">
      <div className="flex w-full items-center gap-3 px-5 py-1">
        <Text typography="bodyLarge" className="flex-1">
          {title}
        </Text>
        <RepeatButton label="-" onStep={() => step(-1)} />
        <div className="w-24 shrink-0" onBlur={commit}>
          <OutlinedTextField value={typed} onValueChange={setTyped} ariaLabel={`${title}, ${fact.name}`} singleLine keyboardType="number" onDone={commit} />
        </div>
        <RepeatButton label="+" onStep={() => step(1)} />
      </div>
      {refusal !== null ? (
        <Text color={md('error')} className="block px-5">
          {refusal}
        </Text>
      ) : null}
    </div>
  );
};

const RepeatButton = ({ label, onStep }: { label: string; onStep: () => void }) => {
  const press = useRepeatWhileHeld(true, () => onStep(), REPEAT_DELAY_MS, REPEAT_MS);
  return (
    <button type="button" className="grid size-10 shrink-0 touch-none select-none place-items-center rounded-shape-sm" style={{ background: md('secondaryContainer') }} {...press}>
      <Text typography="titleMedium">{label}</Text>
    </button>
  );
};
