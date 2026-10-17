import { Fragment, useState } from 'react';
import { type Json, arrayJSONObject, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optJSONObject, optString, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { openUri } from '@/kotlin/Platform';
import { delay } from '@/kotlin/Std';
import { AlertDialog, Button, Checkbox, DropdownMenu, DropdownMenuItem, HorizontalDivider, OutlinedButton, OutlinedCard, OutlinedTextField, ScrollableTabRow, Tab, Text, TextButton } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { md } from '@/theme/Colors';
import { type ActuatorActionGroup, type ActuatorTesting, ActuatorTestSection, actuatorActions, actuatorTesting } from '@/ui/ActuatorTestSection';
import { type GeometryMotor, GeometryImage, geometryMotors } from '@/ui/GeometryImage';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow } from '@/ui/SettingsScreen';

export const ACTUATOR_OUTPUTS_VIEW = 'view.actuatorOutputs';
export const ACTUATORS_SCREEN = 'actuators';
const SHOW_BITSET = 'bitset';
const SHOW_TRUE_IF_POSITIVE = 'true-if-positive';

export type ActuatorFact = { label: string; showAs: string; bit: number; advanced: boolean; fact: Fact };
export type ActuatorColumn = { label: string; advanced: boolean; visible: boolean };
export type ActuatorChannel = { label: string; configs: readonly (ActuatorFact | null)[] };
export type ActuatorSubgroup = { label: string; primary: ActuatorFact | null; params: readonly ActuatorFact[]; columns: readonly ActuatorColumn[]; channels: readonly ActuatorChannel[] };
export type ActuatorGroup = { label: string; enable: ActuatorFact | null; groupsVisible: boolean; params: readonly ActuatorFact[]; subgroups: readonly ActuatorSubgroup[]; notes: readonly string[] };

export type GeometryCell =
  | { kind: 'Editable'; item: ActuatorFact; param: string; channelFunction: number; hidden: boolean; disabled: boolean }
  | { kind: 'Fixed'; label: string; valueString: string; advanced: boolean; hidden: boolean }
  | { kind: 'Axis'; options: readonly string[]; index: number; params: readonly string[]; advanced: boolean; hidden: boolean; disabled: boolean }
  | { kind: 'Unavailable'; label: string; advanced: boolean; hidden: boolean };

export const GeometryCell = {
  Editable: (item: ActuatorFact, param: string, channelFunction: number, hidden: boolean, disabled: boolean): GeometryCell => ({ kind: 'Editable', item, param, channelFunction, hidden, disabled }),
  Fixed: (label: string, valueString: string, advanced: boolean, hidden = false): GeometryCell => ({ kind: 'Fixed', label, valueString, advanced, hidden }),
  Axis: (options: readonly string[], index: number, params: readonly string[], advanced: boolean, hidden: boolean, disabled: boolean): GeometryCell => ({ kind: 'Axis', options, index, params, advanced, hidden, disabled }),
  Unavailable: (label: string, advanced: boolean, hidden: boolean): GeometryCell => ({ kind: 'Unavailable', label, advanced, hidden }),
};

export const PARAM_NOT_AVAILABLE = '(Param not available)';

export const ACTUATOR_MIXER_SET = 'actuatorMixer.set';
export const ACTUATOR_MIXER_AXIS = 'actuatorMixer.setAxis';
export type GeometryChannel = { label: string; cells: readonly (GeometryCell | null)[] };
export type GeometryGroup = { label: string; count: Fact | null; channels: readonly GeometryChannel[]; params: readonly ActuatorFact[] };
export type Geometry = { title: string; helpUrl: string; groups: readonly GeometryGroup[]; motors: readonly GeometryMotor[] };
export type MotorAssignmentState = { multirotor: boolean; enabled: boolean; active: boolean; message: string; highlighted: ReadonlySet<number> };

export const MotorAssignmentState = (fields: Partial<MotorAssignmentState> = {}): MotorAssignmentState => ({
  multirotor: false,
  enabled: false,
  active: false,
  message: '',
  highlighted: new Set(),
  ...fields,
});

