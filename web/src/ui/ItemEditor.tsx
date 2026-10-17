import { useCallback, useMemo, useRef, useState } from 'react';
import { type Json, arrayJSONObject, has, isNull, jsonObjects, jsonStrings, optArrayText, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optString, optText } from '@/bridge/Json';
import { PlanCommands } from '@/bridge/BridgeCommands';
import { type Fact, type FactSlider, Qgc, makeFact } from '@/bridge/Qgc';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useKeyedState, useLaunchedEffect, useProduceState } from '@/kotlin/Compose';
import { delay, format } from '@/kotlin/Std';
import { AlertDialog, Button, Checkbox, DropdownMenu, DropdownMenuItem, FilterChip, ListItem, OutlinedTextField, RadioButton, Switch, Text, TextButton } from '@/m3';
import type { KeyboardType } from '@/m3';
import { typedNumber } from '@/map/AltitudeEntry';
import { ItemCameraSection } from '@/map/ItemCameraSection';
import { PlanBridge } from '@/map/PlanBridge';
import { sentenceCase } from '@/map/SentenceCase';
import { type SurveyStats, statisticsRows, surveyStats } from '@/map/SurveyBridge';
import { TrackPoint } from '@/map/VehicleMap';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { CameraCalcHeader, cameraCalc, shownCameraFacts } from '@/ui/CameraCalcSection';
import { useHasVehicle } from '@/ui/FlyState';
import { MissionAltitudeFrame } from '@/ui/MissionAltitudeFrame';
import { factFromControl, factSlider } from '@/ui/ParameterForm';
import { ValueDetailsSheet } from '@/ui/ParameterLinks';
import { PatternPresets, presetKind } from '@/ui/PatternPresets';
import { PlanVehicleRows } from '@/ui/PlanVehicleRows';
import { FactRow, FieldSlider, LocalTypedDiscarded } from '@/ui/SettingsScreen';
import { SectionHeader } from '@/ui/SetupRow';

export const itemFactsPath = (index: number): string => `view.itemFacts(${index})`;

export const itemCommandPath = (index: number): string => `plan.missionController.visualItems.${index}.command`;

export const itemRawEditPath = (index: number): string => `plan.missionController.visualItems.${index}.rawEdit`;

export const PRESETS_FIRST_SETTING = 'settings.planViewSettings.displayPresetsTabFirst.rawValue';

export const presetsShownFirst = (setting: Json | null): boolean => optBoolean(setting, 'value');

export const RAW_EDIT_NOTE = 'Provides advanced access to all commands/parameters. Be very careful!';

const blankToNull = (text: string): string | null => (text.trim() === '' ? null : text);

export const itemNote = (view: Json | null, rawOn: boolean): string | null => (rawOn ? RAW_EDIT_NOTE : blankToNull(optText(view, 'commandDescription')));

export const wizardLines = (view: Json | null): readonly string[] => (optBoolean(view, 'wizardMode') ? jsonStrings(optJSONArray(view, 'wizardText')) : []);

export const wizardModePath = (index: number): string => `plan.missionController.visualItems.${index}.wizardMode`;

export const commandEditable = (view: Json | null): boolean => optBoolean(view, 'simple') && !optBoolean(view, 'takeoff');

export const startCategory = (categories: readonly string[], itemCategory: string | null): string | null =>
  itemCategory !== null && categories.includes(itemCategory) ? itemCategory : (categories[0] ?? null);

export const mapCenterHintPath = (index: number): string => `plan.missionController.visualItems.${index}.setMapCenterHintForCommandChange`;

export const RAW_EDIT_STUCK = 'You have made changes to the mission item which cannot be shown in Simple Mode';

export type EntryPoint = { label: string; value: string; path: string };

export const entryPoint = (view: Json | null): EntryPoint | null => {
  const entry = optJSONObject(view, 'entryPoint');
  if (entry === null) return null;
  const point = { label: optText(entry, 'label'), value: optText(entry, 'value'), path: optText(entry, 'path') };
  return point.path.trim() === '' ? null : point;
};

export const isLandingPattern = (view: Json | null): boolean => optBoolean(view, 'landing');

export const landingNotes = (view: Json | null): readonly string[] => jsonStrings(optJSONArray(view, 'landingNotes')).filter((it) => it.trim() !== '');

export const vehicleHeading = (fact: Json | null): number | null => {
  const value = fact === null ? NaN : optDouble(fact, 'value');
  return Number.isNaN(value) ? null : value;
};

export const vehicleCoordinate = (coordinate: Json | null): Json | null =>
  coordinate !== null && optBoolean(coordinate, 'valid', true) && has(coordinate, 'latitude') && has(coordinate, 'longitude')
    ? { latitude: optDouble(coordinate, 'latitude'), longitude: optDouble(coordinate, 'longitude'), altitude: 0 }
    : null;

