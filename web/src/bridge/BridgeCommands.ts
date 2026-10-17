import type { WhepReport } from '@/core/whep';
import type { Json } from './Json';
import { Qgc } from './Qgc';

const LINKS_COMMIT = 'links.commitLinkConfigurations';
const LINKS_BLUETOOTH_SCAN = 'links.bluetoothScan';
const LINKS_CONNECT = 'links.createConnectedLink';
const LINKS_REMOVE = 'links.removeConfiguration';
const LINKS_CONFIGURATIONS = 'links.linkConfigurations';
const LINK_SUBJECT = 'The link';
const LINKS_CREATE_CLOUD = 'links.createAircastCloudLink';
const LINKS_CREATE_REPLAY = 'links.createLogReplayConfiguration';
const LINKS_CREATE_BLUETOOTH = 'links.createBluetoothLink';
const LINKS_CREATE_SERIAL = 'links.createSerialConfiguration';
const LINKS_CREATE_SUPPORT = 'links.createMavlinkForwardingSupportLink';
const LINKS_START_MOCK = 'links.startMockLink';
const ACCOUNT_STATE = 'account';
const ACCOUNT_API_BASE = 'account.apiBase';
const ACCOUNT_SIGN_IN = 'account.signIn';
const ACCOUNT_SIGN_OUT = 'account.signOut';
const ACCOUNT_CANCEL_SIGN_IN = 'account.cancelSignIn';
const PLAN_REDO = 'plan.redo';
const PLAN_UNDO_TRACKING = 'plan.undoTracking';
const PLAN_REMOVE_ALL_FROM_VEHICLE = 'plan.removeAllFromVehicle';
const PLAN_RESUME_MISSION = 'planFly.missionController.resumeMission';
const PLAN_COMMAND_CATEGORIES = 'missionCommandTree.categoriesForVehicle';
const PLAN_COMMANDS_FOR_CATEGORY = 'missionCommandTree.getCommandsForCategory';
const MISSION_COMPLETE_DISMISS = 'missionComplete.dismiss';
const VIDEO_NATIVE_RENDERING = 'video.setNativeRendering';
const VIDEO_INIT_NATIVE = 'video.initNative';
const VIDEO_ACTIVE_SOURCE = 'video.setActiveVideoSource';
const VIDEO_START_RECORDING = 'video.startRecording';
const VIDEO_STOP_RECORDING = 'video.stopRecording';
const VIDEO_RESTART = 'video.restart';
const VIDEO_DEVICE_CAMERA_ROTATION = 'video.setDeviceCameraRotation';
const VIDEO_PICTURE_IN_PICTURE = 'settings.videoSettings.multiViewEnabled';
const VIDEO_PIP_SHOWN = 'video.setPipShown';
const VIDEO_REPORT_NATIVE = 'video.reportNative';
const VIDEO_STREAM_ENABLED = 'settings.videoSettings.streamEnabled';
const CAMERAS_ADD = 'cameras.add';
const CAMERAS_UPDATE = 'cameras.update';
const CAMERAS_REMOVE = 'cameras.remove';
const CAMERAS_MOVE = 'cameras.move';
const VEHICLE_FIELDS = 'vehicle';
const ESC_CALIBRATION_START = 'escCalibration.start';
const ESC_CALIBRATION_CLOSE = 'escCalibration.close';
const SUB_MOTORS_OPEN = 'apmSubMotors.open';
const SUB_MOTORS_TEST = 'apmSubMotors.test';
const PACKET_RADIO_REFRESH = 'packetRadio.refreshAdapters';
const INSPECTOR_MESSAGE_INTERVAL = 'mavlinkInspector.setMessageInterval';
const INSPECTOR_ACTIVE_SYSTEM = 'mavlinkInspector.setActiveSystem';
const GIMBAL_CONTROL = 'gimbal.control';
const FIRST_RUN_SHOWN = 'firstRun.markShown';
const SUBTITLES_INSTRUMENTS = 'subtitles.setInstruments';
const HOST_ACKNOWLEDGE = 'host.acknowledgeThrough';