export const MOTOR_ASSIGNMENT_INIT = 'motorAssignment.init';
export const MOTOR_ASSIGNMENT_START = 'motorAssignment.start';
export const MOTOR_ASSIGNMENT_SELECT = 'motorAssignment.selectMotor';
export const MOTOR_ASSIGNMENT_SPIN = 'motorAssignment.spinCurrentMotor';
export const MOTOR_ASSIGNMENT_ABORT = 'motorAssignment.abort';
const ASSIGNMENT_POLL_MS = 300;
const TAG = /<[^>]+>/g;

export const plainMessage = (html: string): string =>
  html
    .replaceAll('<br />', '\n')
    .replace(TAG, '')
    .split(/\r\n|\r|\n/)
    .map((it) => it.trimEnd())
    .join('\n');

const arrayInt = (value: unknown): number => {
  const number = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
  return Number.isFinite(number) ? Math.trunc(number) : 0;
};

export const motorAssignment = (json: Json | null): MotorAssignmentState =>
  json === null
    ? MotorAssignmentState()
    : MotorAssignmentState({
        multirotor: optBoolean(json, 'multirotor'),
        enabled: optBoolean(json, 'enabled'),
        active: optBoolean(json, 'active'),
        message: optText(json, 'message'),
        highlighted: new Set((optJSONArray(json, 'highlighted') ?? []).map(arrayInt)),
      });

export type ActuatorOutputs = {
  available: boolean;
  reason: string;
  showUi: boolean;
  groups: readonly ActuatorGroup[];
  testing: ActuatorTesting | null;
  geometry: Geometry | null;
  hasUnsetRequiredFunctions: boolean;
  assignment: MotorAssignmentState;
  actions: readonly ActuatorActionGroup[];
};

const objects = <T,>(list: unknown[] | null, read: (json: Json) => T | null): T[] =>
  jsonObjects(list).flatMap((it) => {
    const value = read(it);
    return value === null ? [] : [value];
  });

const actuatorFact = (json: Json | null): ActuatorFact | null => {
  if (json === null) return null;
  const fact = factFromControl(json);
  return fact === null ? null : { label: optText(json, 'label'), showAs: optText(json, 'showAs'), bit: optInt(json, 'bit'), advanced: optBoolean(json, 'advanced'), fact };
};

const strings = (json: Json, key: string): readonly string[] => jsonStrings(optJSONArray(json, key));

export const geometryCell = (json: Json | null): GeometryCell | null => {
  if (json === null) return null;
  if (optBoolean(json, 'axis')) return GeometryCell.Axis(strings(json, 'options'), optInt(json, 'index'), strings(json, 'params'), optBoolean(json, 'advanced'), optBoolean(json, 'hidden'), optBoolean(json, 'disabled'));
  if (optBoolean(json, 'unavailable')) return GeometryCell.Unavailable(optText(json, 'label'), optBoolean(json, 'advanced'), optBoolean(json, 'hidden'));
  if (optBoolean(json, 'fixed')) return GeometryCell.Fixed(optText(json, 'label'), optText(json, 'valueString'), optBoolean(json, 'advanced'), optBoolean(json, 'hidden'));
  const item = actuatorFact(json);
  return item === null ? null : GeometryCell.Editable(item, optText(json, 'param'), optInt(json, 'channelFunction'), optBoolean(json, 'hidden'), optBoolean(json, 'disabled'));
};

const cellsOf = (list: unknown[] | null): readonly (GeometryCell | null)[] => (list ?? []).map((_, index) => geometryCell(arrayJSONObject(list, index)));

export const geometry = (json: Json | null): Geometry | null =>
  json === null
    ? null
    : {
        title: optText(json, 'title'),
        helpUrl: optText(json, 'helpUrl'),
        motors: geometryMotors(json),
        groups: objects(optJSONArray(json, 'groups'), (group) => {
          const count = optJSONObject(group, 'count');
          return {
            label: optText(group, 'label'),
            count: count === null ? null : factFromControl(count),
            channels: objects(optJSONArray(group, 'channels'), (channel) => ({ label: optText(channel, 'label'), cells: cellsOf(optJSONArray(channel, 'cells')) })),
            params: objects(optJSONArray(group, 'params'), actuatorFact),
          };
        }),
      };

const configsOf = (list: unknown[] | null): readonly (ActuatorFact | null)[] => (list ?? []).map((_, index) => actuatorFact(arrayJSONObject(list, index)));

