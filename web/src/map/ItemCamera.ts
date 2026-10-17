import { type Json, isNull, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { toDoubleOrNull } from './AltitudeEntry';
import { setOk } from './BridgeCalls';
import { coordinate } from './FenceBridge';
import { MapBridge } from './MapBridge';
import type { ClosedRange } from './SettingStepper';
import type { TrackPoint } from './VehicleMap';

const cameraSection = (index: number): string => `plan.missionController.visualItems.${index}.cameraSection`;

const written = (path: string, value: unknown): Promise<boolean> => setOk(path, JSON.stringify({ value })).catch(() => false);

export const ItemCameraBridge = {
  read: (index: number): Promise<Json | null> => MapBridge.read(`view.itemCamera(${index})`),

  set: (index: number, member: string, value: unknown): Promise<boolean> => written(`${cameraSection(index)}.${member}`, value),

  chooseAction: (index: number, choice: number): Promise<boolean> => written(`${cameraSection(index)}.cameraAction.enumIndex`, choice),
};

export const VIDEO_VIEW_PATH = 'view.video';

export const VideoBridge = {
  read: (): Promise<Json | null> => MapBridge.read(VIDEO_VIEW_PATH),
};

export const shotPoints = (view: Json | null): readonly TrackPoint[] =>
  jsonObjects(optJSONArray(view, 'shotPoints')).flatMap((it) => coordinate(it) ?? []);

export type CameraChoices = { labels: readonly string[]; chosen: number };

export type CameraExtras = {
  intervalTime: number | null;
  intervalDistance: number | null;
  distanceUnits: string;
  modeSupported: boolean;
  commandsMode: boolean;
  mode: number;
  commandsGimbal: boolean;
  pitch: number;
  yaw: number;
  pitchRange: ClosedRange | null;
  yawRange: ClosedRange | null;
};

const measured = (view: Json, key: string): number | null => {
  const value = optDouble(optJSONObject(view, key), 'value');
  return Number.isNaN(value) ? null : value;
};

const userRange = (view: Json, key: string): ClosedRange | null => {
  const slider = optJSONObject(optJSONObject(view, key), 'slider');
  if (slider === null) return null;
  const from = optDouble(slider, 'from');
  const to = optDouble(slider, 'to');
  return Number.isNaN(from) || Number.isNaN(to) || from === to ? null : { start: Math.min(from, to), endInclusive: Math.max(from, to) };
};

export const cameraExtras = (view: Json | null): CameraExtras | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const units = optText(optJSONObject(view, 'intervalDistance'), 'units');
  const mode = optJSONObject(view, 'cameraMode');
  return {
    intervalTime: measured(view, 'intervalTime'),
    intervalDistance: measured(view, 'intervalDistance'),
    distanceUnits: units.trim() === '' ? 'm' : units,
    modeSupported: optBoolean(view, 'cameraModeSupported'),
    commandsMode: optBoolean(view, 'commandsMode'),
    mode: mode === null ? 0 : optInt(mode, 'choice', 0),
    commandsGimbal: optBoolean(view, 'commandsGimbal'),
    pitch: measured(view, 'gimbalPitch') ?? 0.0,
    yaw: measured(view, 'gimbalYaw') ?? 0.0,
    pitchRange: userRange(view, 'gimbalPitch'),
    yawRange: userRange(view, 'gimbalYaw'),
  };
};

export const cameraChoices = (view: Json | null): CameraChoices | null => {
  const measure = view !== null && optBoolean(view, 'available') ? optJSONObject(view, 'cameraAction') : null;
  const listed = optJSONArray(measure, 'choices');
  if (listed === null) return null;
  const labels = jsonStrings(listed);
  if (!labels.some((it) => it.trim() !== '')) return null;
  return { labels, chosen: optInt(measure, 'choice', -1) };
};

export const measureText = (view: Json | null, key: string): string => {
  const measure = optJSONObject(view, key);
  if (measure === null) return '';
  const text = optText(measure, 'text');
  if (text.trim() === '') return '';
  const units = optText(measure, 'units');
  return units.trim() === '' ? text : `${text} ${units}`;
};

export const namedAction = (text: string): string => (text.trim() === '' || toDoubleOrNull(text.trim()) !== null ? '' : text);

export const actionLabel = (view: Json | null): string => {
  const choices = cameraChoices(view);
  const chosen = choices === null ? undefined : choices.labels[choices.chosen];
  return chosen ?? namedAction(measureText(view, 'cameraAction'));
};

export const itemCameraNote = (view: Json | null): string | null => {
  if (view === null || !optBoolean(view, 'available') || isNull(view, 'note')) return null;
  const note = optText(view, 'note');
  return note.trim() === '' ? null : note;
};

export const itemCameraTextBeside = (view: Json | null, pickerLabel: string | null): string | null => {
  const text = itemCameraText(view);
  return text === pickerLabel ? null : text;
};

export const itemCameraText = (view: Json | null): string | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const action = actionLabel(view);
  const angles = [measureText(view, 'gimbalPitch'), measureText(view, 'gimbalYaw')].filter((it) => it.trim() !== '').join(' / ');
  const gimbal = !optBoolean(view, 'commandsGimbal') || angles.trim() === '' ? '' : `gimbal ${angles}`;
  const text = [action, gimbal].filter((it) => it.trim() !== '').join(' · ');
  return text.trim() === '' ? null : text;
};
