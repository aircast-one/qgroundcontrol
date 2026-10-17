import { useState } from 'react';
import { type Json, isNull, optDouble, optJSONObject, optText } from '@/bridge/Json';
import { useProduceState } from '@/kotlin/Compose';
import { AlertDialog, DropdownMenu, DropdownMenuItem, FilterChip, IconButton, RadioButton, Text, TextButton } from '@/m3';
import { Drawable, MaterialIcon } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { md } from '@/theme/Colors';
import { setOk, settingJson } from './BridgeCalls';
import { CAMERA_MODES, trimmedNumber } from './CameraSectionExtras';
import { type CameraChoices, type CameraExtras, ItemCameraBridge, cameraChoices, cameraExtras } from './ItemCamera';
import { sentenceCase } from './SentenceCase';
import { type ClosedRange, SettingStepper, useKeyedState } from './SettingStepper';
import type { WaypointHold } from './WaypointSpeed';

export type WaypointYaw = { degrees: number | null; units: string; path: string };

export const waypointYaw = (view: Json | null): WaypointYaw | null => {
  const yaw = optJSONObject(view, 'yaw');
  if (yaw === null) return null;
  const value = optDouble(yaw, 'value');
  const units = optText(yaw, 'units');
  const read = { degrees: isNull(yaw, 'value') || Number.isNaN(value) ? null : value, units: units.trim() === '' ? 'deg' : units, path: optText(yaw, 'path') };
  return read.path.trim() === '' ? null : read;
};

export type ActionKind = { readonly name: 'Hover' | 'Camera' | 'Gimbal' | 'Mode' | 'Turn'; readonly ordinal: number };

export const ActionKind = {
  Hover: { name: 'Hover', ordinal: 0 },
  Camera: { name: 'Camera', ordinal: 1 },
  Gimbal: { name: 'Gimbal', ordinal: 2 },
  Mode: { name: 'Mode', ordinal: 3 },
  Turn: { name: 'Turn', ordinal: 4 },
} as const satisfies Record<string, ActionKind>;

export const ActionKindEntries: readonly ActionKind[] = [ActionKind.Hover, ActionKind.Camera, ActionKind.Gimbal, ActionKind.Mode, ActionKind.Turn];

export type WaypointAction = { kind: ActionKind; title: string; value: string; icon: DrawableName };

export type ActionOffer = { title: string; icon: DrawableName; kind: ActionKind; choice: number };

const DEFAULT_HOVER_SECONDS = 5.0;
const NO_CAMERA_ACTION = 0;
const HOVER_RANGE: ClosedRange = { start: 0.0, endInclusive: 600.0 };
const PITCH_RANGE: ClosedRange = { start: -90.0, endInclusive: 30.0 };
const HEADING_RANGE: ClosedRange = { start: -180.0, endInclusive: 180.0 };
const INTERVAL_RANGE: ClosedRange = { start: 1.0, endInclusive: 3600.0 };
const ANGLE_STEP = 5.0;

const mentions = (label: string, word: string): boolean => label.toLowerCase().includes(word);

export const cameraIcon = (label: string): DrawableName =>
  mentions(label, 'video') && mentions(label, 'stop') ? 'plan_video_off' : mentions(label, 'video') ? 'plan_video' : mentions(label, 'stop') ? 'plan_stop' : 'plan_photo';

export const degreesText = (value: number): string => `${trimmedNumber(value)}°`;

export const cameraInterval = (extras: CameraExtras | null): string =>
  extras !== null && extras.intervalTime !== null
    ? `every ${trimmedNumber(extras.intervalTime)} s`
    : extras !== null && extras.intervalDistance !== null
      ? `every ${trimmedNumber(extras.intervalDistance)} ${extras.distanceUnits}`
      : '';

export const startingChoice = (labels: readonly string[], subject: string): number | null => {
  const at = labels.findIndex((it) => mentions(it, subject.toLowerCase()) && !mentions(it, 'stop'));
  return at > NO_CAMERA_ACTION ? at : null;
};