export const actuatorOutputs = (view: Json | null): ActuatorOutputs | null =>
  view === null || optString(view, 'class') !== 'ActuatorOutputs'
    ? null
    : {
        available: optBoolean(view, 'available'),
        reason: optText(view, 'reason'),
        showUi: optBoolean(view, 'showUi', true),
        testing: actuatorTesting(view),
        geometry: geometry(optJSONObject(view, 'geometry')),
        hasUnsetRequiredFunctions: optBoolean(view, 'hasUnsetRequiredFunctions'),
        assignment: motorAssignment(optJSONObject(view, 'motorAssignment')),
        actions: actuatorActions(view),
        groups: objects(optJSONArray(view, 'groups'), (group) => ({
          label: optText(group, 'label'),
          enable: actuatorFact(optJSONObject(group, 'enable')),
          groupsVisible: optBoolean(group, 'groupsVisible'),
          notes: jsonStrings(optJSONArray(group, 'notes')),
          params: objects(optJSONArray(group, 'params'), actuatorFact),
          subgroups: objects(optJSONArray(group, 'subgroups'), (subgroup) => ({
            label: optText(subgroup, 'label'),
            primary: actuatorFact(optJSONObject(subgroup, 'primary')),
            params: objects(optJSONArray(subgroup, 'params'), actuatorFact),
            columns: objects(optJSONArray(subgroup, 'columns'), (it) => ({ label: optText(it, 'label'), advanced: optBoolean(it, 'advanced'), visible: optBoolean(it, 'visible') })),
            channels: objects(optJSONArray(subgroup, 'channels'), (it) => ({ label: optText(it, 'label'), configs: configsOf(optJSONArray(it, 'configs')) })),
          })),
        })),
      };

export const rawNumber = (fact: Fact): number | null => (typeof fact.value === 'number' ? fact.value : toDoubleOrNull(fact.valueString));

const toLong = (raw: number): bigint => (Number.isFinite(raw) ? BigInt(Math.trunc(raw)) : 0n);

const bitMask = (bit: number): bigint => BigInt.asIntN(64, 1n << BigInt(bit & 63));

export const bitsetChecked = (raw: number, bit: number): boolean => (toLong(raw) & bitMask(bit)) !== 0n;

export const bitsetWritten = (raw: number, bit: number, on: boolean): number =>
  Number(BigInt.asIntN(64, on ? toLong(raw) | bitMask(bit) : toLong(raw) & ~bitMask(bit)));

export const signWritten = (raw: number, on: boolean): number => (on ? Math.abs(raw) : -Math.abs(raw));

type Write = (path: string, value: unknown) => void;