export const LinkCommands = {
  commitConfigurations: (): Promise<boolean> => Qgc.invoke(LINKS_COMMIT),
  scanBluetooth: (scanning: boolean): Promise<boolean> => Qgc.invoke(LINKS_BLUETOOTH_SCAN, scanning),
  connect: (configuration: string): Promise<string | null> => Qgc.refusalOfSubject(LINK_SUBJECT, LINKS_CONNECT, configuration),
  remove: (configuration: string, name: string): Promise<string | null> => Qgc.refusalOfSubject(LINK_SUBJECT, LINKS_REMOVE, configuration, name),
  disconnect: (index: number): Promise<string | null> => Qgc.refusalOfSubject(LINK_SUBJECT, `${LINKS_CONFIGURATIONS}.${index}.link.disconnect`),
  createAircastCloud: async (name: string, apiBase: string, deviceId: string): Promise<boolean> => (await Qgc.invokeResult(LINKS_CREATE_CLOUD, name, apiBase, deviceId)) === true,
  createLogReplay: async (name: string, log: string): Promise<boolean> => (await Qgc.invokeResult(LINKS_CREATE_REPLAY, name, log)) === true,
  createBluetooth: async (name: string, deviceName: string, address: string): Promise<boolean> => (await Qgc.invokeResult(LINKS_CREATE_BLUETOOTH, name, deviceName, address)) === true,
  createSerial: async (name: string, port: string, baud: number): Promise<boolean> => (await Qgc.invokeResult(LINKS_CREATE_SERIAL, name, port, baud)) === true,
  createSupportForwarding: (): Promise<boolean> => Qgc.invoke(LINKS_CREATE_SUPPORT),
  startMock: async (args: unknown[]): Promise<boolean> => (await Qgc.invokeResult(LINKS_START_MOCK, ...args)) === true,
};

export const AccountCommands = {
  state: (): Promise<Json> => Qgc.get(ACCOUNT_STATE),
  setApiBase: (apiBase: string): Promise<boolean> => Qgc.set(ACCOUNT_API_BASE, apiBase),
  signIn: (): Promise<boolean> => Qgc.invoke(ACCOUNT_SIGN_IN),
  signOut: (): Promise<boolean> => Qgc.invoke(ACCOUNT_SIGN_OUT),
  cancelSignIn: (): Promise<boolean> => Qgc.invoke(ACCOUNT_CANCEL_SIGN_IN),
};

export const PlanCommands = {
  redo: (): Promise<boolean> => Qgc.invoke(PLAN_REDO),
  setUndoTracking: (tracking: boolean): Promise<boolean> => Qgc.set(PLAN_UNDO_TRACKING, tracking),
  removeAllFromVehicle: (): Promise<boolean> => Qgc.invoke(PLAN_REMOVE_ALL_FROM_VEHICLE),
  resumeMission: (waypoint: number): Promise<boolean> => Qgc.invoke(PLAN_RESUME_MISSION, waypoint),
  dismissMissionComplete: (notice: number): Promise<boolean> => Qgc.invoke(MISSION_COMPLETE_DISMISS, notice),
  commandCategories: (): Promise<unknown> => Qgc.invokeResult(PLAN_COMMAND_CATEGORIES),
  commandsForCategory: (category: string): Promise<unknown> => Qgc.invokeResult(PLAN_COMMANDS_FOR_CATEGORY, null, category, true),
  setLandingHeading: (index: number, heading: number): Promise<string | null> => Qgc.writeRefusal(`plan.missionController.visualItems.${index}.landingHeading`, heading),
  setLandingCoordinate: (index: number, coordinate: Json): Promise<string | null> => Qgc.writeRefusal(`plan.missionController.visualItems.${index}.landingCoordinate`, coordinate),
  setAltitudesRelative: (index: number, relative: boolean): Promise<string | null> => Qgc.writeRefusal(`plan.missionController.visualItems.${index}.altitudesAreRelative`, relative),
};