export const activeActions = (hold: WaypointHold | null, yaw: WaypointYaw | null, choices: CameraChoices | null, extras: CameraExtras | null): readonly WaypointAction[] => {
  const label = choices === null ? '' : (choices.labels[choices.chosen] ?? '');
  const actions: readonly (WaypointAction | null)[] = [
    hold !== null && hold.seconds > 0.0 ? { kind: ActionKind.Hover, title: 'Hover', value: `${trimmedNumber(hold.seconds)} ${hold.units}`, icon: 'plan_timer' } : null,
    choices !== null && choices.chosen > NO_CAMERA_ACTION ? { kind: ActionKind.Camera, title: sentenceCase(label), value: cameraInterval(extras), icon: cameraIcon(label) } : null,
    extras !== null && extras.commandsGimbal ? { kind: ActionKind.Gimbal, title: 'Gimbal', value: `${degreesText(extras.pitch)} / ${degreesText(extras.yaw)}`, icon: 'plan_gimbal' } : null,
    extras !== null && extras.modeSupported && extras.commandsMode ? { kind: ActionKind.Mode, title: 'Camera mode', value: CAMERA_MODES[extras.mode] ?? '', icon: 'plan_tune' } : null,
    yaw !== null && yaw.degrees !== null ? { kind: ActionKind.Turn, title: 'Turn aircraft', value: degreesText(yaw.degrees), icon: 'plan_turn' } : null,
  ];
  return actions.filter((it) => it !== null);
};

export const offeredActions = (hold: WaypointHold | null, yaw: WaypointYaw | null, choices: CameraChoices | null, extras: CameraExtras | null): readonly ActionOffer[] => {
  const camera = choices !== null && choices.chosen <= NO_CAMERA_ACTION ? choices : null;
  const photo = camera === null ? null : startingChoice(camera.labels, 'photo');
  const video = camera === null ? null : startingChoice(camera.labels, 'video');
  const offers: readonly (ActionOffer | null)[] = [
    hold !== null && hold.seconds <= 0.0 ? { title: 'Hover', icon: 'plan_timer', kind: ActionKind.Hover, choice: -1 } : null,
    photo !== null ? { title: 'Photo', icon: 'plan_photo', kind: ActionKind.Camera, choice: photo } : null,
    video !== null ? { title: 'Video', icon: 'plan_video', kind: ActionKind.Camera, choice: video } : null,
    extras !== null && !extras.commandsGimbal ? { title: 'Tilt camera', icon: 'plan_gimbal', kind: ActionKind.Gimbal, choice: -1 } : null,
    extras !== null && extras.modeSupported && !extras.commandsMode ? { title: 'Camera mode', icon: 'plan_tune', kind: ActionKind.Mode, choice: -1 } : null,
    yaw !== null && yaw.degrees === null ? { title: 'Turn aircraft', icon: 'plan_turn', kind: ActionKind.Turn, choice: -1 } : null,
  ];
  return offers.filter((it) => it !== null);
};

