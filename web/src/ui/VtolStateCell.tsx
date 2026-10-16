import { useMemo, useState } from 'react';
import { type Json, optBoolean } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Text, TextButton } from '@/m3';
import { GUIDED_ACTIONS, type GuidedOffer, guidedOffers } from './GuidedActions';
import { guidedCommand } from './MoreActions';

export type VtolState = { forward: boolean; flying: boolean; label: string; transition: string };

export const VtolState = (forward: boolean, flying: boolean): VtolState => ({
  forward,
  flying,
  label: forward ? 'FW(vtol)' : 'MR(vtol)',
  transition: forward ? 'vtolTransitionToMultiRotor' : 'vtolTransitionToFixedWing',
});

export const vtolState = (view: Json | null): VtolState | null => (view !== null && optBoolean(view, 'vtol') ? VtolState(optBoolean(view, 'vtolInFwdFlight'), optBoolean(view, 'flying')) : null);

export const VtolStateCell = () => {
  const view = useQgcPath(GUIDED_ACTIONS);
  const state = useMemo(() => vtolState(view), [view]);
  const offer = useMemo(() => (state === null ? null : (guidedOffers(view).get(state.transition) ?? null)), [view, state]);
  return state === null ? null : <VtolCell state={state} offer={offer} />;
};

const VtolCell = ({ state, offer }: { state: VtolState; offer: GuidedOffer | null }) => {
  const [confirming, setConfirming] = useState(false);
  const transitionText = state.forward ? 'Transition to Multi-Rotor' : 'Transition to Fixed Wing';
  const command = confirming ? guidedCommand(state.transition, null) : null;
  return (
    <>
      {state.flying ? (
        <button type="button" onClick={() => setConfirming(true)} className="text-left">
          <Text typography="titleMedium">{state.label}</Text>
        </button>
      ) : (
        <Text typography="labelLarge">{state.label}</Text>
      )}
      {confirming ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title={offer !== null && offer.title.trim() !== '' ? offer.title : transitionText}
          text={offer === null ? undefined : offer.prompt}
          confirmButton={
            <TextButton
              enabled={offer?.ready === true && command !== null}
              onClick={() => {
                setConfirming(false);
                command?.();
              }}
            >
              {transitionText}
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};
