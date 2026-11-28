import { useState } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { useKeyedState } from '@/kotlin/Compose';
import { AlertDialog, DropdownMenu, DropdownMenuItem, FilledTonalButton, FilterChip, IconButton, RadioButton, Text, TextButton } from '@/m3';
import { Drawable, MaterialIcon } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { md } from '@/theme/Colors';
import { invokeRefusal, setOk, settingJson } from './BridgeCalls';
import { CAMERA_MODES, trimmedNumber } from './CameraSectionExtras';
import { type CameraChoices, type CameraExtras, ItemCameraBridge, cameraChoices, cameraExtras, itemCameraPath } from './ItemCamera';
import { useMapPath } from './MapBridge';
import { sentenceCase } from './SentenceCase';
import { type ClosedRange, SettingStepper } from './SettingStepper';
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

export type WaypointDrop = {
  readonly index: number | null;
  readonly wait: number;
  readonly waitOffered: boolean;
  readonly servo: boolean;
  readonly output: number | null;
  readonly pwm: number | null;
  readonly bay: number | null;
};

const numberOrNull = (view: Json, name: string): number | null => (isNull(view, name) ? null : optDouble(view, name));

export const waypointDrop = (view: Json | null): WaypointDrop | null => {
  const drop = optJSONObject(view, 'drop');
  return drop === null
    ? null
    : {
        index: isNull(drop, 'index') ? null : optInt(drop, 'index'),
        wait: optDouble(drop, 'wait'),
        waitOffered: optBoolean(drop, 'waitOffered'),
        servo: optBoolean(drop, 'servo'),
        output: numberOrNull(drop, 'output'),
        pwm: numberOrNull(drop, 'pwm'),
        bay: numberOrNull(drop, 'bay'),
      };
};

export const PLACE_DROP = 'core.plan.placeDrop';
export const REMOVE_DROP = 'core.plan.removeDrop';
export const SET_DROP_RELEASE = 'core.plan.setDropRelease';
const OUTPUT_RANGE: ClosedRange = { start: 1.0, endInclusive: 32.0 };
const PWM_RANGE: ClosedRange = { start: 500.0, endInclusive: 2500.0 };
const WAIT_RANGE: ClosedRange = { start: 0.0, endInclusive: 600.0 };
const PWM_STEP = 50.0;

export const dropText = (drop: WaypointDrop): string =>
  [
    drop.bay === null ? '' : `Bay ${trimmedNumber(drop.bay)}`,
    drop.output === null || drop.pwm === null ? 'Gripper' : `Servo ${trimmedNumber(drop.output)} to ${trimmedNumber(drop.pwm)}`,
    drop.wait > 0.0 ? `after ${trimmedNumber(drop.wait)} s` : '',
  ]
    .filter((it) => it !== '')
    .join(' · ');

export type ActionKind = { readonly name: 'Hover' | 'Camera' | 'Gimbal' | 'Mode' | 'Turn' | 'Drop'; readonly ordinal: number };

export const ActionKind = {
  Hover: { name: 'Hover', ordinal: 0 },
  Camera: { name: 'Camera', ordinal: 1 },
  Gimbal: { name: 'Gimbal', ordinal: 2 },
  Mode: { name: 'Mode', ordinal: 3 },
  Turn: { name: 'Turn', ordinal: 4 },
  Drop: { name: 'Drop', ordinal: 5 },
} as const satisfies Record<string, ActionKind>;

export const ActionKindEntries: readonly ActionKind[] = [ActionKind.Hover, ActionKind.Camera, ActionKind.Gimbal, ActionKind.Mode, ActionKind.Turn, ActionKind.Drop];

const DROP_TITLE = 'Drop payload';

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

export const activeActions = (
  hold: WaypointHold | null,
  yaw: WaypointYaw | null,
  choices: CameraChoices | null,
  extras: CameraExtras | null,
  drop: WaypointDrop | null,
): readonly WaypointAction[] => {
  const label = choices === null ? '' : (choices.labels[choices.chosen] ?? '');
  const actions: readonly (WaypointAction | null)[] = [
    hold !== null && hold.seconds > 0.0 ? { kind: ActionKind.Hover, title: 'Hover', value: `${trimmedNumber(hold.seconds)} ${hold.units}`, icon: 'plan_timer' } : null,
    choices !== null && choices.chosen > NO_CAMERA_ACTION ? { kind: ActionKind.Camera, title: sentenceCase(label), value: cameraInterval(extras), icon: cameraIcon(label) } : null,
    extras !== null && extras.commandsGimbal ? { kind: ActionKind.Gimbal, title: 'Gimbal', value: `${degreesText(extras.pitch)} / ${degreesText(extras.yaw)}`, icon: 'plan_gimbal' } : null,
    extras !== null && extras.modeSupported && extras.commandsMode ? { kind: ActionKind.Mode, title: 'Camera mode', value: CAMERA_MODES[extras.mode] ?? '', icon: 'plan_tune' } : null,
    yaw !== null && yaw.degrees !== null ? { kind: ActionKind.Turn, title: 'Turn aircraft', value: degreesText(yaw.degrees), icon: 'plan_turn' } : null,
    drop !== null && drop.index !== null ? { kind: ActionKind.Drop, title: DROP_TITLE, value: dropText(drop), icon: 'plan_drop' } : null,
  ];
  return actions.filter((it) => it !== null);
};

