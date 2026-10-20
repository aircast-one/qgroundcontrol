import type { Json } from './Json';
import { Qgc } from './Qgc';

const VEHICLE_ARMED = 'vehicle.armed';
const VEHICLE_FLIGHT_MODE = 'vehicle.flightMode';
const VEHICLE_CHECKLIST_STATE = 'vehicle.checkListState';
const VEHICLE_VTOL_FORWARD_FLIGHT = 'vehicle.vtolInFwdFlight';
const VEHICLE_HEADING = 'vehicle.heading';
const VEHICLE_COORDINATE = 'vehicle.coordinate';
const VEHICLE_TAKEOFF = 'vehicle.guidedModeTakeoff';
const VEHICLE_LAND = 'vehicle.guidedModeLand';
const VEHICLE_RETURN_TO_LAUNCH = 'vehicle.guidedModeRTL';
const VEHICLE_CHANGE_ALTITUDE = 'vehicle.guidedModeChangeAltitude';
const VEHICLE_START_MISSION = 'vehicle.startMission';
const VEHICLE_ABORT_LANDING = 'vehicle.abortLanding';
const VEHICLE_GRIPPER = 'vehicle.sendGripperAction';
const VEHICLE_FIX_PAYLOAD = 'vehicle.fixPayload';
const VEHICLE_STOP_ROI = 'vehicle.stopGuidedModeROI';
const VEHICLE_FORCE_ARM = 'vehicle.forceArm';
const VEHICLE_EMERGENCY_STOP = 'vehicle.emergencyStop';
const VEHICLE_MOTOR_TEST = 'vehicle.motorTest';
const VEHICLE_RC_OVERRIDE = 'vehicle.setRcChannelOverride';
const VEHICLE_RC_RELEASE = 'vehicle.releaseRcChannelOverride';
const VEHICLE_RC_CLEAR = 'vehicle.clearRcChannelOverrides';
const VEHICLE_REMOTE_ID_EMERGENCY = 'vehicle.remoteIDManager.setEmergency';
const VEHICLE_RESET_MESSAGES = 'vehicle.resetAllMessages';
const VEHICLE_CLEAR_MESSAGES = 'vehicle.clearMessages';
const VEHICLE_AUTO_INTERCEPT_DROP = 'vehicle.autoInterceptDrop';

export const VehicleCommands = {
  setArmed: (armed: boolean): Promise<boolean> => Qgc.set(VEHICLE_ARMED, armed),
  setFlightMode: (mode: string): Promise<boolean> => Qgc.set(VEHICLE_FLIGHT_MODE, mode),
  setChecklistState: (state: number): Promise<boolean> => Qgc.set(VEHICLE_CHECKLIST_STATE, state),
  setForwardFlight: (forward: boolean): Promise<boolean> => Qgc.set(VEHICLE_VTOL_FORWARD_FLIGHT, forward),

  heading: (): Promise<Json> => Qgc.get(VEHICLE_HEADING),
  coordinate: (): Promise<Json> => Qgc.get(VEHICLE_COORDINATE),

  takeoff: (meters?: number): Promise<boolean> => (meters === undefined ? Qgc.invoke(VEHICLE_TAKEOFF) : Qgc.invoke(VEHICLE_TAKEOFF, meters)),
  takeoffRefusal: (meters?: number): Promise<string | null> => (meters === undefined ? Qgc.refusalOf(VEHICLE_TAKEOFF) : Qgc.refusalOf(VEHICLE_TAKEOFF, meters)),
  land: (): Promise<boolean> => Qgc.invoke(VEHICLE_LAND),
  landRefusal: (): Promise<string | null> => Qgc.refusalOf(VEHICLE_LAND),
  returnToLaunch: (smart: boolean): Promise<boolean> => Qgc.invoke(VEHICLE_RETURN_TO_LAUNCH, smart),
  changeSpeed: (command: string, metersSecond: number): Promise<boolean> => Qgc.invoke(`vehicle.${command}`, metersSecond),
  changeAltitude: (deltaMeters: number, pause: boolean): Promise<boolean> => Qgc.invoke(VEHICLE_CHANGE_ALTITUDE, deltaMeters, pause),
  startMission: (): Promise<boolean> => Qgc.invoke(VEHICLE_START_MISSION),
  abortLanding: (climbMeters: number): Promise<boolean> => Qgc.invoke(VEHICLE_ABORT_LANDING, climbMeters),
  gripper: (action: number, bay = 0): Promise<boolean> => Qgc.invoke(VEHICLE_GRIPPER, action, bay),
  gripperRefusal: (action: number, bay = 0): Promise<string | null> => Qgc.refusalOf(VEHICLE_GRIPPER, action, bay),
  fixPayloadRefusal: (code: string): Promise<string | null> => Qgc.refusalOf(VEHICLE_FIX_PAYLOAD, code),
  stopRoi: (): Promise<boolean> => Qgc.invoke(VEHICLE_STOP_ROI),
  forceArm: (): Promise<boolean> => Qgc.invoke(VEHICLE_FORCE_ARM),
  emergencyStop: (): Promise<boolean> => Qgc.invoke(VEHICLE_EMERGENCY_STOP),
  motorTest: (motor: number, percent: number, seconds: number, inOrder: boolean): Promise<boolean> => Qgc.invoke(VEHICLE_MOTOR_TEST, motor, percent, seconds, inOrder),
  overrideRcChannel: (channel: number, pwm: number): Promise<boolean> => Qgc.invoke(VEHICLE_RC_OVERRIDE, channel, pwm),
  releaseRcChannel: (channel: number): Promise<boolean> => Qgc.invoke(VEHICLE_RC_RELEASE, channel),
  clearRcOverrides: (): Promise<boolean> => Qgc.invoke(VEHICLE_RC_CLEAR),
  declareRemoteIdEmergency: (declare: boolean): Promise<boolean> => Qgc.invoke(VEHICLE_REMOTE_ID_EMERGENCY, declare),
  resetAllMessages: (): Promise<boolean> => Qgc.invoke(VEHICLE_RESET_MESSAGES),
  clearMessages: (): Promise<boolean> => Qgc.invoke(VEHICLE_CLEAR_MESSAGES),
  startAutoInterceptDrop: (): Promise<boolean> => Qgc.invoke(VEHICLE_AUTO_INTERCEPT_DROP),
  stopAutoInterceptDrop: (): Promise<boolean> => Qgc.invoke(VEHICLE_AUTO_INTERCEPT_DROP, 'stop'),
};