export const ActuatorsScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<ActuatorOutputs | null>(null);
  const [tab, setTab] = useState(0);
  const [advanced, setAdvanced] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const bump = () => setRevision((it) => it + 1);

  useLaunchedEffect([revision], async (scope) => {
    const next = actuatorOutputs(await Qgc.get(ACTUATOR_OUTPUTS_VIEW));
    if (scope.isActive()) setRead(next);
  });
  const assigning = read?.assignment.active === true;
  useLaunchedEffect([assigning], async (scope) => {
    const loop = async (): Promise<void> => {
      if (!assigning || !scope.isActive()) return;
      await delay(ASSIGNMENT_POLL_MS);
      if (!scope.isActive()) return;
      bump();
      return loop();
    };
    await loop();
  });

  const outputs = read;
  if (outputs === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>Reading actuator metadata.</Text>;
  }
  if (!outputs.available || !outputs.showUi) {
    return <Text className={`block p-4 ${className ?? ''}`}>{outputs.reason.trim() === '' ? 'This vehicle does not configure its actuators here.' : outputs.reason}</Text>;
  }

  const write: Write = (path, value) =>
    void (async () => {
      setRefusal(await Qgc.writeRefusal(path, value));
      bump();
    })();

  const group = outputs.groups[tab] ?? outputs.groups[0] ?? null;
  if (group === null) return null;
  const groupIndex = outputs.groups.indexOf(group);
  return (
    <div className={`flex h-full flex-col overflow-y-auto ${className ?? ''}`}>
      <div className="flex w-full items-center gap-3 px-4">
        <span className="flex-1" />
        <Checkbox checked={advanced} onCheckedChange={setAdvanced} label="Advanced" />
        <Text>Advanced</Text>
      </div>
      {outputs.geometry !== null ? (
        <div className="flex flex-col p-4">
          <GeometrySection
            geometry={outputs.geometry}
            advanced={advanced}
            editable={mixerEditable(testing, outputs.assignment.active)}
            write={write}
            highlighted={outputs.assignment.highlighted}
            onMotor={(motor) =>
              void (async () => {
                await Qgc.invoke(MOTOR_ASSIGNMENT_SELECT, motor);
                bump();
              })()
            }
            onWrite={bump}
          />
        </div>
      ) : null}
      {outputs.testing !== null ? (
        <div className="flex flex-col p-4">
          <ActuatorTestSection testing={outputs.testing} actions={outputs.actions} enabled={testing} assigning={outputs.assignment.active} onEnabled={setTesting} />
        </div>
      ) : null}
      <Text typography="titleMedium" className="block px-4">
        Actuator outputs
      </Text>
      {outputs.hasUnsetRequiredFunctions ? (
        <Text color={md('error')} className="block px-4">
          One or more actuator still needs to be assigned to an output.
        </Text>
      ) : null}
      {outputs.assignment.multirotor ? (
        <div className="flex gap-2 px-4">
          {!outputs.assignment.active && group.groupsVisible ? (
            <Button
              onClick={() =>
                void (async () => {
                  const refused = await Qgc.refusalOf(MOTOR_ASSIGNMENT_INIT, groupIndex);
                  const message = actuatorOutputs(await Qgc.get(ACTUATOR_OUTPUTS_VIEW))?.assignment.message ?? '';
                  if (refused === null) setConfirming(plainMessage(message));
                  else setFailure(plainMessage(refused));
                })()
              }
              enabled={outputs.assignment.enabled && !testing}
            >
              {'Identify & Assign Motors'}
            </Button>
          ) : null}
          {outputs.assignment.active ? (
            <>
              <OutlinedButton onClick={() => void Qgc.invoke(MOTOR_ASSIGNMENT_SPIN)}>Spin motor again</OutlinedButton>
              <OutlinedButton
                onClick={() =>
                  void (async () => {
                    await Qgc.invoke(MOTOR_ASSIGNMENT_ABORT);
                    bump();
                  })()
                }
              >
                Abort
              </OutlinedButton>
            </>
          ) : null}
        </div>
      ) : null}
      {confirming !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(null)}
          title="Motor order identification and assignment"
          text={<span className="whitespace-pre-line">{confirming}</span>}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(null);
                void (async () => {
                  await Qgc.invoke(MOTOR_ASSIGNMENT_START);
                  bump();
                })();
              }}
            >
              Yes
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(null)}>No</TextButton>}
        />
      ) : null}
      {failure !== null ? (
        <AlertDialog
          onDismissRequest={() => setFailure(null)}
          title="Error"
          text={<span className="whitespace-pre-line">{failure}</span>}
          confirmButton={<TextButton onClick={() => setFailure(null)}>Ok</TextButton>}
        />
      ) : null}
      <ScrollableTabRow selectedTabIndex={groupIndex}>
        {outputs.groups.map((each, index) => (
          <Tab key={index} selected={each === group} onClick={() => setTab(index)} text={each.label} />
        ))}
      </ScrollableTabRow>
      <div className="flex w-full flex-col gap-3 p-4">
        {group.enable !== null ? <ActuatorFactRow item={group.enable} write={write} onWrite={bump} /> : null}
        {group.groupsVisible
          ? group.subgroups.map((subgroup, subgroupIndex) => (
              <Fragment key={subgroupIndex}>
                {subgroup.label !== '' ? <Text typography="titleSmall">{subgroup.label}</Text> : null}
                {subgroup.primary !== null ? <ActuatorFactRow item={subgroup.primary} write={write} onWrite={bump} /> : null}
                {subgroup.channels.map((channel, channelIndex) => (
                  <OutlinedCard key={channelIndex} className="w-full">
                    <div className="flex flex-col p-2">
                      <Text typography="labelLarge">{channel.label}</Text>
                      {channel.configs.map((config, index) => {
                        const column = subgroup.columns[index] ?? null;
                        if (column === null || !column.visible || (!advanced && column.advanced)) return null;
                        return config !== null ? (
                          <ActuatorFactRow key={index} item={{ ...config, label: column.label }} write={write} onWrite={bump} />
                        ) : (
                          <NotAvailableRow key={index} label={column.label} />
                        );
                      })}
                    </div>
                  </OutlinedCard>
                ))}
                {subgroup.params.map((it, index) => (
                  <ActuatorFactRow key={index} item={it} write={write} onWrite={bump} />
                ))}
              </Fragment>
            ))
          : null}
        {group.params.map((it, index) => (
          <ActuatorFactRow key={index} item={it} write={write} onWrite={bump} />
        ))}
        {group.notes.map((it, index) => (
          <Text key={index} typography="bodySmall">
            {it}
          </Text>
        ))}
        {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
      </div>
    </div>
  );
};

