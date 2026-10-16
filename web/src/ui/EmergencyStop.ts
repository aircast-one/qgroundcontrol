import { VehicleCommands } from '@/bridge/VehicleCommands';
import type { GuidedOffer } from './GuidedActions';
import { GuidedAction } from './VehicleUi';

export const EMERGENCY_STOP = 'emergencyStop';

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
    run: () => void VehicleCommands.emergencyStop(),
  });