export const VideoCommands = {
  setNativeRendering: (native: boolean): Promise<boolean> => Qgc.invoke(VIDEO_NATIVE_RENDERING, native),
  initNative: (): Promise<boolean> => Qgc.invoke(VIDEO_INIT_NATIVE),
  setActiveSource: (slot: number): Promise<boolean> => Qgc.invoke(VIDEO_ACTIVE_SOURCE, slot),
  setRecording: (recording: boolean): Promise<boolean> => Qgc.invoke(recording ? VIDEO_START_RECORDING : VIDEO_STOP_RECORDING),
  restart: (): Promise<boolean> => Qgc.invoke(VIDEO_RESTART),
  setDeviceCameraRotation: (degrees: number): Promise<boolean> => Qgc.invoke(VIDEO_DEVICE_CAMERA_ROTATION, degrees),
  setPictureInPicture: (shown: boolean): Promise<boolean> => Qgc.set(VIDEO_PICTURE_IN_PICTURE, shown),
  setPipShown: (shown: boolean): Promise<boolean> => Qgc.invoke(VIDEO_PIP_SHOWN, shown),
  reportPlayback: (channel: number, { running, frames, width, height, error, restarted }: WhepReport): Promise<boolean> =>
    Qgc.invoke(VIDEO_REPORT_NATIVE, running, frames, width, height, error, null, restarted, channel),
  turnStreamOn: (): Promise<boolean> => Qgc.set(VIDEO_STREAM_ENABLED, true),
};

export const CameraCommands = {
  add: (name: string, source: string, url: string, tilt: number | null): Promise<string | null> => Qgc.refusalOf(CAMERAS_ADD, name, source, url, tilt),
  update: (slot: number, name: string, source: string, url: string, tilt: number | null): Promise<string | null> => Qgc.refusalOf(CAMERAS_UPDATE, slot, name, source, url, tilt),
  remove: (slot: number): Promise<string | null> => Qgc.refusalOf(CAMERAS_REMOVE, slot),
  move: (from: number, to: number): Promise<string | null> => Qgc.refusalOf(CAMERAS_MOVE, from, to),
};

export const SetupCommands = {
  vehicleFields: (fields: string[]): Promise<Json> => Qgc.getFields(VEHICLE_FIELDS, fields),
  startEscCalibration: (): Promise<boolean> => Qgc.invoke(ESC_CALIBRATION_START),
  closeEscCalibration: (): Promise<boolean> => Qgc.invoke(ESC_CALIBRATION_CLOSE),
  openSubMotors: (): Promise<boolean> => Qgc.invoke(SUB_MOTORS_OPEN),
  testSubMotor: (motor: number, value: number): Promise<boolean> => Qgc.invoke(SUB_MOTORS_TEST, motor, value),
  refreshPacketRadios: (): Promise<boolean> => Qgc.invoke(PACKET_RADIO_REFRESH),
  setInspectorMessageInterval: (rate: number): Promise<boolean> => Qgc.invoke(INSPECTOR_MESSAGE_INTERVAL, rate),
  setInspectorSystem: (system: number): Promise<boolean> => Qgc.invoke(INSPECTOR_ACTIVE_SYSTEM, system),
  takeGimbalControlRefusal: (): Promise<string | null> => Qgc.refusalOf(GIMBAL_CONTROL, true),
};

export const AppCommands = {
  markFirstRunShown: (): Promise<boolean> => Qgc.invoke(FIRST_RUN_SHOWN),
  setSubtitleInstruments: (vehicleClass: string, instruments: string[]): Promise<boolean> => Qgc.invoke(SUBTITLES_INSTRUMENTS, vehicleClass, instruments.join(',')),
  acknowledgeNoticesThrough: (through: number): Promise<boolean> => Qgc.invoke(HOST_ACKNOWLEDGE, through),
};
