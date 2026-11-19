import { useState } from 'react';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { type Json, optInt, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath, useQgcValue } from '@/bridge/QgcState';
import { useTicker } from '@/kotlin/Compose';
import { OutlinedButton, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { md } from '@/theme/Colors';
import { type GuidedOffer, type Payload, type PayloadProblem, bayNumbers, bayOutput, multiBay } from './GuidedActions';
import { HoldToConfirm, holdLabel } from './HoldToConfirm';
import { GRIPPER_GRAB, GRIPPER_HOLD, GRIPPER_RELEASE } from './MoreActions';
import { PAYLOAD_RAIL_ARM, PAYLOAD_RAIL_VIEW, type PayloadRail, payloadRailOf } from './PayloadRail';

export const GRIPPER_ACTIONS: readonly string[] = ['release', 'grab', 'hold'];
export const PAYLOAD_STATE = 'vehicle.gripper';
const STATUS_TICK_MS = 1000;
const actionCode = (id: string): number => (id === 'release' ? GRIPPER_RELEASE : id === 'hold' ? GRIPPER_HOLD : GRIPPER_GRAB);

type Refused = (reason: string | null) => void;

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
  const [verb, done] = bay > 0 ? [`${plainVerb} bay ${bay}`, `Bay ${bay} ${plainDone.toLowerCase()}`] : [plainVerb, plainDone];
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

const send = (action: number, bay: number, onRefused: Refused) => void VehicleCommands.gripperRefusal(action, bay).then(onRefused);

export type PayloadLock = { readonly text: string; readonly button: string; readonly arms: boolean };

export const payloadLock = (rail: PayloadRail | null): PayloadLock | null =>
  rail === null ? null : rail.armed ? { text: 'The payload is armed.', button: 'Make safe', arms: false } : { text: 'The payload is safe. Arm it to drop.', button: 'Arm payload', arms: true };

const LockRow = ({ lock, onRefused }: { lock: PayloadLock; onRefused: Refused }) => (
  <div className="flex items-center gap-2">
    <Text className="flex-1" typography="bodyMedium" color={lock.arms ? md('onSurfaceVariant') : md('error')}>
      {lock.text}
    </Text>
    <OutlinedButton onClick={() => void Qgc.refusalOf(PAYLOAD_RAIL_ARM, lock.arms).then(onRefused)}>{lock.button}</OutlinedButton>
  </div>
);

const SingleGripper = ({ offers, safe, onRefused }: { offers: readonly GuidedOffer[]; safe: boolean; onRefused: Refused }) => (
  <>
    {offers
      .filter((it) => it.id === 'release')
      .map((offer) => (
        <HoldToConfirm key={offer.id} label={holdLabel(offer.title)} enabled={offer.ready && !safe} onConfirm={() => send(GRIPPER_RELEASE, 0, onRefused)} />
      ))}
    <div className="flex gap-2">
      {offers
        .filter((it) => it.id !== 'release')
        .map((offer) => (
          <OutlinedButton key={offer.id} enabled={offer.ready} onClick={() => send(actionCode(offer.id), 0, onRefused)}>
            {offer.title}
          </OutlinedButton>
        ))}
    </div>
  </>
);

const Bays = ({ offers, payload, safe, onRefused }: { offers: readonly GuidedOffer[]; payload: Payload; safe: boolean; onRefused: Refused }) => (
  <>
    {bayNumbers(payload).map((bay) => (
      <div key={bay} className="flex flex-col gap-1">
        {bayOutput(bay, payload) === '' ? null : (
          <Text typography="labelMedium" color={md('onSurfaceVariant')}>
            {bayOutput(bay, payload)}
          </Text>
        )}
        <div className="flex items-center gap-2">
          <HoldToConfirm className="flex-1" label={bayLabel(bay, payload)} enabled={offerReady(offers, 'release') && !safe} onConfirm={() => send(GRIPPER_RELEASE, bay, onRefused)} />
          <OutlinedButton enabled={offerReady(offers, 'grab')} label={`Grab bay ${bay}`} onClick={() => send(GRIPPER_GRAB, bay, onRefused)}>
            Grab
          </OutlinedButton>
        </div>
      </div>
    ))}
  </>
);

export const PayloadFixButton = ({ problem, onRefused }: { problem: PayloadProblem; onRefused: Refused }) =>
  problem.fix === '' ? null : <OutlinedButton onClick={() => void VehicleCommands.fixPayloadRefusal(problem.code).then(onRefused)}>{problem.fix}</OutlinedButton>;

const Problems = ({ problems, onRefused }: { problems: readonly PayloadProblem[]; onRefused: Refused }) => (
  <>
    {problems.map((problem) => (
      <div key={problem.code} className="flex items-center gap-2">
        <Text className="flex-1" typography="bodySmall" color={problem.blocking ? md('error') : md('onSurfaceVariant')}>
          {problem.text}
        </Text>
        <PayloadFixButton problem={problem} onRefused={onRefused} />
      </div>
    ))}
  </>
);

export const GripperPanel = ({ offers, payload, onDismiss }: { offers: readonly GuidedOffer[]; payload: Payload; onDismiss: () => void }) => {
  const stateJson = useQgcValue(PAYLOAD_STATE) as Json | null;
  const [now, setNow] = useState(() => Date.now());
  useTicker(STATUS_TICK_MS, [], () => setNow(Date.now()));
  const [refusal, setRefusal] = useState<string | null>(null);
  const status = refusal === null ? payloadStatus(stateJson, now) : { text: refusal, warning: true };
  const lock = payloadLock(payloadRailOf(useQgcPath(PAYLOAD_RAIL_VIEW) as Json | null));
  const safe = lock?.arms === true;
  return (
    <AircastSheet onDismissRequest={onDismiss} label="Payload">
      <div className="flex w-full flex-col gap-3 px-sheet pb-6">
        <Text typography="titleLarge">Payload</Text>
        {lock !== null ? <LockRow lock={lock} onRefused={setRefusal} /> : null}
        <Problems problems={payload.problems} onRefused={setRefusal} />
        <Text typography="bodyMedium" color={status?.warning === true ? md('error') : md('onSurfaceVariant')}>
          {status?.text ?? 'No payload command sent yet'}
        </Text>
        {multiBay(payload) ? <Bays offers={offers} payload={payload} safe={safe} onRefused={setRefusal} /> : <SingleGripper offers={offers} safe={safe} onRefused={setRefusal} />}
      </div>
    </AircastSheet>
  );
};
