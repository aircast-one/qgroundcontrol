import { useState } from 'react';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { type Json, optInt, optText } from '@/bridge/Json';
import { useQgcValue } from '@/bridge/QgcState';
import { useTicker } from '@/kotlin/Compose';
import { OutlinedButton, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { md } from '@/theme/Colors';
import { type GuidedOffer, type Payload, bayNumbers, multiBay } from './GuidedActions';
import { HoldToConfirm, holdLabel } from './HoldToConfirm';
import { GRIPPER_GRAB, GRIPPER_RELEASE, guidedCommand } from './MoreActions';

export const GRIPPER_ACTIONS: readonly string[] = ['release', 'grab', 'hold'];
export const PAYLOAD_STATE = 'vehicle.gripper';
const STATUS_TICK_MS = 1000;

export const gripperOffers = (extras: readonly GuidedOffer[]): readonly GuidedOffer[] => GRIPPER_ACTIONS.flatMap((id) => extras.find((it) => it.id === id) ?? []);

export type PayloadStatus = { readonly text: string; readonly warning: boolean };

export const agoText = (atMs: number, nowMs: number): string => {
  const seconds = Math.max(0, Math.floor((nowMs - atMs) / 1000));
  if (seconds < 60) return `${seconds} s ago`;
  const minutes = Math.floor(seconds / 60);
  return minutes < 60 ? `${minutes} min ago` : `${Math.floor(minutes / 60)} h ago`;
};

const verbs = (action: string): readonly [string, string] => (action === 'release' ? ['Drop', 'Dropped'] : action === 'grab' ? ['Grab', 'Grabbed'] : ['Hold', 'Holding']);

export const payloadStatus = (json: Json | null, nowMs: number): PayloadStatus | null => {
  if (json === null) return null;
  const action = optText(json, 'action');
  const bay = optInt(json, 'bay');
  const [plainVerb, plainDone] = verbs(action);
  const [verb, done] = bay > 0 ? [`${plainVerb} ${bay}`, `Bay ${bay} ${plainDone.toLowerCase()}`] : [plainVerb, plainDone];
  const ago = agoText(optInt(json, 'atMs'), nowMs);
  switch (optText(json, 'outcome')) {
    case 'sent':
      return { text: `${verb} sent, waiting for the aircraft`, warning: false };
    case 'confirmed':
      return { text: `${done} ${ago}`, warning: false };
    case 'refused':
      return { text: `${verb} refused by the flight controller ${ago}`, warning: true };
    case 'unanswered':
      return { text: action === 'release' ? `${verb} not confirmed ${ago}. It may have dropped, check before dropping again.` : `${verb} not confirmed ${ago}. Check the gripper before trying again.`, warning: true };
    default:
      return null;
  }
};

export const bayLabel = (bay: number, payload: Payload): string => (payload.empty.includes(bay) ? `Bay ${bay} empty, slide to drop` : `Slide to drop bay ${bay}`);

const offerReady = (offers: readonly GuidedOffer[], id: string): boolean => offers.some((it) => it.id === id && it.ready);

const SingleGripper = ({ offers }: { offers: readonly GuidedOffer[] }) => (
  <>
    {offers
      .filter((it) => it.id === 'release')
      .map((offer) => (
        <HoldToConfirm key={offer.id} label={holdLabel(offer.title)} enabled={offer.ready} onConfirm={() => guidedCommand(offer.id, null)?.()} />
      ))}
    <div className="flex gap-2">
      {offers
        .filter((it) => it.id !== 'release')
        .map((offer) => (
          <OutlinedButton key={offer.id} enabled={offer.ready} onClick={() => guidedCommand(offer.id, null)?.()}>
            {offer.title}
          </OutlinedButton>
        ))}
    </div>
  </>
);

const Bays = ({ offers, payload }: { offers: readonly GuidedOffer[]; payload: Payload }) => (
  <>
    {bayNumbers(payload).map((bay) => (
      <div key={bay} className="flex items-center gap-2">
        <HoldToConfirm className="flex-1" label={bayLabel(bay, payload)} enabled={offerReady(offers, 'release')} onConfirm={() => void VehicleCommands.gripper(GRIPPER_RELEASE, bay)} />
        <OutlinedButton enabled={offerReady(offers, 'grab')} label={`Grab bay ${bay}`} onClick={() => void VehicleCommands.gripper(GRIPPER_GRAB, bay)}>
          Grab
        </OutlinedButton>
      </div>
    ))}
  </>
);

export const GripperPanel = ({ offers, payload, onDismiss }: { offers: readonly GuidedOffer[]; payload: Payload; onDismiss: () => void }) => {
  const stateJson = useQgcValue(PAYLOAD_STATE) as Json | null;
  const [now, setNow] = useState(() => Date.now());
  useTicker(STATUS_TICK_MS, [], () => setNow(Date.now()));
  const status = payloadStatus(stateJson, now);
  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <div className="flex w-full flex-col gap-3 px-sheet pb-6">
        <Text typography="bodyMedium" color={status?.warning === true ? md('error') : md('onSurfaceVariant')}>
          {status?.text ?? 'No payload command sent yet'}
        </Text>
        {multiBay(payload) ? <Bays offers={offers} payload={payload} /> : <SingleGripper offers={offers} />}
      </div>
    </AircastSheet>
  );
};