const setToVehicleHeading = async (index: number): Promise<string | null> => {
  const heading = vehicleHeading(await VehicleCommands.heading());
  return heading === null ? 'The vehicle has not reported its heading.' : PlanCommands.setLandingHeading(index, heading);
};

const setToVehicleLocation = async (index: number): Promise<string | null> => {
  const reported = await VehicleCommands.coordinate();
  const coordinate = vehicleCoordinate(optJSONObject(reported, 'value') ?? reported);
  return coordinate === null ? 'The vehicle has no position yet.' : PlanCommands.setLandingCoordinate(index, coordinate);
};

const DELETE_LABEL = 'Delete from the plan';

export const areaHelp = (view: Json | null): string | null => blankToNull(optText(view, 'areaHelp'));

export const gridNote = (view: Json | null): string | null => blankToNull(optText(view, 'gridNote'));

export type RawEdit = { on: boolean; friendlyAllowed: boolean };

export const rawEdit = (view: Json | null): RawEdit | null =>
  optBoolean(view, 'simple') ? { on: optBoolean(view, 'rawEdit'), friendlyAllowed: optBoolean(view, 'friendlyEditAllowed') } : null;

export const rawEditRefusal = (current: RawEdit): string | null => (current.on && !current.friendlyAllowed ? RAW_EDIT_STUCK : null);

export type CommandChoice = { id: number; name: string; description: string };

export const commandChoices = (result: unknown): readonly CommandChoice[] =>
  Array.isArray(result)
    ? jsonObjects(result).map((it) => ({ id: optInt(it, 'command'), name: optText(it, 'friendlyName'), description: optText(it, 'description') }))
    : [];

export const categoryNames = (result: unknown): readonly string[] => (Array.isArray(result) ? result.map((_, at) => optArrayText(result, at)) : []);

const OptionalFactRow = ({ fact, onWrite }: { fact: Fact; onWrite: () => void }) => (
  <div className="flex w-full items-center pl-5">
    <Switch
      checked={fact.optionalSet}
      label={fact.title}
      onCheckedChange={(on) =>
        void (async () => {
          await Qgc.set(fact.path, on ? 0.0 : null);
          onWrite();
        })()
      }
    />
    <div className="min-w-0 flex-1">
      <FactRow fact={fact.optionalSet ? fact : makeFact({ ...fact, enabled: false })} onWrite={onWrite} />
    </div>
  </div>
);

export const altitudesRelative = (view: Json | null): boolean | null =>
  optBoolean(view, 'landing') && has(view, 'altitudesAreRelative') && !isNull(view, 'altitudesAreRelative') ? optBoolean(view, 'altitudesAreRelative') : null;

const LANDING_ALTITUDES = ['finalApproachAltitude', 'landingAltitude'];

export const withLandingFrameUnits = (fields: readonly Fact[], relative: boolean | null): readonly Fact[] => {
  if (relative === null) return fields;
  const frame = relative ? 'Rel' : 'AMSL';
  return fields.map((fact) =>
    LANDING_ALTITUDES.some((it) => fact.path.endsWith(`.${it}`)) ? makeFact({ ...fact, units: [fact.units, frame].filter((it) => it.trim() !== '').join(' ') }) : fact,
  );
};

export const altitudeHint = (view: Json | null): string | null =>
  has(view, 'altitudeHint') && !isNull(view, 'altitudeHint') ? blankToNull(optString(view, 'altitudeHint')) : null;

export const previousCoordinate = (view: Json | null): readonly [number, number] | null => {
  const previous = optJSONObject(view, 'previousCoordinate');
  if (previous === null) return null;
  const latitude = optDouble(previous, 'latitude');
  const longitude = optDouble(previous, 'longitude');
  return Number.isNaN(latitude) || Number.isNaN(longitude) ? null : [latitude, longitude];
};

export const sectionStarts = (view: Json | null): ReadonlyMap<string, string> => {
  const listed = optJSONArray(view, 'fields');
  if (listed === null) return new Map();
  const rows = jsonObjects(listed).map((it) => [optString(it, 'path'), optString(it, 'section')] as const);
  return new Map(rows.filter(([, section], at) => section.trim() !== '' && section !== rows[at - 1]?.[1]));
};

export type RadioChoice = { path: string; value: boolean; selected: boolean };

export const radioChoices = (view: Json | null): ReadonlyMap<string, RadioChoice> => {
  const listed = optJSONArray(view, 'fields');
  if (listed === null) return new Map();
  return new Map(
    jsonObjects(listed).flatMap((row) => {
      const choice = optJSONObject(row, 'choice');
      return choice === null ? [] : [[optString(row, 'path'), { path: optString(choice, 'path'), value: optBoolean(choice, 'value'), selected: optBoolean(choice, 'selected') }] as const];
    }),
  );
};

const HOLD_LABEL = 'Hold';

