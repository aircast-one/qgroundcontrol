import { type MouseEvent, type PointerEvent, type ReactNode, useEffect, useRef, useState } from 'react';
import { AlertDialog, OutlinedTextField, Slider, Text, TextButton } from '@/m3';
import { useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { coerceIn, delay, format } from '@/kotlin/Std';
import { md } from '@/theme/Colors';
import { typedNumber } from './AltitudeEntry';

const REPEAT_DELAY_MS = 400;
const COMMIT_DELAY_MS = 500;
const REPEAT_INTERVAL_MS = 90;
const DISABLED_ALPHA = 0.38;
const STEP_BUTTON = 40;
const VALUE_MIN_WIDTH = 72;

export type ClosedRange = { readonly start: number; readonly endInclusive: number };

const sameKeys = (a: readonly unknown[], b: readonly unknown[]) => a.length === b.length && a.every((key, index) => Object.is(key, b[index]));

export const useKeyedState = <T,>(keys: readonly unknown[], initial: () => T): [T, (next: T) => void] => {
  const [held, setHeld] = useState(() => ({ keys, value: initial() }));
  const current = sameKeys(held.keys, keys) ? held : { keys, value: initial() };
  if (current !== held) setHeld(current);
  return [current.value, (next: T) => setHeld((previous) => ({ keys: previous.keys, value: next }))];
};

export const steppedValue = (current: number, step: number, direction: number, range: ClosedRange | null): number => {
  const next = current + step * direction;
  return range ? coerceIn(next, range.start, range.endInclusive) : next;
};

export const stepperText = (value: number | null, decimals: number): string =>
  value !== null && Number.isFinite(value) ? format(`%.${decimals}f`, value) : '—';

export const stepperDecimals = (value: number | null, step: number): number =>
  [value, step].filter((it): it is number => it !== null).every((it) => it === Math.floor(it)) ? 0 : 1;

export const useRepeatWhileHeld = (enabled: boolean, onStep: (repeats: number) => void, firstDelay: number, interval: number, onRelease: () => void = () => {}) => {
  const step = useUpdatedRef(onStep);
  const release = useUpdatedRef(onRelease);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const held = useRef(false);
  const cancel = () => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
  };
  const end = () => {
    if (!held.current) return;
    held.current = false;
    cancel();
    release.current();
  };
  useEffect(() => cancel, []);
  const repeat = (repeats: number) => () => {
    step.current(repeats);
    timer.current = setTimeout(repeat(repeats + 1), interval);
  };
  return {
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      if (!enabled || event.button !== 0) return;
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      cancel();
      held.current = true;
      step.current(0);
      timer.current = setTimeout(repeat(1), firstDelay);
    },
    onPointerUp: end,
    onPointerCancel: end,
    onLostPointerCapture: end,
    onClick: (event: MouseEvent<HTMLElement>) => {
      if (!enabled || event.detail !== 0) return;
      step.current(0);
      release.current();
    },
  };
};

const StepButton = ({ sign, description, enabled, onStep }: { sign: string; description: string; enabled: boolean; onStep: () => void }) => {
  const press = useRepeatWhileHeld(enabled, onStep, REPEAT_DELAY_MS + REPEAT_INTERVAL_MS, REPEAT_INTERVAL_MS);
  return (
    <button
      type="button"
      aria-label={description}
      aria-disabled={!enabled}
      className="grid shrink-0 touch-none select-none place-items-center rounded-full"
      style={{ width: STEP_BUTTON, height: STEP_BUTTON, background: md('surfaceContainerHighest'), opacity: enabled ? 1 : DISABLED_ALPHA }}
      {...press}
    >
      <Text typography="titleLarge">{sign}</Text>
    </button>
  );
};