export const WaypointActions = ({ index, hold, yaw }: { index: number; hold: WaypointHold | null; yaw: WaypointYaw | null }) => {
  const [revision, setRevision] = useState(0);
  const camera = useProduceState<Json | null>(null, [index, revision], async (set) => set(await ItemCameraBridge.read(index)));
  const choices = cameraChoices(camera);
  const extras = cameraExtras(camera);
  const active = activeActions(hold, yaw, choices, extras);
  const offers = offeredActions(hold, yaw, choices, extras);
  const [adding, setAdding] = useState(false);
  const [editing, setEditing] = useKeyedState<ActionKind | null>([index], () => null);
  if (active.length === 0 && offers.length === 0) return null;
  const write = (work: () => Promise<boolean>) => {
    void (async () => {
      await work();
      setRevision((it) => it + 1);
    })();
  };
  const setHold = (seconds: number) => {
    if (hold !== null) write(() => setOk(hold.path, settingJson(`${seconds}`)));
  };
  const setYaw = (degrees: number | null) => {
    if (yaw !== null) write(() => setOk(yaw.path, settingJson(degrees === null ? 'null' : `${degrees}`)));
  };
  const setCamera = (member: string, value: unknown) => write(() => ItemCameraBridge.set(index, member, value));
  const chooseAction = (choice: number) => write(() => ItemCameraBridge.chooseAction(index, choice));
  const remove = (kind: ActionKind) => {
    switch (kind.name) {
      case 'Hover': return setHold(0.0);
      case 'Camera': return chooseAction(NO_CAMERA_ACTION);
      case 'Gimbal': return setCamera('specifyGimbal', false);
      case 'Mode': return setCamera('specifyCameraMode', false);
      case 'Turn': return setYaw(null);
    }
  };
  const add = (offer: ActionOffer) => {
    switch (offer.kind.name) {
      case 'Hover': return setHold(DEFAULT_HOVER_SECONDS);
      case 'Camera': return chooseAction(offer.choice);
      case 'Gimbal': return setCamera('specifyGimbal', true);
      case 'Mode': return setCamera('specifyCameraMode', true);
      case 'Turn': return setYaw(0.0);
    }
  };
  const editor = (kind: ActionKind) => {
    switch (kind.name) {
      case 'Hover':
        return hold === null ? null : <SettingStepper label="Hover" value={hold.seconds} unit={hold.units} step={1.0} onSet={setHold} range={HOVER_RANGE} slider />;
      case 'Turn':
        return <SettingStepper label="Heading" value={yaw?.degrees ?? 0.0} unit="°" step={ANGLE_STEP} onSet={(it) => setYaw(it)} range={HEADING_RANGE} slider />;
      case 'Gimbal':
        return extras === null ? null : (
          <>
            <SettingStepper label="Pitch" value={extras.pitch} unit="°" step={ANGLE_STEP} onSet={(it) => setCamera('gimbalPitch', it)} range={extras.pitchRange ?? PITCH_RANGE} slider />
            <SettingStepper label="Yaw" value={extras.yaw} unit="°" step={ANGLE_STEP} onSet={(it) => setCamera('gimbalYaw', it)} range={extras.yawRange ?? HEADING_RANGE} slider />
          </>
        );
      case 'Mode':
        return (
          <div className="flex gap-2">
            {CAMERA_MODES.map((label, at) => (
              <FilterChip key={label} selected={extras?.mode === at} onClick={() => setCamera('cameraMode', at)} label={label} />
            ))}
          </div>
        );
      case 'Camera':
        return (
          <>
            {(choices?.labels ?? []).map((label, at) =>
              at === 0 ? null : (
                <div key={at} className="flex min-h-12 w-full cursor-pointer items-center gap-3" onClick={() => chooseAction(at)}>
                  <RadioButton selected={choices?.chosen === at} onClick={null} label={sentenceCase(label)} />
                  <Text>{sentenceCase(label)}</Text>
                </div>
              ),
            )}
            {extras !== null && extras.intervalTime !== null ? <SettingStepper label="Every" value={extras.intervalTime} unit="s" step={1.0} onSet={(it) => setCamera('cameraPhotoIntervalTime', it)} range={INTERVAL_RANGE} /> : null}
            {extras !== null && extras.intervalDistance !== null ? (
              <SettingStepper label="Every" value={extras.intervalDistance} unit={extras.distanceUnits} step={1.0} onSet={(it) => setCamera('cameraPhotoIntervalDistance', it)} range={INTERVAL_RANGE} />
            ) : null}
          </>
        );
    }
  };
  return (
    <>
      <div className="flex w-full flex-col py-1">
        <div className="flex items-center">
          <Text typography="titleSmall" className="flex-1">
            Actions
          </Text>
          {offers.length > 0 ? (
            <div className="relative">
              <TextButton onClick={() => setAdding(true)}>
                <Drawable name="plan_add" size={18} />
                Add
              </TextButton>
              <DropdownMenu expanded={adding} onDismissRequest={() => setAdding(false)}>
                {offers.map((offer) => (
                  <DropdownMenuItem
                    key={offer.title}
                    text={offer.title}
                    leadingIcon={<Drawable name={offer.icon} />}
                    onClick={() => {
                      setAdding(false);
                      add(offer);
                      setEditing(offer.kind);
                    }}
                  />
                ))}
              </DropdownMenu>
            </div>
          ) : null}
        </div>
        {active.length === 0 ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            Tap Add to take a photo, film, hover or turn here.
          </Text>
        ) : null}
        <div className="flex flex-wrap gap-2">
          {active.map((action) => (
            <ActionChip key={action.kind.name} action={action} onClick={() => setEditing(action.kind)} onRemove={() => remove(action.kind)} />
          ))}
        </div>
      </div>
      {editing !== null ? (
        <AlertDialog
          onDismissRequest={() => setEditing(null)}
          title={active.find((it) => it.kind === editing)?.title ?? offers.find((it) => it.kind === editing)?.title ?? ''}
          text={<div className="flex flex-col">{editor(editing)}</div>}
          confirmButton={<TextButton onClick={() => setEditing(null)}>Done</TextButton>}
        />
      ) : null}
    </>
  );
};

const ActionChip = ({ action, onClick, onRemove }: { action: WaypointAction; onClick: () => void; onRemove: () => void }) => (
  <div className="flex items-center bg-secondary-container text-on-secondary-container" style={{ borderRadius: 14 }}>
    <button type="button" className="flex items-center pl-3 text-left" onClick={onClick}>
      <Drawable name={action.icon} size={20} />
      <span className="flex flex-col pt-1.5 pb-1.5 pl-2">
        <Text typography="labelLarge">{action.title}</Text>
        {action.value.trim() !== '' ? <Text typography="labelSmall">{action.value}</Text> : null}
      </span>
    </button>
    <IconButton onClick={onRemove} label={`Remove ${action.title}`} contentColor={md('onSecondaryContainer')}>
      <MaterialIcon name="close" filled size={18} />
    </IconButton>
  </div>
);
