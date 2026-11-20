import { VehicleCommands } from '@/bridge/VehicleCommands';
import { mutableStateOf } from '@/kotlin/State';
import type { GuidedOffer } from './GuidedActions';
import { GuidedAction } from './VehicleUi';

export const EMERGENCY_STOP = 'emergencyStop';

export const KILL_SETTLE_MS = 5000;

export const motorsKilledAt = mutableStateOf<number | null>(null);

export const killedRecently = (killedAt: number | null, now: number): boolean => killedAt !== null && now >= killedAt && now - killedAt <= KILL_SETTLE_MS;

export const emergencyStopOffer = (offers: ReadonlyMap<string, GuidedOffer>): GuidedOffer | null => {
  const offer = offers.get(EMERGENCY_STOP);
  return offer?.shown ? offer : null;
};

export const armedStopOffer = (offers: ReadonlyMap<string, GuidedOffer>, armed: boolean): GuidedOffer | null => {
  const offer = emergencyStopOffer(offers);
  return offer !== null && armed && offer.ready ? offer : null;
};

export const emergencyStopAction = (offer: GuidedOffer): GuidedAction =>
  GuidedAction({
    name: offer.title,
    confirm: offer.prompt,
    destructive: true,
    run: () => {
      motorsKilledAt.value = performance.now();
      void VehicleCommands.emergencyStop();
    },
  });
