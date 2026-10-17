import { type PointerEvent, type ReactNode, Fragment, createContext, useContext, useRef, useState } from 'react';
import { type Fact, type FactSlider, Qgc } from '@/bridge/Qgc';
import { AlertDialog, CircularProgressIndicator, DropdownMenu, DropdownMenuItem, HorizontalDivider, Text, TextButton } from '@/m3';
import { useKeyedState, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { coerceAtMost, coerceIn, delay } from '@/kotlin/Std';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { type ClosedRange, useRepeatWhileHeld } from '@/map/SettingStepper';
import { md, withAlpha } from '@/theme/Colors';
import { sliderValue } from '@/ui/ParameterForm';
import { factNumber } from '@/ui/Px4TuningScreen';
import { FieldSlider, showsAsField } from '@/ui/SettingsScreen';

export const VALUE_OFF = 'Off';
export const VALUE_NO_LIMIT = 'No limit';
const NO_LIMIT_NOTCH = 0.1;
const NO_LIMIT_SNAP = 0.25;
const SLIDER_SPAN_LIMIT = 100_000.0;
const DISABLED_AT_ZERO = /(disabled if 0|0 (disables|to disable|hides|turns .* off)|set to 0 to disable|zero disables)/i;

export const opensAsValue = (fact: Fact): boolean => showsAsField(fact) && !fact.isString && !fact.isBitmask && !fact.isEnum;

const numberOf = (fact: Fact): number | null => factNumber(fact);

export const disablesAtZero = (fact: Fact): boolean => DISABLED_AT_ZERO.test(`${fact.description} ${fact.longDescription}`);

export const valueText = (fact: Fact): string =>
  disablesAtZero(fact) && numberOf(fact) === 0
    ? offLabel(fact)
    : [fact.valueString.trim(), fact.units].filter((it) => it.trim() !== '').join(' ');

export const valueDecimals = (fact: Fact): number => {
  const text = fact.valueString.trim();
  const after = text.includes('.') ? text.slice(text.indexOf('.') + 1) : '';
  return (/^\d*/.exec(after)?.[0] ?? '').length;
};

export const valueStep = (fact: Fact): number => (USEFUL_BANDS.has(fact.name) ? metresScale(fact) : Math.pow(10, -valueDecimals(fact)));

const boundOf = (text: string, isDefaultForType: boolean): number | null => {
  const bound = toDoubleOrNull(text);
  return bound !== null && !isDefaultForType && Number.isFinite(bound) ? bound : null;
};

export const valueBounds = (fact: Fact): readonly [number | null, number | null] => [boundOf(fact.minString, fact.minIsDefaultForType), boundOf(fact.maxString, fact.maxIsDefaultForType)];

export const derivedSlider = (fact: Fact): FactSlider | null => {
  const [min, max] = valueBounds(fact);
  return min !== null && max !== null && max > min && max - min <= SLIDER_SPAN_LIMIT ? { from: min, to: max, decimals: valueDecimals(fact), hint: '' } : null;
};

const ALTITUDE_BAND: ClosedRange = { start: 20.0, endInclusive: 500.0 };
const DISTANCE_BAND: ClosedRange = { start: 50.0, endInclusive: 5000.0 };
const ALTITUDE_ON = 120.0;
const DISTANCE_ON = 500.0;

const USEFUL_BANDS: ReadonlyMap<string, ClosedRange> = new Map([
  ['RTL_RETURN_ALT', ALTITUDE_BAND],
  ['RTL_ALT', ALTITUDE_BAND],
  ['RTL_ALT_M', ALTITUDE_BAND],
  ['GF_MAX_VER_DIST', ALTITUDE_BAND],
  ['FENCE_ALT_MAX', ALTITUDE_BAND],
  ['GF_MAX_HOR_DIST', DISTANCE_BAND],
  ['FENCE_RADIUS', DISTANCE_BAND],
]);

const TURN_ON_VALUES: ReadonlyMap<string, number> = new Map([
  ['GF_MAX_VER_DIST', ALTITUDE_ON],
  ['FENCE_ALT_MAX', ALTITUDE_ON],
  ['GF_MAX_HOR_DIST', DISTANCE_ON],
  ['FENCE_RADIUS', DISTANCE_ON],
]);

const metresScale = (fact: Fact): number => (fact.units.trim() === 'cm' ? 100.0 : 1.0);

const clampToBounds = (fact: Fact, value: number): number => {
  const [min, max] = valueBounds(fact);
  return coerceIn(value, min ?? Number.NEGATIVE_INFINITY, max ?? Number.POSITIVE_INFINITY);
};

const bandSlider = (fact: Fact): FactSlider | null => {
  const band = USEFUL_BANDS.get(fact.name);
  if (!band) return null;
  const from = clampToBounds(fact, band.start * metresScale(fact));
  const to = clampToBounds(fact, band.endInclusive * metresScale(fact));
  return to > from ? { from, to, decimals: valueDecimals(fact), hint: '' } : null;
};

export const sliderFor = (fact: Fact): FactSlider | null => bandSlider(fact) ?? fact.slider ?? derivedSlider(fact);

export const offLabel = (fact: Fact): string => (USEFUL_BANDS.has(fact.name) ? VALUE_NO_LIMIT : VALUE_OFF);

export type InlineSlider = { from: number; top: number; noLimitAt: number | null; end: number };

export const InlineSlider = (fields: { from: number; top: number; noLimitAt: number | null }): InlineSlider => ({ ...fields, end: fields.noLimitAt ?? fields.top });

export const inlineSlider = (fact: Fact): InlineSlider | null => {
  const slider = sliderFor(fact);
  return slider === null
    ? null
    : InlineSlider({ from: slider.from, top: slider.to, noLimitAt: disablesAtZero(fact) ? slider.to + (slider.to - slider.from) * NO_LIMIT_NOTCH : null });
};

export const sliderPosition = (slider: InlineSlider, value: number | null): number =>
  slider.noLimitAt !== null && value === 0 ? slider.noLimitAt : coerceIn(value ?? slider.from, slider.from, slider.top);

export const valueAtPosition = (fact: Fact, slider: InlineSlider, position: number): number =>
  slider.noLimitAt !== null && position > slider.top + (slider.noLimitAt - slider.top) * NO_LIMIT_SNAP
    ? 0.0
    : clampToBounds(fact, roundedValue(fact, coerceAtMost(position, slider.top)));

export const writeValueRefusal = (fact: Fact, value: number): Promise<string | null> => {
  const rounded = roundedValue(fact, value);
  return Qgc.writeRefusal(fact.path, valueDecimals(fact) === 0 ? Math.trunc(rounded) : rounded);
};

export type ChangeNotice = { show: (message: string, undo: (() => Promise<string | null>) | null) => void };

export const LocalChangeNotice = createContext<ChangeNotice>({ show: () => {} });

const DONE_MARK_MS = 3000;

export const SliderValueRow = ({ fact, label, slider, title, onOpen, onWrite }: { fact: Fact; label: string; slider: InlineSlider; title: ReactNode; onOpen: () => void; onWrite: () => void }) => {
  const notice = useContext(LocalChangeNotice);
  const [dragging, setDraggingState] = useKeyedState<number | null>([fact.path, fact.valueString], () => null);
  const draggingRef = useRef(dragging);
  draggingRef.current = dragging;
  const setDragging = (next: number | null) => {
    draggingRef.current = next;
    setDraggingState(next);
  };
  const [writing, setWriting] = useKeyedState([fact.path], () => false);
  const [done, setDone] = useKeyedState([fact.path], () => false);
  useLaunchedEffect([done], async (scope) => {
    if (!done) return;
    await delay(DONE_MARK_MS);
    if (scope.isActive()) setDone(false);
  });
  const shownValue = dragging !== null ? valueAtPosition(fact, slider, dragging) : numberOf(fact);
  const commit = () => {
    const before = numberOf(fact);
    const held = draggingRef.current;
    const target = held !== null ? valueAtPosition(fact, slider, held) : null;
    if (target === null || target === before) {
      setDragging(null);
      return;
    }
    void (async () => {
      setWriting(true);
      const refusal = await writeValueRefusal(fact, target);
      setWriting(false);
      if (refusal === null) {
        setDone(true);
        onWrite();
        notice.show(
          `${label} ${valueTextOf(fact, target)}`,
          before === null
            ? null
            : async () => {
                const undone = await writeValueRefusal(fact, before);
                if (undone === null) onWrite();
                return undone;
              },
        );
      } else {
        setDragging(null);
        notice.show(`${label} not changed: ${refusal}`, null);
      }
    })();
  };
  return (
    <div className="@container w-full px-4 py-1.5">
      <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center @min-[560px]:grid-cols-[minmax(0,1fr)_auto_240px] @min-[560px]:gap-x-3">
        <div className="min-w-0">{title}</div>
        <div className="flex items-center gap-1.5">
          {writing ? <CircularProgressIndicator size={14} strokeWidth={2} /> : done ? (
            <Text typography="bodyLarge" color={md('primary')}>
              {'✓'}
            </Text>
          ) : null}
          <button type="button" onClick={onOpen} title="Set exactly" className="px-1 py-2">
            <Text typography="bodyLarge" color={md('onSurfaceVariant')}>
              {valueTextOf(fact, shownValue)}
            </Text>
          </button>
        </div>
        <ThinSlider
          value={dragging ?? sliderPosition(slider, numberOf(fact))}
          range={{ start: slider.from, endInclusive: slider.end }}
          enabled={fact.acceptsWrite && !writing}
          onChange={setDragging}
          onDone={commit}
          label={label}
          className="col-span-2 @min-[560px]:col-span-1"
        />
      </div>
    </div>
  );
};

const THUMB_SIZE = 18;
const THUMB_GRAB = 24;
const TRACK_HEIGHT = 2;
const SLIDER_HEIGHT = 48;
const TOUCH_SLOP = 8;

export const thumbFraction = (value: number, range: ClosedRange): number => {
  const span = range.endInclusive - range.start;
  return span > 0 ? coerceIn((value - range.start) / span, 0, 1) : 0;
};

export const grabsThumb = (touchX: number, thumbX: number, grabRadius: number): boolean => Math.abs(touchX - thumbX) <= grabRadius;

const ThinSlider = ({ value, range, enabled, onChange, onDone, label, className }: { value: number; range: ClosedRange; enabled: boolean; onChange: (value: number) => void; onDone: () => void; label: string; className?: string }) => {
  const active = withAlpha(md('onSurface'), enabled ? 1 : DISABLED_SLIDER_ALPHA);
  const inactive = md('outlineVariant');
  const latest = useUpdatedRef(value);
  const change = useUpdatedRef(onChange);
  const finish = useUpdatedRef(onDone);
  const gesture = useRef<{ id: number; startX: number; dragging: boolean } | null>(null);
  const travelOf = (box: DOMRect) => Math.max(box.width - THUMB_SIZE, 1);
  const positionAt = (box: DOMRect, clientX: number) => range.start + coerceIn((clientX - box.left - THUMB_SIZE / 2) / travelOf(box), 0, 1) * (range.endInclusive - range.start);
  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (!enabled) return;
    const box = event.currentTarget.getBoundingClientRect();
    if (!grabsThumb(event.clientX - box.left, THUMB_SIZE / 2 + travelOf(box) * thumbFraction(latest.current, range), THUMB_GRAB)) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = { id: event.pointerId, startX: event.clientX, dragging: false };
  };
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const held = gesture.current;
    if (held === null || held.id !== event.pointerId) return;
    if (!held.dragging && Math.abs(event.clientX - held.startX) <= TOUCH_SLOP) return;
    gesture.current = { ...held, dragging: true };
    change.current(positionAt(event.currentTarget.getBoundingClientRect(), event.clientX));
  };
  const onPointerEnd = (event: PointerEvent<HTMLDivElement>) => {
    const held = gesture.current;
    if (held === null || held.id !== event.pointerId) return;
    gesture.current = null;
    if (held.dragging) finish.current();
  };
  const fraction = thumbFraction(value, range);
  const span = range.endInclusive - range.start;
  return (
    <div
      className={`relative touch-pan-y select-none ${className ?? ''}`}
      style={{ height: SLIDER_HEIGHT }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
    >
      <span className="absolute top-1/2 -translate-y-1/2" style={{ left: THUMB_SIZE / 2, right: THUMB_SIZE / 2, height: TRACK_HEIGHT, background: inactive }} />
      <span className="absolute top-1/2 -translate-y-1/2" style={{ left: THUMB_SIZE / 2, width: `calc((100% - ${THUMB_SIZE}px) * ${fraction})`, height: TRACK_HEIGHT, background: active }} />
      <span
        className="absolute top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full"
        style={{ left: `calc(${THUMB_SIZE / 2}px + (100% - ${THUMB_SIZE}px) * ${fraction})`, width: THUMB_SIZE, height: THUMB_SIZE, background: active }}
      />
      <input
        type="range"
        className="sr-only"
        aria-label={label}
        min={range.start}
        max={range.endInclusive}
        step={span > 0 ? span / 100 : 'any'}
        value={value}
        disabled={!enabled}
        onChange={(event) => {
          change.current(coerceIn(Number(event.target.value), range.start, range.endInclusive));
          finish.current();
        }}
      />
    </div>
  );
};

