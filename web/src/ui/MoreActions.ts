import { PlanCommands } from '@/bridge/BridgeCommands';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { EMERGENCY_STOP } from './EmergencyStop';
import type { GuidedOffer } from './GuidedActions';

export const PAUSE = 'pause';

export const BAR_ACTIONS: ReadonlySet<string> = new Set(['arm', 'disarm', 'takeoff', 'land', 'rtl', 'changeSpeed', 'changeAltitude']);

export const SHEET_ACTIONS: ReadonlySet<string> = new Set([
  'startMission', 'continueMission', 'resumeMission', 'cancelRoi', PAUSE,
  'landAbort', 'release', 'grab', 'hold', 'vtolTransitionToFixedWing',
  'vtolTransitionToMultiRotor', 'forceArm',
]);

export const GRIPPER_RELEASE = 0;
export const GRIPPER_GRAB = 1;
const GRIPPER_HOLD = 2;
export const RELEASE = 'release';
const LAND_ABORT_CLIMB_METERS = 50.0;

export const moreActions = (offers: ReadonlyMap<string, GuidedOffer>): readonly GuidedOffer[] =>
  [...offers.values()].filter((it) => it.shown && !BAR_ACTIONS.has(it.id) && it.id !== EMERGENCY_STOP && SHEET_ACTIONS.has(it.id));

const LAND_ABORT = 'landAbort';

export const AUTO_POPUP_ACTIONS: readonly string[] = [LAND_ABORT, 'startMission', 'continueMission'];

export const popupReplacesOpenConfirm = (id: string): boolean => id === LAND_ABORT;

export const autoMissionPopup = (wasReady: ReadonlySet<string>, offers: ReadonlyMap<string, GuidedOffer>, enabled: boolean): GuidedOffer | null =>
  AUTO_POPUP_ACTIONS.flatMap((id) => offers.get(id) ?? []).find((it) => (enabled || it.id === LAND_ABORT) && it.ready && !wasReady.has(it.id)) ?? null;

export const guidedCommand = (id: string, resumeFrom: number | null): (() => void) | null => {
  switch (id) {
    case 'startMission':
    case 'continueMission':
      return () => void VehicleCommands.startMission();
    case 'landAbort':
      return () => void VehicleCommands.abortLanding(LAND_ABORT_CLIMB_METERS);
    case 'grab':
      return () => void VehicleCommands.gripper(GRIPPER_GRAB);
    case 'release':
      return () => void VehicleCommands.gripper(GRIPPER_RELEASE);
    case 'hold':
      return () => void VehicleCommands.gripper(GRIPPER_HOLD);
    case 'cancelRoi':
      return () => void VehicleCommands.stopRoi();
    case 'vtolTransitionToFixedWing':
      return () => void VehicleCommands.setForwardFlight(true);
    case 'vtolTransitionToMultiRotor':
      return () => void VehicleCommands.setForwardFlight(false);
    case 'forceArm':
      return () => void VehicleCommands.forceArm();
    case 'resumeMission':
      return resumeFrom === null ? null : () => void PlanCommands.resumeMission(resumeFrom);
    default:
      return null;
  }
};