export const withoutHeroFields = (fields: readonly Fact[], view: Json | null): readonly Fact[] =>
  optJSONObject(view, 'hold') === null ? fields : fields.filter((it) => it.description !== HOLD_LABEL);

export const itemFields = (view: Json | null): readonly Fact[] => {
  const listed = optJSONArray(view, 'fields');
  if (listed === null) return [];
  return listed.flatMap((_, at) => {
    const control = arrayJSONObject(listed, at);
    const fact = control === null ? null : factFromControl(control);
    return fact === null ? [] : [fact];
  });
};

type ItemEditorProps = { index: number; at: TrackPoint | null; mapCentre: readonly [number, number] | null; legDetail: string | null; onRemove: (() => void) | null };

export const ItemEditor = ({ index, at, mapCentre, legDetail, onRemove }: ItemEditorProps) => {
  const removing = useRef<number | null>(null);
  const discarded = useCallback(() => removing.current === index, [index]);
  return (
    <LocalTypedDiscarded.Provider value={discarded}>
      <ItemEditorBody
        index={index}
        at={at}
        mapCentre={mapCentre}
        legDetail={legDetail}
        onRemove={
          onRemove === null
            ? null
            : () => {
                removing.current = index;
                onRemove();
              }
        }
      />
    </LocalTypedDiscarded.Provider>
  );
};