const DISABLED_SLIDER_ALPHA = 0.4;

const DANGEROUS_CHOICE = /(terminat|disarm|stop motors|kill)/i;

export const dangerousChoice = (label: string): boolean => DANGEROUS_CHOICE.test(label);

export const safeFirst = (options: readonly string[]): number[] =>
  options.map((_, index) => index).toSorted((a, b) => Number(dangerousChoice(options[a] ?? '')) - Number(dangerousChoice(options[b] ?? '')));

export const DANGER_NOTE = 'Motors stop — the aircraft falls';

export const SafeChoiceMenu = ({ expanded, options, onDismiss, onPick }: { expanded: boolean; options: readonly string[]; onDismiss: () => void; onPick: (index: number) => void }) => {
  const [confirming, setConfirming] = useState<number | null>(null);
  const order = safeFirst(options);
  const firstDanger = order.find((it) => dangerousChoice(options[it] ?? ''));
  return (
    <>
      <DropdownMenu expanded={expanded} onDismissRequest={onDismiss}>
        {order.map((index) => {
          const option = options[index] ?? '';
          const danger = dangerousChoice(option);
          return (
            <Fragment key={index}>
              {danger && index === firstDanger ? <HorizontalDivider /> : null}
              <DropdownMenuItem
                text={
                  <span className="flex flex-col">
                    <span style={danger ? { color: md('error') } : undefined}>{option}</span>
                    {danger ? (
                      <Text typography="bodySmall" color={md('error')}>
                        {DANGER_NOTE}
                      </Text>
                    ) : null}
                  </span>
                }
                onClick={() => {
                  onDismiss();
                  if (danger) setConfirming(index);
                  else onPick(index);
                }}
              />
            </Fragment>
          );
        })}
      </DropdownMenu>
      {confirming !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(null)}
          title={`${options[confirming] ?? ''}?`}
          text="If this happens in flight, the motors stop and the aircraft falls. Only choose it if a falling aircraft is safer than a flying one."
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(null);
                onPick(confirming);
              }}
            >
              <span style={{ color: md('error') }}>Choose it</span>
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(null)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};

