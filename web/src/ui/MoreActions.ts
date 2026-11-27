import { PlanCommands } from '@/bridge/BridgeCommands';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { EMERGENCY_STOP } from './EmergencyStop';
import { type Json, jsonObjects, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import type { GuidedOffer, PursuitConfirm } from './GuidedActions';

export const PAUSE = 'pause';

export const BAR_ACTIONS: ReadonlySet<string> = new Set(['arm', 'disarm', 'takeoff', 'land', 'rtl', 'changeSpeed', 'changeAltitude']);

export const SHEET_ACTIONS: ReadonlySet<string> = new Set([
  'startMission', 'continueMission', 'resumeMission', 'cancelRoi', PAUSE,
  'landAbort', 'release', 'grab', 'hold', 'vtolTransitionToFixedWing',
  'vtolTransitionToMultiRotor', 'forceArm',
]);

export const GRIPPER_RELEASE = 0;
export const GRIPPER_GRAB = 1;
export const GRIPPER_HOLD = 2;
export const RELEASE = 'release';
const LAND_ABORT_CLIMB_METERS = 50.0;

export const moreActions = (offers: ReadonlyMap<string, GuidedOffer>): readonly GuidedOffer[] =>
  [...offers.values()].filter((it) => it.shown && !BAR_ACTIONS.has(it.id) && it.id !== EMERGENCY_STOP && SHEET_ACTIONS.has(it.id));

const LAND_ABORT = 'landAbort';

const START_MISSION = 'startMission';

export const AUTO_POPUP_ACTIONS: readonly string[] = [LAND_ABORT, START_MISSION, 'continueMission'];

export const popupReplacesOpenConfirm = (id: string): boolean => id === LAND_ABORT;

export const missionFlown = (flown: boolean, armed: boolean, missionAvailable: boolean): boolean => armed || (flown && missionAvailable);

export const autoMissionPopup = (wasReady: ReadonlySet<string>, offers: ReadonlyMap<string, GuidedOffer>, enabled: boolean, flown = false): GuidedOffer | null =>
  AUTO_POPUP_ACTIONS.flatMap((id) => offers.get(id) ?? []).find((it) => (enabled || it.id === LAND_ABORT) && it.ready && !wasReady.has(it.id) && !(flown && it.id === START_MISSION)) ?? null;

export const guidedCommand = (id: string, resumeFrom: number | null): (() => Promise<string | null>) | null => {
  switch (id) {
    case 'startMission':
    case 'continueMission':
      return () => VehicleCommands.startMissionRefusal();
    case 'landAbort':
      return () => VehicleCommands.abortLandingRefusal(LAND_ABORT_CLIMB_METERS);
    case 'grab':
      return () => VehicleCommands.gripperRefusal(GRIPPER_GRAB);
    case 'release':
      return () => VehicleCommands.gripperRefusal(GRIPPER_RELEASE);
    case 'hold':
      return () => VehicleCommands.gripperRefusal(GRIPPER_HOLD);
    case 'cancelRoi':
      return () => VehicleCommands.stopRoiRefusal();
    case 'vtolTransitionToFixedWing':
      return () => VehicleCommands.forwardFlightRefusal(true);
    case 'vtolTransitionToMultiRotor':
      return () => VehicleCommands.forwardFlightRefusal(false);
    case 'forceArm':
      return () => VehicleCommands.forceArmRefusal();
    case 'resumeMission':
      return resumeFrom === null ? null : () => PlanCommands.resumeMissionRefusal(resumeFrom);
    default:
      return null;
  }
};

export const CALIBRATE = 'Calibrate';
export const AUX_SWITCHES_VIEW = 'view.auxSwitches';
export const AUX_SWITCH = 'vehicle.auxSwitch';

export type AuxSwitch = { readonly id: string; readonly on: boolean; readonly action: string; readonly confirm: PursuitConfirm | null };

export const auxSwitchReadings = (view: Json | null): readonly AuxSwitch[] =>
  jsonObjects(optJSONArray(view, 'switches')).map((it) => {
    const confirm = optJSONObject(it, 'confirm');
    return {
      id: optText(it, 'id'),
      on: optBoolean(it, 'on'),
      action: optText(it, 'action'),
      confirm: confirm === null ? null : { title: optText(confirm, 'title'), text: optText(confirm, 'text') },
    };
  });

export const auxSwitchIcon = (id: string): 'ic_shield' | 'ic_straighten' => (id === 'avoidance' ? 'ic_shield' : 'ic_straighten');