export const offeredActions = (hold: WaypointHold | null, yaw: WaypointYaw | null, choices: CameraChoices | null, extras: CameraExtras | null, drop: WaypointDrop | null): readonly ActionOffer[] => {
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
    drop !== null && drop.index === null ? { title: DROP_TITLE, icon: 'plan_drop', kind: ActionKind.Drop, choice: -1 } : null,
  ];
  return offers.filter((it) => it !== null);
};

const OFFER_VERBS: readonly (readonly [string, string])[] = [
  ['Photo', 'take a photo'],
  ['Video', 'film'],
  ['Hover', 'hover'],
  ['Turn aircraft', 'turn'],
  [DROP_TITLE, 'drop'],
];

export const addActionsHint = (offers: readonly ActionOffer[]): string => {
  const verbs = OFFER_VERBS.filter(([title]) => offers.some((it) => it.title === title)).map(([, verb]) => verb);
  const last = verbs.at(-1);
  if (last === undefined) return 'Tap Add action to give the camera something to do here.';
  return `Tap Add action to ${verbs.length === 1 ? last : `${verbs.slice(0, -1).join(', ')} or ${last}`} here.`;
};

export type Refusing = (label: string, work: () => Promise<string | null>) => void;

const DROP_NOT_CHANGED = 'The drop was not changed.';

type ActionWrites = {
  readonly setHold: (seconds: number) => void;
  readonly setYaw: (degrees: number | null) => void;
  readonly setCamera: (member: string, value: unknown) => void;
  readonly chooseAction: (choice: number) => void;
  readonly addDrop: (onAdded: () => void) => void;
  readonly removeDrop: () => void;
};

const actionWrites = (index: number, hold: WaypointHold | null, yaw: WaypointYaw | null, onRefusal: Refusing): ActionWrites => {
  const write = (work: () => Promise<boolean>) => void work();
  return {
    setHold: (seconds) => (hold === null ? undefined : write(() => setOk(hold.path, settingJson(`${seconds}`)))),
    setYaw: (degrees) => (yaw === null ? undefined : write(() => setOk(yaw.path, settingJson(degrees === null ? 'null' : `${degrees}`)))),
    setCamera: (member, value) => write(() => ItemCameraBridge.set(index, member, value)),
    chooseAction: (choice) => write(() => ItemCameraBridge.chooseAction(index, choice)),
    addDrop: (onAdded) =>
      onRefusal('Adding a drop', async () => {
        const refusal = await invokeRefusal(PLACE_DROP, `[${index}]`, DROP_NOT_CHANGED);
        if (refusal === null) onAdded();
        return refusal;
      }),
    removeDrop: () => onRefusal('Removing the drop', () => invokeRefusal(REMOVE_DROP, `[${index}]`, DROP_NOT_CHANGED)),
  };
};

type KindName = ActionKind['name'];

const REMOVALS: Record<KindName, (writes: ActionWrites) => void> = {
  Hover: (writes) => writes.setHold(0.0),
  Camera: (writes) => writes.chooseAction(NO_CAMERA_ACTION),
  Gimbal: (writes) => writes.setCamera('specifyGimbal', false),
  Mode: (writes) => writes.setCamera('specifyCameraMode', false),
  Turn: (writes) => writes.setYaw(null),
  Drop: (writes) => writes.removeDrop(),
};

const thenOpened = (write: (writes: ActionWrites, offer: ActionOffer) => void) => (writes: ActionWrites, offer: ActionOffer, opened: () => void) => {
  write(writes, offer);
  opened();
};

const ADDITIONS: Record<KindName, (writes: ActionWrites, offer: ActionOffer, opened: () => void) => void> = {
  Hover: thenOpened((writes) => writes.setHold(DEFAULT_HOVER_SECONDS)),
  Camera: thenOpened((writes, offer) => writes.chooseAction(offer.choice)),
  Gimbal: thenOpened((writes) => writes.setCamera('specifyGimbal', true)),
  Mode: thenOpened((writes) => writes.setCamera('specifyCameraMode', true)),
  Turn: thenOpened((writes) => writes.setYaw(0.0)),
  Drop: (writes, _offer, opened) => writes.addDrop(opened),
};