export const turnOnValue = (fact: Fact): number => {
  const parsed = toDoubleOrNull(fact.defaultValueString);
  const fallback = parsed !== null && parsed > 0 ? parsed : null;
  const named = TURN_ON_VALUES.get(fact.name);
  const minimum = valueBounds(fact)[0];
  const smallest = minimum !== null && minimum > 0 ? minimum : valueStep(fact);
  return clampToBounds(fact, fallback ?? (named === undefined ? null : named * metresScale(fact)) ?? smallest);
};

const roundHalfEven = (value: number): number => (value - Math.floor(value) === 0.5 ? 2 * Math.round(value / 2) : Math.round(value));

export const roundedValue = (fact: Fact, value: number): number => roundHalfEven(value / valueStep(fact)) * valueStep(fact);

const FAST_AFTER_REPEATS = 10;
const FAST_STEP_FACTOR = 10;

export const nudged = (fact: Fact, from: number, direction: number, repeats: number): number =>
  disablesAtZero(fact) && from === 0 && direction > 0
    ? turnOnValue(fact)
    : clampToBounds(fact, roundedValue(fact, from + direction * valueStep(fact) * (repeats >= FAST_AFTER_REPEATS ? FAST_STEP_FACTOR : 1)));

export const steppedValue = (fact: Fact, direction: number): number | null => {
  const current = numberOf(fact);
  return current === null ? null : nudged(fact, current, direction, 0);
};