const StepperEntry = ({ label, unit, initial, range, onSet, onDismiss }: { label: string; unit: string; initial: string; range: ClosedRange | null; onSet: (value: number) => void; onDismiss: () => void }) => {
  const [text, setText] = useState(initial);
  const wanted = typedNumber(text);
  const typed = wanted === null ? null : range ? coerceIn(wanted, range.start, range.endInclusive) : wanted;
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title={label}
      text={<OutlinedTextField value={text} onValueChange={setText} suffix={unit.trim() !== '' ? unit : undefined} singleLine keyboardType="decimal" />}
      confirmButton={
        <TextButton
          enabled={typed !== null}
          onClick={() => {
            onDismiss();
            if (typed !== null) onSet(typed);
          }}
        >
          Set
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

export const SettingStepper = ({
  label,
  value,
  unit,
  step,
  onSet,
  className,
  note = null,
  range = null,
  slider = false,
  enabled = true,
  trailing = null,
}: {
  label: string;
  value: number | null;
  unit: string;
  step: number;
  onSet: (value: number) => void;
  className?: string;
  note?: string | null;
  range?: ClosedRange | null;
  slider?: boolean;
  enabled?: boolean;
  trailing?: ReactNode;
}) => {
  const [shown, setShownState] = useKeyedState<number | null>([value], () => value);
  const shownRef = useRef(shown);
  shownRef.current = shown;
  const setShown = (next: number | null) => {
    shownRef.current = next;
    setShownState(next);
  };
  const [typing, setTyping] = useState(false);
  const [stepped, setSteppedState] = useState<number | null>(null);
  const steppedRef = useRef<number | null>(null);
  const setStepped = (next: number | null) => {
    steppedRef.current = next;
    setSteppedState(next);
  };
  const commit = useUpdatedRef(onSet);
  useLaunchedEffect([stepped], async (scope) => {
    if (stepped === null) return;
    await delay(COMMIT_DELAY_MS);
    if (!scope.isActive()) return;
    setStepped(null);
    commit.current(stepped);
  });
  useEffect(
    () => () => {
      if (steppedRef.current !== null) commit.current(steppedRef.current);
    },
    [],
  );
  const decimals = stepperDecimals(shown, step);
  const set = (wanted: number) => {
    setShown(wanted);
    onSet(wanted);
  };
  const stepTo = (direction: number) => {
    const current = shownRef.current;
    if (current === null) return;
    const wanted = steppedValue(current, step, direction, range);
    setShown(wanted);
    setStepped(wanted);
  };
  return (
    <div className={`flex w-full flex-col py-1 ${className ?? ''}`}>
      <div className="flex min-h-14 w-full items-center">
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge">{label}</Text>
          {note !== null ? (
            <Text typography="labelSmall" color={md('onSurfaceVariant')}>
              {note}
            </Text>
          ) : null}
        </div>
        {trailing}
        <div className="flex items-center gap-1">
          <StepButton sign="−" description={`Decrease ${label}`} enabled={enabled && shown !== null} onStep={() => stepTo(-1)} />
          <button
            type="button"
            disabled={!enabled}
            onClick={() => setTyping(true)}
            className="flex items-end justify-center px-1"
            style={{ minWidth: VALUE_MIN_WIDTH }}
          >
            <Text typography="titleLarge" textAlign="center">
              {stepperText(shown, decimals)}
            </Text>
            {unit.trim() !== '' ? (
              <Text typography="labelMedium" color={md('onSurfaceVariant')}>
                {` ${unit}`}
              </Text>
            ) : null}
          </button>
          <StepButton sign="+" description={`Increase ${label}`} enabled={enabled && shown !== null} onStep={() => stepTo(1)} />
        </div>
      </div>
      {slider && range !== null ? (
        <Slider
          value={coerceIn(shown ?? range.start, range.start, range.endInclusive)}
          onValueChange={(it) => setShown(Math.round(it / step) * step)}
          onValueChangeFinished={() => {
            if (shownRef.current !== null) onSet(shownRef.current);
          }}
          valueRange={[range.start, range.endInclusive]}
          enabled={enabled}
          label={label}
        />
      ) : null}
      {typing ? (
        <StepperEntry
          label={label}
          unit={unit}
          initial={shown !== null ? stepperText(shown, decimals) : ''}
          range={range}
          onSet={set}
          onDismiss={() => setTyping(false)}
        />
      ) : null}
    </div>
  );
};