type ActionEditorProps = {
  kind: ActionKind;
  index: number;
  hold: WaypointHold | null;
  yaw: WaypointYaw | null;
  drop: WaypointDrop | null;
  choices: CameraChoices | null;
  extras: CameraExtras | null;
  writes: ActionWrites;
  onRefusal: Refusing;
};

const ActionEditor = ({ kind, index, hold, yaw, drop, choices, extras, writes, onRefusal }: ActionEditorProps) => {
  const { setHold, setYaw, setCamera, chooseAction } = writes;
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
    case 'Drop':
      return drop === null || drop.index === null ? null : <DropEditor index={index} drop={drop} onRefusal={onRefusal} />;
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

export type ActionStep = { readonly kind: string; readonly title: string; readonly seconds: number | null; readonly item: number | null; readonly earlier: boolean; readonly later: boolean };

export const actionSteps = (view: Json | null): readonly ActionStep[] =>
  jsonObjects(optJSONArray(view, 'actions')).map((step) => ({
    kind: optText(step, 'kind'),
    title: optText(step, 'title'),
    seconds: numberOrNull(step, 'seconds'),
    item: isNull(step, 'item') ? null : optInt(step, 'item'),
    earlier: optBoolean(step, 'earlier'),
    later: optBoolean(step, 'later'),
  }));

export type OrderedAction = { readonly action: WaypointAction | null; readonly title: string; readonly value: string; readonly icon: DrawableName; readonly place: number | null; readonly item: number | null; readonly earlier: boolean; readonly later: boolean };

const STEP_ICONS: Readonly<Record<string, DrawableName>> = { Wait: 'plan_timer' };

export const orderedActions = (steps: readonly ActionStep[], active: readonly WaypointAction[]): readonly OrderedAction[] => {
  const listed = steps.map((step, place): OrderedAction => {
    const action = active.find((it) => it.kind.name === step.kind) ?? null;
    return {
      action,
      title: action?.title ?? step.title,
      value: action?.value ?? (step.seconds === null ? '' : `${trimmedNumber(step.seconds)} s`),
      icon: action?.icon ?? STEP_ICONS[step.kind] ?? 'plan_tune',
      place,
      item: step.item,
      earlier: step.earlier,
      later: step.later,
    };
  });
  const unordered = active.filter((it) => !steps.some((step) => step.kind === it.kind.name)).map((action): OrderedAction => ({ action, title: action.title, value: action.value, icon: action.icon, place: null, item: null, earlier: false, later: false }));
  return [...listed, ...unordered];
};

export const MOVE_ACTION = 'core.plan.moveAction';
const REMOVE_ITEM = 'core.plan.remove';
const ACTION_NOT_CHANGED = 'The action was not changed.';

export const WaypointActions = ({ index, hold, yaw, drop, steps, onRefusal }: { index: number; hold: WaypointHold | null; yaw: WaypointYaw | null; drop: WaypointDrop | null; steps: readonly ActionStep[]; onRefusal: Refusing }) => {
  const camera = useMapPath(itemCameraPath(index));
  const choices = cameraChoices(camera);
  const extras = cameraExtras(camera);
  const active = activeActions(hold, yaw, choices, extras, drop);
  const offers = offeredActions(hold, yaw, choices, extras, drop);
  const [adding, setAdding] = useState(false);
  const [editing, setEditing] = useKeyedState<ActionKind | null>([index], () => null);
  if (active.length === 0 && offers.length === 0 && steps.length === 0) return null;
  const writes = actionWrites(index, hold, yaw, onRefusal);
  const add = (offer: ActionOffer) => ADDITIONS[offer.kind.name](writes, offer, () => setEditing(offer.kind));
  const ordered = orderedActions(steps, active);
  const remove = (step: OrderedAction) =>
    step.action !== null ? REMOVALS[step.action.kind.name](writes) : step.item === null ? undefined : onRefusal('Removing the action', () => invokeRefusal(REMOVE_ITEM, `[${step.item}]`, ACTION_NOT_CHANGED));
  const move = (step: OrderedAction, later: boolean) => (step.place === null ? undefined : onRefusal('Moving the action', () => invokeRefusal(MOVE_ACTION, `[${index}, ${step.place}, ${later}]`, ACTION_NOT_CHANGED)));
  return (
    <>
      <div className="flex w-full flex-col py-1">
        <div className="flex items-center">
          <Text typography="titleSmall" className="flex-1">
            Actions
          </Text>
          {offers.length > 0 ? (
            <div className="relative">
              <FilledTonalButton onClick={() => setAdding(true)}>
                <Drawable name="plan_add" size={18} />
                Add action
              </FilledTonalButton>
              <DropdownMenu expanded={adding} onDismissRequest={() => setAdding(false)}>
                {offers.map((offer) => (
                  <DropdownMenuItem
                    key={offer.title}
                    text={offer.title}
                    leadingIcon={<Drawable name={offer.icon} />}
                    onClick={() => {
                      setAdding(false);
                      add(offer);
                    }}
                  />
                ))}
              </DropdownMenu>
            </div>
          ) : null}
        </div>
        {ordered.length === 0 ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {addActionsHint(offers)}
          </Text>
        ) : null}
        {ordered.some((it) => it.earlier || it.later) ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            Done in this order after arriving.
          </Text>
        ) : null}
        <ol className="flex flex-col gap-2" aria-label="Actions in order">
          {ordered.map((step, at) => (
            <li key={`${step.title}-${at}`}>
              <ActionChip step={step} onClick={step.action === null ? null : () => setEditing(step.action?.kind ?? null)} onRemove={() => remove(step)} onMove={(later) => move(step, later)} />
            </li>
          ))}
        </ol>
      </div>
      {editing !== null ? (
        <AlertDialog
          onDismissRequest={() => setEditing(null)}
          title={active.find((it) => it.kind === editing)?.title ?? offers.find((it) => it.kind === editing)?.title ?? ''}
          text={
            <div className="flex flex-col">
              <ActionEditor kind={editing} index={index} hold={hold} yaw={yaw} drop={drop} choices={choices} extras={extras} writes={writes} onRefusal={onRefusal} />
            </div>
          }
          confirmButton={<TextButton onClick={() => setEditing(null)}>Done</TextButton>}
        />
      ) : null}
    </>
  );
};

