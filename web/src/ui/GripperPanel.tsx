import type { GuidedOffer } from './GuidedActions';

export const GRIPPER_ACTIONS: readonly string[] = ['release', 'grab', 'hold'];

export const gripperOffers = (extras: readonly GuidedOffer[]): readonly GuidedOffer[] => GRIPPER_ACTIONS.flatMap((id) => extras.find((it) => it.id === id) ?? []);

export const GripperPanel = (_props: { offers: readonly GuidedOffer[]; onDismiss: () => void }) => null;
