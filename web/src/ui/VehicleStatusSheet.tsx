import type { GuidedOffer } from './GuidedActions';

export const ARM_REQUEST = 'arm';
export const ARM_UNAVAILABLE = 'Arming is not available right now.';

export const deckRequestRefusal = (offer: GuidedOffer | null): string => (offer === null || offer.reason.trim() === '' ? ARM_UNAVAILABLE : offer.reason);

export const VehicleStatusSheet = (_props: { onMessages: () => void; onDismiss: () => void }) => null;