const ItemEditorBody = ({ index, at, mapCentre, legDetail, onRemove }: ItemEditorProps) => {
  const [revision, setRevision] = useKeyedState([index], () => 0);
  const [view, setView] = useKeyedState<Json | null>([index], () => null);
  const [choosing, setChoosing] = useKeyedState([index], () => false);
  const [editingPosition, setEditingPosition] = useKeyedState([index], () => false);
  const [positionMenu, setPositionMenu] = useKeyedState([index], () => false);
  const [refusal, setRefusal] = useKeyedState<string | null>([index], () => null);
  const [detailsPath, setDetailsPath] = useKeyedState<string | null>([index], () => null);
  const [advanced, setAdvanced] = useKeyedState([index], () => false);
  const bump = () => setRevision((it) => it + 1);
  const refuse = (work: () => Promise<string | null>) =>
    void (async () => {
      setRefusal(await work());
      bump();
    })();

  useLaunchedEffect([index, revision], async (scope) => {
    const read = await Qgc.get(itemFactsPath(index));
    if (scope.isActive()) setView(read);
  });

  const fields = useMemo(() => withoutHeroFields(withLandingFrameUnits(itemFields(view), altitudesRelative(view)), view), [view]);
  const sections = useMemo(() => sectionStarts(view), [view]);
  const choices = useMemo(() => radioChoices(view), [view]);
  const raw = useMemo(() => rawEdit(view), [view]);
  const connected = useHasVehicle();
  const advancedShown = useAdvancedUiShown();
  const camera = useMemo(() => cameraCalc(view), [view]);
  const stats = useProduceState<SurveyStats | null>(null, [index, revision, camera !== null], async (set) => {
    set(camera === null ? null : surveyStats(await Qgc.get(`view.surveyStats(${index})`)));
  });
  const presetsFirst = useProduceState<boolean | null>(null, [index], async (set) => {
    set(presetsShownFirst(await Qgc.get(PRESETS_FIRST_SETTING)));
  });
  const startAt = mapCentre ?? previousCoordinate(view);
  const positionStart = at ?? (startAt === null ? null : TrackPoint(startAt[0], startAt[1]));
  const details = fields.find((it) => it.path === detailsPath) ?? null;
  const simple = optBoolean(view, 'simple');
  const wizard = wizardLines(view);
  const previous = previousCoordinate(view);
  const launch = (() => {
    const control = optJSONObject(view, 'launchAltitude');
    return control === null ? null : factFromControl(control);
  })();
  const relative = altitudesRelative(view);
  const hint = altitudeHint(view);
  const note = itemNote(view, raw?.on === true);
  const help = areaHelp(view);
  const grid = gridNote(view);
  const entry = entryPoint(view);
  const kind = presetKind(view);
  const presets = kind === null ? null : <PatternPresets index={index} kind={kind} onApplied={bump} />;

  const moveTo = (target: readonly [number, number] | null) => {
    setPositionMenu(false);
    void (async () => {
      const goal = target ?? geoOf(await VehicleCommands.coordinate());
      const moved = goal === null ? false : await PlanBridge.moveItem(index, goal[0], goal[1]);
      setRefusal(moved ? null : 'The item could not be moved there.');
      bump();
    })();
  };

  return (
    <>
      {details !== null ? <ValueDetailsSheet fact={details} onWrite={bump} onDismiss={() => setDetailsPath(null)} /> : null}
      <div className="flex w-full flex-col pb-2">
        {simple ? (
          <button type="button" aria-expanded={advanced} onClick={() => setAdvanced(!advanced)} className="flex min-h-12 w-full items-center px-3 text-left">
            <Text typography="titleSmall" className="min-w-0 flex-1">
              Advanced
            </Text>
            <MaterialIcon name={advanced ? 'keyboard_arrow_up' : 'keyboard_arrow_down'} filled label={advanced ? 'Hide advanced settings' : 'Show advanced settings'} />
          </button>
        ) : null}
        {!simple || advanced ? (
          <>
            {legDetail !== null ? (
              <Text typography="bodySmall" color={md('onSurfaceVariant')} className="px-5 py-1">
                {legDetail}
              </Text>
            ) : null}
            <div className="flex w-full items-center px-5">
              <div className="flex-1" />
              {at !== null || optBoolean(view, 'specifiesCoordinate') ? (
                <div className="relative">
                  <TextButton onClick={() => setPositionMenu(true)}>Position</TextButton>
                  <DropdownMenu expanded={positionMenu} onDismissRequest={() => setPositionMenu(false)}>
                    <DropdownMenuItem text="Move to Vehicle Position" enabled={connected} onClick={() => moveTo(null)} />
                    <DropdownMenuItem text="Move to Previous Item" enabled={previous !== null} onClick={() => moveTo(previous)} />
                    <DropdownMenuItem
                      text="Edit position…"
                      enabled={positionStart !== null}
                      onClick={() => {
                        setPositionMenu(false);
                        setEditingPosition(true);
                      }}
                    />
                  </DropdownMenu>
                </div>
              ) : null}
              {commandEditable(view) ? <TextButton onClick={() => setChoosing(true)}>Change command</TextButton> : null}
            </div>
            {wizard.length > 0 ? (
              <div className="flex w-full flex-col gap-2 px-5 py-2">
                <Text typography="bodyMedium">{wizard[0]}</Text>
                {wizard.slice(1).map((line, at) => (
                  <Text key={at} typography="bodySmall" color={md('onSurfaceVariant')}>
                    {line}
                  </Text>
                ))}
                {(() => {
                  const clockwise = fields.find((it) => it.path.endsWith('.loiterClockwise'));
                  return clockwise === undefined ? null : <FactRow fact={clockwise} onWrite={bump} />;
                })()}
                <Button className="w-full" onClick={() => refuse(() => Qgc.writeRefusal(wizardModePath(index), false))}>
                  Done
                </Button>
              </div>
            ) : null}
            {index === 0 ? (
              <>
                <MissionAltitudeFrame />
                <PlanVehicleRows />
                {launch !== null ? (
                  <>
                    <Text typography="titleSmall" className="px-5 py-1">
                      Launch position
                    </Text>
                    <FactRow fact={launch} subtitle="Actual position is set by the vehicle at flight time." onWrite={bump} />
                    <div className="flex w-full items-center px-5">
                      <Text typography="bodyMedium" className="min-w-0 flex-1">
                        Position
                      </Text>
                      <TextButton
                        enabled={mapCentre !== null}
                        onClick={() => {
                          if (mapCentre === null) return;
                          const [latitude, longitude] = mapCentre;
                          void (async () => {
                            const moved = await PlanBridge.moveItem(0, latitude, longitude);
                            setRefusal(moved ? null : 'The launch position could not be moved there.');
                            bump();
                          })();
                        }}
                      >
                        Set to map center
                      </TextButton>
                    </div>
                  </>
                ) : null}
              </>
            ) : null}
            {relative !== null && wizard.length === 0 ? (
              <div className="flex w-full items-center px-5">
                <Text typography="bodyMedium" className="min-w-0 flex-1">
                  Altitudes relative to launch
                </Text>
                <Switch checked={relative} label="Altitudes relative to launch" onCheckedChange={(wanted) => refuse(() => PlanCommands.setAltitudesRelative(index, wanted))} />
              </div>
            ) : null}
            {hint !== null ? (
              <Text typography="bodySmall" color={md('onSurfaceVariant')} className="px-5 py-1">
                {hint}
              </Text>
            ) : null}
            {raw !== null && advancedShown ? (
              <div className="flex w-full items-center px-5">
                <Text typography="bodyMedium" className="min-w-0 flex-1">
                  Show all values
                </Text>
                <Switch
                  checked={raw.on}
                  label="Show all values"
                  onCheckedChange={(wanted) =>
                    void (async () => {
                      setRefusal(rawEditRefusal(raw) ?? (await Qgc.writeRefusal(itemRawEditPath(index), wanted)));
                      bump();
                    })()
                  }
                />
              </div>
            ) : null}
            {note !== null ? (
              <Text typography="bodySmall" color={md('onSurfaceVariant')} className="px-5">
                {note}
              </Text>
            ) : null}
            {refusal !== null ? (
              <Text color={md('error')} className="px-5">
                {refusal}
              </Text>
            ) : null}
            {isLandingPattern(view) && connected && wizard.length === 0 ? (
              <div className="flex w-full gap-2 px-5">
                <TextButton onClick={() => refuse(() => setToVehicleHeading(index))}>Set to vehicle heading</TextButton>
                <TextButton onClick={() => refuse(() => setToVehicleLocation(index))}>Set to vehicle location</TextButton>
              </div>
            ) : null}
            {(wizard.length === 0 ? landingNotes(view) : []).map((line, at) => (
              <Text key={at} typography="bodySmall" color={aircast('warning')} className="px-5">
                {line}
              </Text>
            ))}
            {help !== null ? (
              <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="px-5 py-3">
                {help}
              </Text>
            ) : null}
            {help === null && wizard.length === 0 && presetsFirst !== null ? (
              <div className="flex flex-col">
                {presetsFirst ? presets : null}
                {fields.map((fact) => {
                  const choice = choices.get(fact.path);
                  const section = sections.get(fact.path);
                  return (
                    <div key={fact.path} className="flex flex-col">
                      {section !== undefined ? <SectionHeader text={section} /> : null}
                      {choice !== undefined ? (
                        <div className="flex w-full items-center pl-2">
                          <RadioButton selected={choice.selected} label={fact.title} onClick={() => refuse(() => Qgc.writeRefusal(choice.path, choice.value))} />
                          <div className="min-w-0 flex-1">
                            <FactRow fact={fact} fieldModifier="w-full pr-4 py-2" onWrite={bump} />
                          </div>
                        </div>
                      ) : fact.optional ? (
                        <OptionalFactRow fact={fact} onWrite={bump} />
                      ) : fact.valueDetails.trim() !== '' ? (
                        <div className="flex w-full items-center">
                          <div className="min-w-0 flex-1">
                            <FactRow fact={fact} onWrite={bump} />
                          </div>
                          <TextButton className="mr-2" onClick={() => setDetailsPath(fact.path)}>
                            ?
                          </TextButton>
                        </div>
                      ) : (
                        <FactRow fact={fact} onWrite={bump} />
                      )}
                    </div>
                  );
                })}
                {!simple ? <ItemCameraSection index={index} className="w-full px-5" onChanged={bump} /> : null}
                {camera !== null ? (
                  <>
                    <CameraCalcHeader block={camera} onWrite={(path: string, value: unknown) => refuse(() => Qgc.writeRefusal(path, value))} />
                    {shownCameraFacts(camera).map((fact: Fact) => (
                      <FactRow key={fact.path} fact={fact} onWrite={bump} />
                    ))}
                  </>
                ) : null}
                {grid !== null ? (
                  <Text typography="bodySmall" color={md('onSurfaceVariant')} className="px-5 py-1">
                    {grid}
                  </Text>
                ) : null}
                {entry !== null ? (
                  <button type="button" onClick={() => refuse(() => Qgc.refusalOf(entry.path))} className="flex min-h-12 w-full items-center px-5 text-left">
                    <Text typography="bodyMedium" className="min-w-0 flex-1">
                      {entry.label}
                    </Text>
                    <Text typography="bodyMedium">{entry.value}</Text>
                    <MaterialIcon name="keyboard_arrow_right" filled />
                  </button>
                ) : null}
                {stats !== null ? (
                  <div className="flex w-full flex-col gap-1 px-5 py-2">
                    {stats.warning.trim() !== '' ? (
                      <Text typography="bodyMedium" color={aircast('warning')}>
                        {stats.warning}
                      </Text>
                    ) : null}
                    <Text typography="titleSmall">Statistics</Text>
                    {statisticsRows(stats).map(([label, value]) => (
                      <div key={label} className="flex w-full">
                        <Text typography="bodyMedium" className="min-w-0 flex-1">
                          {label}
                        </Text>
                        <Text typography="bodyMedium">{value.trim() === '' ? '—' : value}</Text>
                      </div>
                    ))}
                  </div>
                ) : null}
                {presetsFirst === false ? presets : null}
              </div>
            ) : null}
            {onRemove !== null ? (
              <TextButton className="mx-2 self-start" contentColor={md('error')} onClick={onRemove}>
                {DELETE_LABEL}
              </TextButton>
            ) : null}
          </>
        ) : null}
      </div>
      {editingPosition && positionStart !== null ? (
        <EditPositionDialog
          at={positionStart}
          onDismiss={() => setEditingPosition(false)}
          altitudeMode={view !== null && !isNull(view, 'altitudeMode') ? optInt(view, 'altitudeMode', -1) : null}
          onAltitude={(shown) =>
            void (async () => {
              const set = await PlanBridge.setAltitude(index, shown);
              setRefusal(set ? null : 'The altitude could not be set.');
              bump();
            })()
          }
          onMove={(latitude, longitude) => {
            setEditingPosition(false);
            void (async () => {
              const moved = await PlanBridge.moveItem(index, latitude, longitude);
              setRefusal(moved ? null : 'The item could not be moved there.');
              bump();
            })();
          }}
        />
      ) : null}
      {choosing ? (
        <CommandPicker
          itemCategory={blankToNull(optText(view, 'category'))}
          onDismiss={() => setChoosing(false)}
          onChosen={(command) => {
            setChoosing(false);
            refuse(async () => {
              if (mapCentre !== null) await Qgc.refusalOf(mapCenterHintPath(index), { latitude: mapCentre[0], longitude: mapCentre[1] });
              return Qgc.writeRefusal(itemCommandPath(index), command);
            });
          }}
        />
      ) : null}
    </>
  );
};