export const mixerEditable = (testing: boolean, assigning: boolean): boolean => !testing && !assigning;

const DISABLED_ALPHA = 0.38;

const shownCell = (cell: GeometryCell, advanced: boolean): boolean => advanced || !(cell.kind === 'Editable' ? cell.item.advanced : cell.advanced);

const GeometrySection = ({
  geometry,
  advanced,
  editable,
  write,
  highlighted = new Set(),
  onMotor = () => undefined,
  onWrite,
}: {
  geometry: Geometry;
  advanced: boolean;
  editable: boolean;
  write: Write;
  highlighted?: ReadonlySet<number>;
  onMotor?: (motor: number) => void;
  onWrite: () => void;
}) => (
  <div className="flex flex-col gap-2">
    <div className="flex items-center">
      <Text typography="titleMedium" className="flex-1">
        {geometry.title}
      </Text>
      {geometry.helpUrl !== '' ? <TextButton onClick={() => openUri(geometry.helpUrl)}>?</TextButton> : null}
    </div>
    {geometry.motors.length > 1 ? <GeometryImage motors={geometry.motors} highlighted={highlighted} onMotor={onMotor} /> : null}
    <div className="relative">
      <div className="flex flex-col gap-2" style={{ opacity: editable ? 1 : DISABLED_ALPHA }} inert={!editable}>
        {geometry.groups.map((group, groupIndex) => (
          <Fragment key={groupIndex}>
            <Text typography="titleSmall">{group.label}</Text>
            {group.count !== null ? <FactRow fact={group.count} onWrite={onWrite} /> : null}
            {group.channels.map((channel, channelIndex) => (
              <OutlinedCard key={channelIndex} className="w-full">
                <div className="flex flex-col p-2">
                  <Text typography="labelLarge">{channel.label}</Text>
                  {channel.cells
                    .filter((it): it is GeometryCell => it !== null)
                    .filter((it) => !it.hidden)
                    .map((cell, index) => {
                      if (!shownCell(cell, advanced)) return null;
                      switch (cell.kind) {
                        case 'Editable':
                          return <MixerCellRow key={index} cell={cell} onWrite={onWrite} />;
                        case 'Fixed':
                          return (
                            <div key={index} className="flex">
                              <Text className="flex-1">{cell.label}</Text>
                              <Text>{cell.valueString}</Text>
                            </div>
                          );
                        case 'Axis':
                          return <AxisRow key={index} cell={cell} onWrite={onWrite} />;
                        case 'Unavailable':
                          return <NotAvailableRow key={index} label={cell.label} />;
                      }
                    })}
                </div>
              </OutlinedCard>
            ))}
            {group.params.map((it, index) => (advanced || !it.advanced ? <ActuatorFactRow key={index} item={it} write={write} onWrite={onWrite} /> : null))}
          </Fragment>
        ))}
      </div>
      {!editable ? <div className="absolute inset-0" aria-hidden /> : null}
    </div>
  </div>
);

const NotAvailableRow = ({ label }: { label: string }) => (
  <div className="flex items-center">
    <Text className="flex-1">{label}</Text>
    <Text>{PARAM_NOT_AVAILABLE}</Text>
  </div>
);

