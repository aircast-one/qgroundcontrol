import { type Json, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';

export const GUIDED_ACTIONS = 'view.guidedActions';

export type GuidedOffer = {
  readonly id: string;
  readonly title: string;
  readonly offer: string;
  readonly reason: string;
  readonly prompt: string;
  readonly destructive: boolean;
  readonly carriesValue: boolean;
  readonly option: string;
  readonly reasonCode: string;
  readonly confirm: boolean;
  readonly ready: boolean;
  readonly shown: boolean;
};

export const GuidedOffer = (fields: {
  id: string;
  title: string;
  offer: string;
  reason: string;
  prompt: string;
  destructive: boolean;
  carriesValue: boolean;
  option?: string;
  reasonCode?: string;
  confirm?: boolean;
}): GuidedOffer => ({
  option: '',
  reasonCode: '',
  confirm: true,
  ...fields,
  ready: fields.offer === 'ready',
  shown: fields.offer !== 'hidden',
});

export const guidedOffers = (view: Json | null): ReadonlyMap<string, GuidedOffer> =>
  new Map(
    jsonObjects(optJSONArray(view, 'actions'))
      .filter((action) => optText(action, 'id').trim() !== '')
      .map((action) => {
        const id = optText(action, 'id');
        return [
          id,
          GuidedOffer({
            id,
            title: optText(action, 'title'),
            offer: optText(action, 'offer'),
            reason: optText(action, 'reason'),
            prompt: optText(action, 'prompt'),
            destructive: optBoolean(action, 'destructive'),
            carriesValue: optBoolean(action, 'carriesValue'),
            reasonCode: optText(action, 'reasonCode'),
            option: optText(action, 'option'),
            confirm: optBoolean(action, 'confirm', true),
          }),
        ] as const;
      }),
  );

export type Payload = { readonly bays: number; readonly empty: readonly number[]; readonly next: number | null };

export const SINGLE_GRIPPER: Payload = { bays: 1, empty: [], next: null };

export const payloadOf = (view: Json | null): Payload => {
  const payload = optJSONObject(view, 'payload');
  if (payload === null) return SINGLE_GRIPPER;
  const bays = Math.max(1, optInt(payload, 'bays', 1));
  return { bays, empty: (optJSONArray(payload, 'empty') ?? []).filter((it): it is number => typeof it === 'number'), next: isNull(payload, 'next') ? null : optInt(payload, 'next') };
};

export const multiBay = (payload: Payload): boolean => payload.bays > 1;

export const bayNumbers = (payload: Payload): readonly number[] => Array.from({ length: payload.bays }, (_, index) => index + 1);

export const resumeFromSequence = (view: Json | null): number | null => {
  if (view === null || isNull(view, 'resumeFromSequence')) return null;
  const sequence = optInt(view, 'resumeFromSequence');
  return sequence > 0 ? sequence : null;
};