export const valueTextOf = (fact: Fact, value: number | null): string => {
  if (value === null) return valueText(fact);
  if (disablesAtZero(fact) && value === 0) return offLabel(fact);
  return sliderValue(value, valueDecimals(fact), fact.units.trim());
};

const HOLD_DELAY_MS = 400;
const REPEAT_MS = 90;

const StepButton = ({ label, description, enabled, onStep, onRelease }: { label: string; description: string; enabled: boolean; onStep: (repeats: number) => void; onRelease: () => void }) => {
  const press = useRepeatWhileHeld(enabled, onStep, HOLD_DELAY_MS, REPEAT_MS, onRelease);
  return (
    <button type="button" aria-label={description} className="grid size-12 shrink-0 touch-none select-none place-items-center rounded-full bg-secondary-container text-on-secondary-container" {...press}>
      <Text typography="titleLarge">{label}</Text>
    </button>
  );
};

export const ValueControls = ({ fact, onWrite, typedEntry }: { fact: Fact; onWrite: () => void; typedEntry: ReactNode }) => {
  const [refusal, setRefusal] = useKeyedState<string | null>([fact.path], () => null);
  const [pending, setPendingState] = useKeyedState<number | null>([fact.path, fact.valueString], () => null);
  const pendingRef = useRef(pending);
  pendingRef.current = pending;
  const setPending = (next: number | null) => {
    pendingRef.current = next;
    setPendingState(next);
  };
  const [typing, setTyping] = useKeyedState([fact.path], () => false);
  const write = (value: number) => {
    void (async () => {
      const refused = await writeValueRefusal(fact, value);
      setRefusal(refused);
      if (refused === null) onWrite();
      else setPending(null);
    })();
  };
  const shown = pending ?? numberOf(fact);
  const commit = () => {
    const held = pendingRef.current;
    if (held !== null && held !== numberOf(fact)) write(held);
  };
  const nudge = (direction: number) => (repeats: number) => {
    const from = pendingRef.current ?? numberOf(fact);
    if (from !== null) setPending(nudged(fact, from, direction, repeats));
  };
  const slider = sliderFor(fact);
  const limit = offLabel(fact) === VALUE_NO_LIMIT;
  return (
    <>
      <div className="flex w-full items-center justify-center gap-4">
        <StepButton label={'−'} description="Decrease" enabled={fact.acceptsWrite} onStep={nudge(-1)} onRelease={commit} />
        <button type="button" onClick={() => setTyping(!typing)} title="Type a value" className="px-2 py-1">
          <Text typography="headlineSmall">{valueTextOf(fact, shown)}</Text>
        </button>
        <StepButton label="+" description="Increase" enabled={fact.acceptsWrite} onStep={nudge(1)} onRelease={commit} />
      </div>
      {slider !== null && !(disablesAtZero(fact) && shown === 0) ? <FieldSlider value={shown} slider={slider} enabled={fact.acceptsWrite} onWrite={write} /> : null}
      {disablesAtZero(fact) ? (
        shown === 0 ? (
          <TextButton onClick={() => write(turnOnValue(fact))}>{limit ? 'Set a limit' : 'Turn on'}</TextButton>
        ) : (
          <TextButton onClick={() => write(0.0)}>{limit ? 'Remove the limit' : 'Turn off'}</TextButton>
        )
      ) : null}
      {typing ? typedEntry : null}
      {refusal !== null ? (
        <Text typography="bodySmall" color={md('error')}>
          {refusal}
        </Text>
      ) : null}
    </>
  );
};