const DropEditor = ({ index, drop, onRefusal }: { index: number; drop: WaypointDrop; onRefusal: Refusing }) => {
  const release = (change: string) => onRefusal('Changing the drop', () => invokeRefusal(SET_DROP_RELEASE, `[${index}, ${change}]`, DROP_NOT_CHANGED));
  const kind = (servo: boolean) => (servo === drop.servo ? undefined : release(`{"servo":${servo}}`));
  return (
    <>
      <div className="flex gap-2">
        <FilterChip selected={!drop.servo} onClick={() => kind(false)} label="Gripper" />
        <FilterChip selected={drop.servo} onClick={() => kind(true)} label="Servo" />
      </div>
      {drop.output !== null && drop.pwm !== null ? (
        <>
          <SettingStepper label="Servo output" value={drop.output} unit="" step={1.0} onSet={(it) => release(`{"output":${it}}`)} range={OUTPUT_RANGE} />
          <SettingStepper label="Release PWM" value={drop.pwm} unit="µs" step={PWM_STEP} onSet={(it) => release(`{"pwm":${it}}`)} range={PWM_RANGE} />
        </>
      ) : null}
      {drop.waitOffered ? (
        <SettingStepper
          label="Wait before dropping"
          value={drop.wait}
          unit="s"
          step={1.0}
          onSet={(it) => onRefusal('Changing the wait', () => invokeRefusal(PLACE_DROP, `[${index}, ${it}]`, DROP_NOT_CHANGED))}
          range={WAIT_RANGE}
        />
      ) : null}
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {drop.waitOffered
          ? 'Photos and camera moves happen on arrival; the drone then waits this long over the point and drops. The servo and PWM belong to this drop, and the next drop starts from them.'
          : 'A plane flies through its drop without stopping. The servo and PWM belong to this drop, and the next drop starts from them.'}
      </Text>
    </>
  );
};

const ActionChip = ({ step, onClick, onRemove, onMove }: { step: OrderedAction; onClick: (() => void) | null; onRemove: () => void; onMove: (later: boolean) => void }) => (
  <div className="flex items-center bg-secondary-container text-on-secondary-container" style={{ borderRadius: 14 }}>
    <button type="button" className="flex min-w-0 flex-1 items-center pl-3 text-left" onClick={onClick ?? undefined} disabled={onClick === null}>
      <Drawable name={step.icon} size={20} />
      <span className="flex flex-col pt-1.5 pb-1.5 pl-2">
        <Text typography="labelLarge">{step.title}</Text>
        {step.value.trim() !== '' ? <Text typography="labelSmall">{step.value}</Text> : null}
      </span>
    </button>
    {step.earlier ? (
      <IconButton onClick={() => onMove(false)} label={`Move ${step.title} earlier`} contentColor={md('onSecondaryContainer')}>
        <MaterialIcon name="arrow_upward" filled size={18} />
      </IconButton>
    ) : null}
    {step.later ? (
      <IconButton onClick={() => onMove(true)} label={`Move ${step.title} later`} contentColor={md('onSecondaryContainer')}>
        <MaterialIcon name="arrow_downward" filled size={18} />
      </IconButton>
    ) : null}
    <IconButton onClick={onRemove} label={`Remove ${step.title}`} contentColor={md('onSecondaryContainer')}>
      <MaterialIcon name="close" filled size={18} />
    </IconButton>
  </div>
);