const MixerCellRow = ({ cell, onWrite }: { cell: Extract<GeometryCell, { kind: 'Editable' }>; onWrite: () => void }) => {
  const [typed, setTyped] = useKeyedState([cell.item.fact.valueString], () => cell.item.fact.valueString);
  const [open, setOpen] = useState(false);
  const set = (value: unknown) =>
    void (async () => {
      await Qgc.invoke(ACTUATOR_MIXER_SET, cell.param, value, cell.channelFunction);
      onWrite();
    })();
  const raw = rawNumber(cell.item.fact);
  if (cell.item.showAs === SHOW_TRUE_IF_POSITIVE && raw !== null) {
    return (
      <div className="flex items-center">
        <Text className="flex-1">{cell.item.label}</Text>
        <Checkbox checked={raw > 0.0} enabled={!cell.disabled} onCheckedChange={(it) => set(signWritten(raw, it))} label={cell.item.label} />
      </div>
    );
  }
  if (cell.item.showAs === SHOW_BITSET && raw !== null) {
    return (
      <div className="flex items-center">
        <Text className="flex-1">{cell.item.label}</Text>
        <Checkbox checked={bitsetChecked(raw, cell.item.bit)} enabled={!cell.disabled} onCheckedChange={(it) => set(bitsetWritten(raw, cell.item.bit, it))} label={cell.item.label} />
      </div>
    );
  }
  if (cell.item.fact.enumStrings.length > 0) {
    return (
      <div className="flex items-center">
        <Text className="flex-1">{cell.item.label}</Text>
        <div className="relative">
          <OutlinedButton onClick={() => setOpen(true)} enabled={!cell.disabled}>
            {cell.item.fact.valueString}
          </OutlinedButton>
          <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
            {cell.item.fact.enumStrings.map((label, index) => (
              <DropdownMenuItem
                key={index}
                text={label}
                onClick={() => {
                  setOpen(false);
                  const value = toDoubleOrNull(cell.item.fact.enumValues[index] ?? '');
                  if (value !== null) set(value);
                }}
              />
            ))}
          </DropdownMenu>
        </div>
      </div>
    );
  }
  return (
    <div className="flex items-center">
      <Text className="flex-1">{cell.item.label}</Text>
      <OutlinedTextField
        value={typed}
        onValueChange={setTyped}
        enabled={!cell.disabled}
        singleLine
        suffix={cell.item.fact.units}
        keyboardType="decimal"
        onDone={() => {
          const value = toDoubleOrNull(typed);
          if (value !== null) set(value);
        }}
        className="flex-1"
      />
    </div>
  );
};

const AxisRow = ({ cell, onWrite }: { cell: Extract<GeometryCell, { kind: 'Axis' }>; onWrite: () => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex items-center">
      <Text className="flex-1">Axis</Text>
      <div className="relative">
        <OutlinedButton onClick={() => setOpen(true)} enabled={!cell.disabled}>
          {cell.options[cell.index] ?? ''}
        </OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {cell.options.map((label, index) => (
            <DropdownMenuItem
              key={index}
              text={label}
              onClick={() => {
                setOpen(false);
                void (async () => {
                  await Qgc.invoke(ACTUATOR_MIXER_AXIS, [...cell.params], index);
                  onWrite();
                })();
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};

const ActuatorFactRow = ({ item, write, onWrite }: { item: ActuatorFact; write: Write; onWrite: () => void }) => {
  const raw = rawNumber(item.fact);
  if (item.showAs === SHOW_BITSET && raw !== null) {
    return (
      <div className="flex items-center">
        <Text className="flex-1">{item.label}</Text>
        <Checkbox checked={bitsetChecked(raw, item.bit)} onCheckedChange={(it) => write(item.fact.path, bitsetWritten(raw, item.bit, it))} label={item.label} />
      </div>
    );
  }
  if (item.showAs === SHOW_TRUE_IF_POSITIVE && raw !== null) {
    return (
      <div className="flex items-center">
        <Text className="flex-1">{item.label}</Text>
        <Checkbox checked={raw > 0.0} onCheckedChange={(it) => write(item.fact.path, signWritten(raw, it))} label={item.label} />
      </div>
    );
  }
  return (
    <div className="flex flex-col">
      <FactRow fact={item.fact} title={item.label === '' ? item.fact.title : item.label} onWrite={onWrite} />
      <HorizontalDivider />
    </div>
  );
};