const CommandPicker = ({ itemCategory, onDismiss, onChosen }: { itemCategory: string | null; onDismiss: () => void; onChosen: (command: number) => void }) => {
  const [categories, setCategories] = useState<readonly string[]>([]);
  const [category, setCategory] = useState<string | null>(null);
  const [commands, setCommands] = useState<readonly CommandChoice[]>([]);

  useLaunchedEffect([], async (scope) => {
    const listed = categoryNames(await PlanCommands.commandCategories());
    if (!scope.isActive()) return;
    setCategories(listed);
    setCategory(startCategory(listed, itemCategory));
  });
  useLaunchedEffect([category], async (scope) => {
    if (category === null) return;
    const listed = commandChoices(await PlanCommands.commandsForCategory(category));
    if (scope.isActive()) setCommands(listed);
  });

  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Select mission command"
      text={
        <div className="flex flex-col">
          <div className="flex overflow-x-auto">
            {categories.map((name) => (
              <FilterChip key={name} selected={name === category} onClick={() => setCategory(name)} label={sentenceCase(name)} className="mr-1 shrink-0" />
            ))}
          </div>
          <div className="max-h-[360px] overflow-y-auto">
            {commands.map((command) => (
              <ListItem
                key={command.id}
                headlineContent={sentenceCase(command.name)}
                supportingContent={<Text typography="bodySmall">{command.description}</Text>}
                onClick={() => onChosen(command.id)}
              />
            ))}
          </div>
        </div>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

export type SpeedSection = { specified: boolean; value: number | null; units: string; path: string; specifyPath: string; slider: FactSlider | null };

export const speedSection = (view: Json | null): SpeedSection | null => speedSectionOf(optJSONObject(view, 'speedSection'));

export const speedSectionOf = (served: Json | null): SpeedSection | null => {
  if (served === null || !optBoolean(served, 'available')) return null;
  const range = optJSONObject(served, 'slider');
  return {
    specified: optBoolean(served, 'specified'),
    value: isNull(served, 'value') ? null : optDouble(served, 'value'),
    units: optText(served, 'units'),
    path: optText(served, 'path'),
    specifyPath: optText(served, 'specifyPath'),
    slider: range === null ? null : factSlider(range, ''),
  };
};

const kotlinDouble = (value: number): string => (Number.isInteger(value) ? value.toFixed(1) : String(value));

export const SpeedSectionRow = ({ speed, withSlider = false, onWrite }: { speed: SpeedSection; withSlider?: boolean; onWrite: (written: () => Promise<string | null>) => void }) => {
  const [typed, setTyped] = useKeyedState([speed.value], () => (speed.value === null ? '' : kotlinDouble(speed.value)));
  const entered = typedNumber(typed);
  return (
    <div className="flex w-full flex-col px-5 py-2">
      <div className="flex items-center">
        <Text className="min-w-0 flex-1">Flight speed</Text>
        <Switch checked={speed.specified} label="Flight speed" onCheckedChange={(on) => onWrite(() => Qgc.writeRefusal(speed.specifyPath, on))} />
      </div>
      {speed.specified ? (
        <>
          <div className="flex items-center">
            <OutlinedTextField
              value={typed}
              onValueChange={setTyped}
              label={['Speed', speed.units].filter((it) => it.trim() !== '').join(' ')}
              singleLine
              keyboardType="decimal"
              className="min-w-0 flex-1"
            />
            <TextButton enabled={entered !== null} onClick={() => entered !== null && onWrite(() => Qgc.writeRefusal(speed.path, entered))}>
              Set
            </TextButton>
          </div>
          {speed.slider !== null && withSlider ? <FieldSlider value={speed.value} slider={speed.slider} label="Speed" enabled onWrite={(value) => onWrite(() => Qgc.writeRefusal(speed.path, value))} /> : null}
        </>
      ) : null}
    </div>
  );
};

export type CoordinateSystem = { readonly name: 'Geographic' | 'Utm' | 'Mgrs' | 'Vehicle'; readonly ordinal: number; readonly label: string };

export const CoordinateSystem = {
  Geographic: { name: 'Geographic', ordinal: 0, label: 'Geographic' },
  Utm: { name: 'Utm', ordinal: 1, label: 'Universal Transverse Mercator' },
  Mgrs: { name: 'Mgrs', ordinal: 2, label: 'Military Grid Reference' },
  Vehicle: { name: 'Vehicle', ordinal: 3, label: 'Vehicle Position' },
} as const satisfies Record<string, CoordinateSystem>;

export const CoordinateSystemEntries: readonly CoordinateSystem[] = [CoordinateSystem.Geographic, CoordinateSystem.Utm, CoordinateSystem.Mgrs, CoordinateSystem.Vehicle];

export type PositionForms = { zone: string; southern: boolean; easting: string; northing: string; mgrs: string };

export const positionForms = (view: Json | null): PositionForms | null => {
  if (view === null) return null;
  const utm = optJSONObject(view, 'utm');
  return {
    zone: utm === null ? '' : String(optInt(utm, 'zone')),
    southern: utm === null ? false : optBoolean(utm, 'southern'),
    easting: utm === null ? '' : format('%.2f', optDouble(utm, 'easting')),
    northing: utm === null ? '' : format('%.2f', optDouble(utm, 'northing')),
    mgrs: optText(view, 'mgrs'),
  };
};

export const positionFormsPath = (at: TrackPoint): string => format('view.positionForms(%.8f,%.8f)', at.latitude, at.longitude);

export const utmToGeoPath = (easting: string, northing: string, zone: string, southern: boolean): string => `view.utmToGeo(${easting},${northing},${zone},${southern})`;

export const geoOf = (view: Json | null): readonly [number, number] | null =>
  view !== null && optBoolean(view, 'valid') ? [optDouble(view, 'latitude'), optDouble(view, 'longitude')] : null;

const VEHICLE_POSITION_POLL_MS = 500;

export type VehiclePosition = { latitude: string; longitude: string; altitude: string };

export const vehiclePositionOf = (coordinate: Json | null, altitude: Json | null): VehiclePosition | null => {
  const geo = geoOf(coordinate);
  if (geo === null) return null;
  const shown = optText(altitude, 'valueString');
  return {
    latitude: format('%.7f', geo[0]),
    longitude: format('%.7f', geo[1]),
    altitude: altitude !== null && shown !== '' ? `${shown} ${optText(altitude, 'units')}`.trim() : '',
  };
};

export const vehicleAltitudeLabel = (altitudeMode: number | null): string | null =>
  altitudeMode === 1 ? 'Alt (Rel)' : altitudeMode === 2 ? 'Alt (AMSL)' : altitudeMode === 3 || altitudeMode === 4 ? 'Alt (AGL)' : null;

export const vehicleAltitudePath = (altitudeMode: number | null): string | null =>
  altitudeMode === 1 ? 'vehicle.altitudeRelative' : altitudeMode === 2 ? 'vehicle.altitudeAMSL' : altitudeMode === 3 || altitudeMode === 4 ? 'vehicle.altitudeAboveTerr' : null;

const PositionRow = ({ label, value }: { label: string; value: string }) => (
  <div className="flex w-full py-1">
    <Text className="min-w-0 flex-1">{label}</Text>
    <Text>{value}</Text>
  </div>
);

const PositionField = ({ label, value, keyboard = 'text', onChange }: { label: string; value: string; keyboard?: KeyboardType; onChange: (value: string) => void }) => (
  <OutlinedTextField value={value} onValueChange={onChange} label={label} singleLine keyboardType={keyboard} className="w-full" />
);

export const EditPositionDialog = ({
  at,
  onDismiss,
  title = 'Edit position',
  confirm = 'Move',
  vehicleConfirm = null,
  altitudeMode = null,
  onAltitude = null,
  onMove,
}: {
  at: TrackPoint;
  onDismiss: () => void;
  title?: string;
  confirm?: string;
  vehicleConfirm?: string | null;
  altitudeMode?: number | null;
  onAltitude?: ((shown: number) => void) | null;
  onMove: (latitude: number, longitude: number) => void;
}) => {
  const altitudePath = onAltitude !== null ? vehicleAltitudePath(altitudeMode) : null;
  const [setPosition, setSetPosition] = useState(true);
  const [setAltitude, setSetAltitude] = useState(false);
  const [system, setSystem] = useState<CoordinateSystem>(CoordinateSystem.Geographic);
  const [latitude, setLatitude] = useState(() => format('%.7f', at.latitude));
  const [longitude, setLongitude] = useState(() => format('%.7f', at.longitude));
  const [zone, setZone] = useState('');
  const [southern, setSouthern] = useState(false);
  const [easting, setEasting] = useState('');
  const [northing, setNorthing] = useState('');
  const [mgrs, setMgrs] = useState('');
  const [problem, setProblem] = useState<string | null>(null);
  const [vehicleShown, setVehicleShown] = useState<VehiclePosition | null>(null);
  const connected = useHasVehicle();

  useLaunchedEffect([system, altitudePath], async (scope) => {
    if (system !== CoordinateSystem.Vehicle || vehicleConfirm !== null) return;
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const shown = vehiclePositionOf(await VehicleCommands.coordinate(), altitudePath === null ? null : await Qgc.get(altitudePath));
      if (!scope.isActive()) return;
      setVehicleShown(shown);
      await delay(VEHICLE_POSITION_POLL_MS);
      return poll();
    };
    await poll();
  });

  useLaunchedEffect([at.latitude, at.longitude], async (scope) => {
    const forms = positionForms(await Qgc.get(positionFormsPath(at)));
    if (!scope.isActive() || forms === null) return;
    setZone(forms.zone);
    setSouthern(forms.southern);
    setEasting(forms.easting);
    setNorthing(forms.northing);
    setMgrs(forms.mgrs);
  });

  const fromVehicle = () =>
    void (async () => {
      const coordinate = geoOf(await VehicleCommands.coordinate());
      const position = setPosition ? coordinate : null;
      const read = altitudePath !== null && setAltitude ? optDouble(await Qgc.get(altitudePath), 'value', NaN) : NaN;
      const altitude = Number.isFinite(read) ? read : null;
      if (altitude !== null) onAltitude?.(altitude);
      if (setPosition && position === null) setProblem('That position could not be read.');
      else if (position !== null) onMove(position[0], position[1]);
      else onDismiss();
    })();

  const target = async (): Promise<readonly [number, number] | null> => {
    switch (system.name) {
      case 'Geographic': {
        const lat = typedNumber(latitude);
        const lon = typedNumber(longitude);
        return lat !== null && lon !== null ? [lat, lon] : null;
      }
      case 'Utm':
        return geoOf(await Qgc.get(utmToGeoPath(easting, northing, zone, southern)));
      case 'Mgrs':
        return geoOf(await Qgc.get(`view.mgrsToGeo(${mgrs.replaceAll(' ', '')})`));
      case 'Vehicle':
        return geoOf(await VehicleCommands.coordinate());
    }
  };

  const move = () => {
    if (system === CoordinateSystem.Vehicle) return fromVehicle();
    void (async () => {
      const goal = await target();
      if (goal === null) setProblem('That position could not be read.');
      else onMove(goal[0], goal[1]);
    })();
  };

  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title={title}
      text={
        <div className="flex flex-col">
          <div className="flex overflow-x-auto">
            {CoordinateSystemEntries.filter((it) => it !== CoordinateSystem.Vehicle || connected).map((choice) => (
              <FilterChip
                key={choice.name}
                selected={choice === system}
                onClick={() => {
                  setSystem(choice);
                  setProblem(null);
                }}
                label={choice.label}
                className="mr-1 shrink-0"
              />
            ))}
          </div>
          {system === CoordinateSystem.Geographic ? (
            <>
              <PositionField label="Latitude" value={latitude} onChange={setLatitude} />
              <PositionField label="Longitude" value={longitude} onChange={setLongitude} />
            </>
          ) : null}
          {system === CoordinateSystem.Utm ? (
            <>
              <PositionField label="Zone" value={zone} onChange={setZone} />
              <div className="flex items-center">
                <Text className="min-w-0 flex-1">Southern hemisphere</Text>
                <Switch checked={southern} label="Southern hemisphere" onCheckedChange={setSouthern} />
              </div>
              <PositionField label="Easting" value={easting} onChange={setEasting} />
              <PositionField label="Northing" value={northing} onChange={setNorthing} />
            </>
          ) : null}
          {system === CoordinateSystem.Mgrs ? <PositionField label="MGRS" value={mgrs} keyboard="text" onChange={setMgrs} /> : null}
          {system === CoordinateSystem.Vehicle && vehicleConfirm === null ? (
            <>
              <PositionRow label="Latitude" value={vehicleShown?.latitude ?? ''} />
              <PositionRow label="Longitude" value={vehicleShown?.longitude ?? ''} />
              {(() => {
                const label = vehicleAltitudeLabel(altitudeMode);
                return label !== null && altitudePath !== null ? <PositionRow label={label} value={vehicleShown?.altitude ?? ''} /> : null;
              })()}
              <div className="flex items-center gap-3">
                <Checkbox checked={setPosition} label="Set position from vehicle" onCheckedChange={setSetPosition} />
                <Text>Set position from vehicle</Text>
              </div>
              {altitudePath !== null ? (
                <div className="flex items-center gap-3">
                  <Checkbox checked={setAltitude} label="Set altitude from vehicle" onCheckedChange={setSetAltitude} />
                  <Text>Set altitude from vehicle</Text>
                </div>
              ) : null}
            </>
          ) : null}
          {problem !== null ? <Text color={md('error')}>{problem}</Text> : null}
        </div>
      }
      confirmButton={<TextButton onClick={move}>{vehicleConfirm !== null && system === CoordinateSystem.Vehicle ? vehicleConfirm : confirm}</TextButton>}
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};
