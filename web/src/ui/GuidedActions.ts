import { type Json, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';

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

export const resumeFromSequence = (view: Json | null): number | null => {
  if (view === null || isNull(view, 'resumeFromSequence')) return null;
  const sequence = optInt(view, 'resumeFromSequence');
  return sequence > 0 ? sequence : null;
};
