import { type Json, isNull, jsonObjects, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import type { DrawableName } from '@/res/drawables';
import { trimmedNumber } from './CameraSectionExtras';

export type ActionStep = {
  readonly kind: string;
  readonly title: string;
  readonly seconds: number | null;
  readonly item: number | null;
  readonly places: readonly number[];
  readonly refusal: string | null;
};

export const actionSteps = (view: Json | null): readonly ActionStep[] =>
  jsonObjects(optJSONArray(view, 'actions')).map((step) => ({
    kind: optText(step, 'kind'),
    title: optText(step, 'title'),
    seconds: isNull(step, 'seconds') ? null : optDouble(step, 'seconds'),
    item: isNull(step, 'item') ? null : optInt(step, 'item'),
    places: (optJSONArray(step, 'places') ?? []).filter((it): it is number => typeof it === 'number' && Number.isInteger(it) && it >= 0),
    refusal: isNull(step, 'refusal') || optText(step, 'refusal').trim() === '' ? null : optText(step, 'refusal'),
  }));

export type ShownAction<A> = {
  readonly action: A | null;
  readonly kind: string;
  readonly title: string;
  readonly value: string;
  readonly icon: DrawableName;
  readonly place: number | null;
  readonly item: number | null;
  readonly seconds: number | null;
  readonly places: readonly number[];
  readonly refusal: string | null;
};

export type ActiveAction = { readonly kind: { readonly name: string }; readonly title: string; readonly value: string; readonly icon: DrawableName };

export const HOVER = 'Hover';
export const WAIT = 'Wait';
const WAIT_ICON: DrawableName = 'ic_pause';
const COMMAND_ICON: DrawableName = 'plan_tune';

const secondsText = (seconds: number | null): string => (seconds === null ? '' : `${trimmedNumber(seconds)} s`);

export const orderedActions = <A extends ActiveAction>(steps: readonly ActionStep[], active: readonly A[]): readonly ShownAction<A>[] => {
  const listed = steps.map((step, place): ShownAction<A> => {
    const action = active.find((it) => it.kind.name === step.kind) ?? null;
    return {
      action,
      kind: step.kind,
      title: action?.title ?? step.title,
      value: action?.value ?? secondsText(step.seconds),
      icon: action?.icon ?? (step.kind === WAIT ? WAIT_ICON : COMMAND_ICON),
      place,
      item: step.item,
      seconds: step.seconds,
      places: step.places,
      refusal: step.refusal,
    };
  });
  const fixed = active
    .filter((it) => !steps.some((step) => step.kind === it.kind.name))
    .map((action): ShownAction<A> => ({ action, kind: action.kind.name, title: action.title, value: action.value, icon: action.icon, place: null, item: null, seconds: null, places: [], refusal: null }));
  return [...listed, ...fixed];
};

export const movedOrder = <T>(list: readonly T[], from: number, to: number): readonly T[] => {
  const moving = list[from];
  if (moving === undefined || to < 0 || to >= list.length) return list;
  const without = list.filter((_, at) => at !== from);
  return [...without.slice(0, to), moving, ...without.slice(to)];
};

export const canDrop = <A>(step: ShownAction<A> | undefined, to: number): boolean => step !== undefined && step.place !== null && step.places.includes(to);

export const projectedPlaces = <A>(steps: readonly ShownAction<A>[], from: number | null, over: number | null): readonly number[] => {
  const identity = steps.map((_, at) => at);
  if (from === null || over === null || !canDrop(steps[from], over)) return identity;
  const order = movedOrder(identity, from, over);
  return identity.map((at) => order.indexOf(at));
};

export const shownAt = <A>(step: ShownAction<A>, place: number): ShownAction<A> =>
  step.kind === HOVER && place > 0 ? { ...step, title: WAIT, icon: WAIT_ICON, value: secondsText(step.seconds) } : step;

export const dropMove = <A>(waypoint: number, step: ShownAction<A> | undefined, to: number | null): string | null =>
  to === null || step === undefined || step.place === null || step.place === to || !canDrop(step, to) ? null : JSON.stringify([waypoint, step.place, to]);

export const stepKeys = <A>(steps: readonly ShownAction<A>[]): readonly string[] =>
  steps.map((step, at) => `${step.kind}:${step.title}:${steps.slice(0, at).filter((it) => it.kind === step.kind && it.title === step.title).length}`);

export const orderedCount = <A>(steps: readonly ShownAction<A>[]): number => steps.filter((it) => it.place !== null).length;

export const announcements = {
  lifted: (title: string, place: number, count: number): string => `Picked up ${title}, action ${place + 1} of ${count}. Use the up and down arrows to move it, Space to drop it, Escape to cancel.`,
  over: (title: string, place: number, count: number, refusal: string | null, allowed: boolean): string =>
    allowed ? `${title} would be action ${place + 1} of ${count}.` : `${title} cannot go to place ${place + 1}. ${refusal ?? ''}`.trim(),
  dropped: (title: string, place: number, count: number, moved: boolean): string => (moved ? `${title} dropped as action ${place + 1} of ${count}.` : `${title} put back where it was.`),
  cancelled: (title: string): string => `Moving ${title} was cancelled. It is back where it was.`,
};
