import { type Json, arrayJSONObject, isNull, jsonStrings, optArrayText, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { coerceIn, format } from '@/kotlin/Std';
import type { PaintedRect } from '@/ui/VideoView';

export const CAM_MODE_PHOTO = 0;
export const CAM_MODE_VIDEO = 1;

export type CameraShutter = {
  label: string;
  recording: boolean;
  enabled: boolean;
  action: string | null;
  video: boolean;
  readout: string;
  readoutActive: boolean;
};

export type CameraPanel = {
  visible: boolean;
  inPhotoMode: boolean;
  selectVideoEnabled: boolean;
  selectPhotoEnabled: boolean;
  bothShown: boolean;
  shutters: readonly CameraShutter[];
  freeText: string | null;
  batteryText: string | null;
};

const blankToNull = (text: string): string | null => (text.trim() === '' ? null : text);

export const cameraPanel = (panel: Json | null): CameraPanel | null => {
  if (panel === null) return null;
  const v = optJSONObject(panel, 'video');
  const video: CameraShutter | null =
    v === null
      ? null
      : {
          label: optBoolean(v, 'capturing') ? 'Stop recording' : 'Start recording',
          recording: optBoolean(v, 'capturing'),
          enabled: optBoolean(v, 'enabled'),
          action: CAMERA_RECORD,
          video: true,
          readout: optText(v, 'clock'),
          readoutActive: !optBoolean(v, 'idle'),
        };
  const p = optJSONObject(panel, 'photo');
  const press = optText(p, 'press');
  const photo: CameraShutter | null =
    p === null
      ? null
      : {
          label: press === 'stop' ? 'Stop photos' : 'Take photo',
          recording: optBoolean(p, 'capturing'),
          enabled: optBoolean(p, 'enabled'),
          action: press === 'stop' ? CAMERA_STOP_PHOTO : press === 'take' ? CAMERA_PHOTO : null,
          video: false,
          readout: optText(p, 'count'),
          readoutActive: !optBoolean(p, 'idle'),
        };
  return {
    visible: optBoolean(panel, 'visible'),
    inPhotoMode: optBoolean(panel, 'inPhotoMode'),
    selectVideoEnabled: optBoolean(panel, 'selectVideoEnabled'),
    selectPhotoEnabled: optBoolean(panel, 'selectPhotoEnabled'),
    bothShown: optBoolean(panel, 'bothShown'),
    shutters: [video, photo].filter((it): it is CameraShutter => it !== null),
    freeText: blankToNull(optText(panel, 'freeText')),
    batteryText: blankToNull(optText(panel, 'batteryText')),
  };
};

export const modeTapSwitches = (camera: CameraReading, video: boolean): boolean => camera.mode !== (video ? CAM_MODE_VIDEO : CAM_MODE_PHOTO);

export const shutterCaption = (panel: CameraPanel, shutter: CameraShutter): string | null => (panel.bothShown ? (shutter.video ? 'Video' : 'Photo') : null);

export const CAMERA_VIEW = 'view.camera';

export type CameraReading = {
  hasModes: boolean;
  canChangeMode: boolean;
  modeText: string;
  isRecording: boolean;
  canPhoto: boolean;
  canRecord: boolean;
  isTakingPhoto: boolean;
  capturesPhotos: boolean;
  hasVideoStream: boolean;
  mode: number;
  timelapse: boolean;
  lapseSeconds: number | null;
  lapseCount: number | null;
  lapseUnlimited: boolean;
  title: string;
  labels: readonly string[];
  stateText: string;
  reportsStorage: boolean;
  storageText: string;
  shotsText: string;
  batteryText: string;
  hasZoom: boolean;
  zoomLevel: number;
  selected: number | null;
  streamLabels: readonly string[];
  currentStream: number;
  panel: CameraPanel | null;
};

export const cameraReading = (view: Json | null): CameraReading | null => {
  if (view === null || !optBoolean(view, 'present')) return null;
  const lapseSeconds = optDouble(view, 'lapseSeconds');
  const zoomLevel = optDouble(view, 'zoomLevel', ZOOM_LOWEST);
  return {
    hasModes: optBoolean(view, 'hasModes'),
    canChangeMode: optBoolean(view, 'canChangeMode'),
    modeText: optText(view, 'modeText'),
    isRecording: optBoolean(view, 'isRecording'),
    canPhoto: optBoolean(view, 'canPhoto'),
    canRecord: optBoolean(view, 'canRecord'),
    isTakingPhoto: optBoolean(view, 'isTakingPhoto'),
    capturesPhotos: optBoolean(view, 'capturesPhotos'),
    hasVideoStream: optBoolean(view, 'hasVideoStream'),
    mode: optInt(view, 'mode', -1),
    timelapse: optText(view, 'photoMode') === 'timelapse',
    lapseSeconds: Number.isFinite(lapseSeconds) ? lapseSeconds : null,
    lapseCount: isNull(view, 'lapseCount') ? null : optInt(view, 'lapseCount'),
    lapseUnlimited: optBoolean(view, 'lapseUnlimited'),
    title: optText(view, 'title'),
    labels: jsonStrings(optJSONArray(view, 'labels')),
    stateText: optText(view, 'stateText'),
    reportsStorage: optBoolean(view, 'reportsStorage'),
    storageText: optText(view, 'storageText'),
    shotsText: optText(view, 'shotsText'),
    batteryText: optText(view, 'batteryText'),
    hasZoom: optBoolean(view, 'hasZoom'),
    zoomLevel: Number.isFinite(zoomLevel) ? zoomLevel : ZOOM_LOWEST,
    selected: isNull(view, 'selected') ? null : optInt(view, 'selected'),
    streamLabels: jsonStrings(optJSONArray(view, 'streamLabels')),
    currentStream: optInt(view, 'currentStream'),
    panel: cameraPanel(optJSONObject(view, 'panel')),
  };
};

export const ZOOM_LOWEST = 0.0;
export const ZOOM_HIGHEST = 100.0;
export const CAMERA_ZOOM = 'vehicle.cameraManager.currentCameraInstance.zoomLevel';
export const CAMERA_START_TRACKING = 'vehicle.cameraManager.currentCameraInstance.startTracking';
export const CAMERA_STOP_TRACKING = 'vehicle.cameraManager.currentCameraInstance.stopTracking';

export type TrackingBox = { x: number; y: number; width: number; height: number };

export type TrackingReading = {
  supported: boolean;
  requested: boolean;
  reported: boolean;
  shapes: readonly string[];
  box: TrackingBox | null;
};

export const trackingReading = (view: Json | null): TrackingReading | null => {
  const tracking = optJSONObject(view, 'tracking');
  if (tracking === null || !optBoolean(tracking, 'supported')) return null;
  const listed = optJSONArray(tracking, 'shapes') ?? [];
  const rect = optJSONObject(tracking, 'rect');
  const box: TrackingBox | null = rect === null ? null : { x: optDouble(rect, 'x'), y: optDouble(rect, 'y'), width: optDouble(rect, 'width'), height: optDouble(rect, 'height') };
  return {
    supported: true,
    requested: optBoolean(tracking, 'requested'),
    reported: optBoolean(tracking, 'reported'),
    shapes: listed.map((_, at) => optArrayText(listed, at)).filter((it) => it.trim() !== ''),
    box: box !== null && box.width > 0.0 && box.height > 0.0 ? box : null,
  };
};

export const CAMERA_TRACKING_ARMED = 'vehicle.cameraManager.currentCameraInstance.trackingEnabled';

export const trackingToggleLabel = (reading: TrackingReading): string => (reading.requested ? 'Stop tracking' : 'Track something');

export const trackingCanAim = (reading: TrackingReading | null): boolean => reading !== null && reading.requested && reading.shapes.length > 0;

export const trackingCanStart = (reading: TrackingReading | null): boolean => reading !== null && !reading.requested && reading.shapes.length > 0;

export const trackingCanStop = (reading: TrackingReading | null): boolean => reading !== null && reading.requested;

export const trackingRectObject = (box: TrackingBox): Json => ({ x: box.x, y: box.y, width: box.width, height: box.height });

export const TRACK_POINT_SLOP_DP = 10.0;
export const TRACK_POINT_RADIUS_DP = 50.0;

export type TrackingRequest = { kind: 'Box'; rect: TrackingBox } | { kind: 'Point'; x: number; y: number; radius: number };

export const TrackingRequest = {
  Box: (rect: TrackingBox): TrackingRequest => ({ kind: 'Box', rect }),
  Point: (x: number, y: number, radius: number): TrackingRequest => ({ kind: 'Point', x, y, radius }),
};

export const trackingRequest = (pressX: number, pressY: number, releaseX: number, releaseY: number, picture: PaintedRect): TrackingRequest | null => {
  if (picture.width <= 0.0 || picture.height <= 0.0) return null;
  const acrossX = (value: number) => coerceIn((value - picture.left) / picture.width, 0.0, 1.0);
  const acrossY = (value: number) => coerceIn((value - picture.top) / picture.height, 0.0, 1.0);
  const x0 = acrossX(Math.min(pressX, releaseX));
  const x1 = acrossX(Math.max(pressX, releaseX));
  const y0 = acrossY(Math.min(pressY, releaseY));
  const y1 = acrossY(Math.max(pressY, releaseY));
  const tapped = Math.abs(releaseX - pressX) < TRACK_POINT_SLOP_DP && Math.abs(releaseY - pressY) < TRACK_POINT_SLOP_DP;
  return tapped
    ? TrackingRequest.Point(x0, y0, TRACK_POINT_RADIUS_DP / picture.width)
    : x1 - x0 <= 0.0 || y1 - y0 <= 0.0
      ? null
      : TrackingRequest.Box({ x: x0, y: y0, width: x1 - x0, height: y1 - y0 });
};

export const trackingPointObject = (point: Extract<TrackingRequest, { kind: 'Point' }>): Json => ({ x: point.x, y: point.y });

export const TRACKING_CENTRE: TrackingBox = { x: 0.4, y: 0.4, width: 0.2, height: 0.2 };

export const CAMERA_FORMAT = 'vehicle.cameraManager.currentCameraInstance.formatCard';

export type DestructiveAction = {
  id: string;
  label: string;
  button: string;
  title: string;
  prompt: string;
  offer: string;
  reason: string;
  readonly ready: boolean;
  readonly blocked: boolean;
  readonly shown: boolean;
};

export const DestructiveAction = (fields: Omit<DestructiveAction, 'ready' | 'blocked' | 'shown'>): DestructiveAction => ({
  ...fields,
  ready: fields.offer === 'ready',
  blocked: fields.offer === 'blocked',
  shown: fields.offer !== 'hidden',
});

export const destructiveActions = (view: Json | null): readonly DestructiveAction[] => {
  const listed = optJSONArray(view, 'destructiveActions');
  if (listed === null) return [];
  return listed
    .flatMap((_, index) => {
      const entry = arrayJSONObject(listed, index);
      const id = optText(entry, 'id');
      return entry === null || id.trim() === ''
        ? []
        : [
            DestructiveAction({
              id,
              label: optText(entry, 'label'),
              button: optText(entry, 'button'),
              title: optText(entry, 'title'),
              prompt: optText(entry, 'prompt'),
              offer: optText(entry, 'offer'),
              reason: optText(entry, 'reason'),
            }),
          ];
    })
    .filter((it) => it.shown);
};

export const destructiveReasonFor = (action: DestructiveAction): string | null => (action.reason.trim() !== '' && action.blocked ? action.reason : null);

export const destructiveInvokePath = (id: string): string | null => (id === 'resetSettings' ? CAMERA_RESET : id === 'formatStorage' ? CAMERA_FORMAT : null);

export const CAMERA_PHOTO_MODE = 'vehicle.cameraManager.currentCameraInstance.photoCaptureMode';
export const CAMERA_PHOTO_LAPSE = 'vehicle.cameraManager.currentCameraInstance.photoLapse';
export const PHOTO_LAPSE_MIN_S = 1;
export const PHOTO_LAPSE_MAX_S = 60;
export const CAMERA_CURRENT_STREAM = 'vehicle.cameraManager.currentCameraInstance.currentStream';
export const CAMERA_THERMAL_MODE = 'vehicle.cameraManager.currentCameraInstance.thermalMode';
export const CAMERA_SHEET_VIDEO_SETTINGS: ReadonlyMap<string, string> = new Map([
  ['settings.videoSettings.gridLines', 'Video Grid Lines'],
  ['settings.videoSettings.videoFit', 'Video Screen Fit'],
]);
export const CAMERA_THERMAL_OPACITY = 'vehicle.cameraManager.currentCameraInstance.thermalOpacity';

export const THERMAL_MODES: readonly string[] = ['off', 'blend', 'full', 'picInPic'];

const THERMAL_MODE_LABELS: ReadonlyMap<string, string> = new Map([
  ['off', 'Off'],
  ['blend', 'Blend'],
  ['full', 'Full'],
  ['picInPic', 'Picture in picture'],
]);

export const thermalModeLabel = (token: string): string => THERMAL_MODE_LABELS.get(token) ?? token;

export type ThermalReading = { mode: string; opacity: number | null };

export const thermalReading = (view: Json | null): ThermalReading | null => {
  if (view === null || !optBoolean(view, 'thermalAvailable')) return null;
  const mode = optText(view, 'thermalMode');
  if (mode.trim() === '') return null;
  const opacity = isNull(view, 'thermalOpacity') ? null : optDouble(view, 'thermalOpacity');
  return { mode, opacity: opacity !== null && Number.isFinite(opacity) ? opacity : null };
};

export const thermalOpacityIsOffered = (reading: ThermalReading | null): boolean => reading !== null && reading.opacity !== null;

export const CAMERA_RESET = 'vehicle.cameraManager.currentCameraInstance.resetSettings';

export const zoomText = (camera: CameraReading): string | null => (camera.hasZoom ? `${Math.trunc(camera.zoomLevel)}%` : null);

export const shutterReadout = (shutter: CameraShutter): string | null => (!shutter.readoutActive ? null : shutter.video ? `REC ${shutter.readout}` : shutter.readout);

export const CAMERA_PHOTO = 'camera.takePhoto';
export const CAMERA_RECORD = 'camera.toggleRecording';
export const CAMERA_STOP_PHOTO = 'camera.stopPhoto';
export const CAMERA_SET_MODE = 'camera.setMode';

type Detail = readonly [string, string];

const detailIf = (shown: boolean, label: string, reading: string): readonly Detail[] => (shown ? [[label, reading]] : []);

export const cameraDetails = (camera: CameraReading): readonly Detail[] => [
  ['State', camera.stateText],
  ...detailIf(camera.reportsStorage && camera.storageText.trim() !== '', 'Storage', camera.storageText),
  ...detailIf(camera.shotsText.trim() !== '', 'Photos', camera.shotsText),
  ...detailIf(camera.batteryText.trim() !== '', 'Battery', camera.batteryText),
];

export const lapsePlan = (camera: CameraReading): string | null => {
  if (!camera.timelapse) return null;
  const every = camera.lapseSeconds !== null && camera.lapseSeconds > 0 ? `every ${trimmed(camera.lapseSeconds)} s` : null;
  const many = camera.lapseUnlimited ? 'until stopped' : camera.lapseCount !== null && camera.lapseCount > 0 ? `${camera.lapseCount} shots` : null;
  const plan = [every, many].filter((it): it is string => it !== null).join(', ');
  return plan.trim() === '' ? 'interval capture' : plan;
};

const trimmed = (value: number): string => (value === Math.trunc(value) ? String(Math.trunc(value)) : format('%.1f', value));
