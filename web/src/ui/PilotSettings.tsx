import { Fragment, useContext, useMemo, useState } from 'react';
import { type Fact, Qgc, makeFact } from '@/bridge/Qgc';
import { useQgcBool, useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import type { Json } from '@/bridge/Json';
import { useProduceState } from '@/kotlin/Compose';
import { AlertDialog, Text, TextButton } from '@/m3';
import { md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { CALIBRATION, type CalibrationRoutine, type CalibrationState, calibrationState } from '@/ui/CalibrationView';
import { FLY_STATE, flyState, useHasVehicle } from '@/ui/FlyState';
import { parameterFact, parameterPath } from '@/ui/ParametersScreen';
import { factNumber } from '@/ui/Px4TuningScreen';
import { SectionHeader, SettingsCard, SetupRow, SetupState } from '@/ui/SetupRow';
import { SETUP, parametersReady } from '@/ui/SetupView';
import { CONNECTIONS_PAGE, FactRow, SettingsGroup, SettingsGroupEntries, sentenceCase } from '@/ui/SettingsScreen';
import { LocalChangeNotice, VALUE_OFF, valueText } from '@/ui/ValueRow';

export type PilotSetting = { label: string; parameters: readonly string[]; section: string; hint: string; whenUnlimited: string | null; quantity: string | null; fenceBit: number | null; reportedOnly: boolean };
export const PilotSetting = (fields: {
  label: string;
  parameters: readonly string[];
  section: string;
  hint: string;
  whenUnlimited?: string | null;
  quantity?: string | null;
  fenceBit?: number | null;
  reportedOnly?: boolean;
}): PilotSetting => ({
  whenUnlimited: null,
  quantity: null,
  fenceBit: null,
  reportedOnly: false,
  ...fields,
});

export const FENCE_MAX_ALTITUDE = 1;
export const FENCE_CIRCLE = 2;
const PX4_FENCE_HOLD = 2;
export const FENCE_PARAMETERS: readonly string[] = ['FENCE_ENABLE', 'FENCE_TYPE', 'GF_ACTION'];

export type ParameterReading = (name: string) => number | null;

export const fenceOff = (bit: number, read: ParameterReading): boolean | null => {
  const enable = read('FENCE_ENABLE');
  const action = read('GF_ACTION');
  if (enable !== null) return enable === 0 || (Math.trunc(read('FENCE_TYPE') ?? 0) & bit) === 0;
  return action === null ? null : action === 0;
};

export type ParameterWrites = readonly (readonly [string, number])[];

export const fenceTurnOn = (bit: number, read: ParameterReading): ParameterWrites => {
  const enable = read('FENCE_ENABLE');
  if (enable !== null) return [['FENCE_TYPE', enable === 0 ? bit : Math.trunc(read('FENCE_TYPE') ?? 0) | bit], ['FENCE_ENABLE', 1]];
  return read('GF_ACTION') === null ? [] : [['GF_ACTION', PX4_FENCE_HOLD]];
};

export const fenceRestore = (writes: ParameterWrites, read: ParameterReading): ParameterWrites =>
  writes.flatMap(([name]) => {
    const before = read(name);
    return before === null ? [] : [[name, before] as const];
  }).toReversed();

export const readingOf = (writes: ParameterWrites): ParameterReading => (name) => writes.find(([it]) => it === name)?.[1] ?? null;

export type ParameterWriter = (name: string, value: number) => Promise<string | null>;

type WriteOutcome = { applied: ParameterWrites; refused: string | null };

const writeEach = (writes: ParameterWrites, write: ParameterWriter): Promise<WriteOutcome> =>
  writes.reduce<Promise<WriteOutcome>>(async (previous, [name, value]) => {
    const outcome = await previous;
    if (outcome.refused !== null) return outcome;
    const refused = await write(name, value);
    return refused === null ? { ...outcome, applied: [...outcome.applied, [name, value] as const] } : { ...outcome, refused };
  }, Promise.resolve({ applied: [], refused: null }));

export const writeAllOrNone = async (writes: ParameterWrites, read: ParameterReading, write: ParameterWriter): Promise<string | null> => {
  const outcome = await writeEach(writes, write);
  if (outcome.refused !== null) await writeEach(fenceRestore(outcome.applied, read), write);
  return outcome.refused;
};

export const FENCE_OFF_NOTE = 'The geofence is off, so nothing holds the aircraft to this.';

export const fenceArmedTitle = (label: string): string => `Turn on ${label.toLowerCase()} while armed?`;

export const FENCE_ARMED_NOTE = 'If the aircraft is already past this limit, the geofence breach action, such as return home or land, may start at once.';

const groupBy = <T,>(items: readonly T[], keyOf: (item: T) => string): ReadonlyMap<string, readonly T[]> =>
  items.reduce((groups, item) => new Map(groups).set(keyOf(item), [...(groups.get(keyOf(item)) ?? []), item]), new Map<string, readonly T[]>());

const RETURN_HOME = 'Return to home';
const FLIGHT_PROTECTION = 'Flight protection';
const FAILSAFES = 'If something goes wrong';
const FLIGHT_LIMITS = 'Flight limits';
const ALTITUDE = 'altitude';
const DISTANCE = 'distance';
const SPEED = 'speed';

const ARDUPILOT_MAX_BATTERIES = 16;
const NUMBERED_BATTERY_PREFIXES = 9;

export const batteryPrefix = (index: number): string =>
  index === 0 ? 'BATT' : index < NUMBERED_BATTERY_PREFIXES ? `BATT${index + 1}` : `BATT${String.fromCharCode('A'.charCodeAt(0) + index - NUMBERED_BATTERY_PREFIXES)}`;

const ARDUPILOT_BATTERIES: readonly number[] = Array.from({ length: ARDUPILOT_MAX_BATTERIES }, (_, index) => index);

const batteryActions = (index: number): readonly PilotSetting[] => [
  PilotSetting({ label: `Battery ${index + 1} low`, parameters: [`${batteryPrefix(index)}_FS_LOW_ACT`], section: FAILSAFES, hint: `When battery ${index + 1} runs low.`, reportedOnly: true }),
  PilotSetting({ label: `Battery ${index + 1} critical`, parameters: [`${batteryPrefix(index)}_FS_CRT_ACT`], section: FAILSAFES, hint: `When battery ${index + 1} is nearly empty.`, reportedOnly: true }),
];

export const pilotSettings = (group: SettingsGroup): readonly PilotSetting[] => {
  if (group === SettingsGroup.Safety) {
    return [
      PilotSetting({ label: 'Return-to-home altitude', parameters: ['RTL_RETURN_ALT', 'RTL_ALT_M', 'RTL_ALT'], section: RETURN_HOME, hint: 'Climbs to this height before flying home.', quantity: ALTITUDE }),
      PilotSetting({
        label: 'Max altitude',
        parameters: ['GF_MAX_VER_DIST', 'FENCE_ALT_MAX'],
        section: FLIGHT_PROTECTION,
        hint: 'The aircraft will not climb above this.',
        whenUnlimited: 'Most countries cap flights at 120 m.',
        quantity: ALTITUDE,
        fenceBit: FENCE_MAX_ALTITUDE,
      }),
      PilotSetting({ label: 'Max distance', parameters: ['GF_MAX_HOR_DIST', 'FENCE_RADIUS'], section: FLIGHT_PROTECTION, hint: 'The aircraft will not fly farther from home than this.', quantity: DISTANCE, fenceBit: FENCE_CIRCLE }),
      PilotSetting({ label: 'Signal lost', parameters: ['NAV_DLL_ACT', 'FS_GCS_ENABLE', 'FS_GCS_ENABL'], section: FAILSAFES, hint: 'When the link to this app drops.' }),
      PilotSetting({ label: 'Remote controller lost', parameters: ['NAV_RCL_ACT', 'FS_THR_ENABLE'], section: FAILSAFES, hint: 'When the remote controller drops.' }),
      PilotSetting({ label: 'Low battery', parameters: ['COM_LOW_BAT_ACT', 'BATT_FS_LOW_ACT'], section: FAILSAFES, hint: 'When the battery runs low.' }),
      PilotSetting({ label: 'Critical battery', parameters: ['BATT_FS_CRT_ACT'], section: FAILSAFES, hint: 'When the battery is nearly empty.', reportedOnly: true }),
      ...ARDUPILOT_BATTERIES.slice(1).flatMap(batteryActions),
    ];
  }
  if (group === SettingsGroup.Control) {
    return [
      PilotSetting({ label: 'Max horizontal speed', parameters: ['MPC_XY_VEL_MAX', 'LOIT_SPEED_MS', 'LOIT_SPEED'], section: FLIGHT_LIMITS, hint: 'Fastest the aircraft flies forward and sideways.', quantity: SPEED }),
      PilotSetting({ label: 'Max climb speed', parameters: ['MPC_Z_VEL_MAX_UP', 'PILOT_SPD_UP', 'PILOT_SPEED_UP'], section: FLIGHT_LIMITS, hint: 'Fastest the aircraft climbs.', quantity: SPEED }),
      PilotSetting({ label: 'Max descent speed', parameters: ['MPC_Z_VEL_MAX_DN', 'PILOT_SPD_DN', 'PILOT_SPEED_DN'], section: FLIGHT_LIMITS, hint: 'Fastest the aircraft descends.', whenUnlimited: 'At 0 it descends as fast as it climbs.', quantity: SPEED }),
    ];
  }
  return [];
};

export type PilotOwner = { setting: PilotSetting; group: SettingsGroup };

export const pilotOwner = (name: string): PilotOwner | null =>
  SettingsGroupEntries.flatMap((group) => pilotSettings(group).map((setting) => ({ setting, group }))).find(({ setting }) => setting.parameters.includes(name)) ?? null;

export const pilotSearchHits = (query: string): readonly PilotSetting[] => {
  const wanted = query.trim().toLowerCase();
  if (wanted === '') return [];
  return SettingsGroupEntries.flatMap((group) => pilotSettings(group).map((setting) => ({ ...setting, section: `${group.title} › ${setting.section}` }))).filter((setting) =>
    [setting.label, setting.section, ...setting.parameters].some((it) => it.toLowerCase().includes(wanted)),
  );
};

export type ShownPilotSetting = { setting: PilotSetting; fact: Fact };

export const firstReported = (setting: PilotSetting, reported: (name: string) => Fact | null): Fact | null => {
  const found = setting.parameters.reduce<Fact | null>((first, name) => first ?? reported(name), null);
  return found === null ? null : makeFact({ ...found, shortLabel: setting.label });
};

const reportedFacts = async (setting: PilotSetting): Promise<ReadonlyMap<string, Fact | null>> =>
  new Map(await Promise.all(setting.parameters.map(async (name) => [name, await parameterFact(name, setting.quantity)] as const)));

export const PilotSettings = (props: { group: SettingsGroup } | { settings: readonly PilotSetting[] }) => {
  const group = 'group' in props ? props.group : null;
  const listed = 'settings' in props ? props.settings : null;
  const settings = useMemo(() => listed ?? (group === null ? [] : pilotSettings(group)), [group, listed]);
  const vehicle = useHasVehicle();
  if (settings.length === 0) return null;
  if (!vehicle) return <OfflinePilotSettings settings={offlineSettings(settings)} note={OFFLINE_PILOT_NOTE} offersLink />;
  return <VehiclePilotSettings settings={settings} />;
};

const VehiclePilotSettings = ({ settings }: { settings: readonly PilotSetting[] }) => {
  const setup = useQgcPath(SETUP);
  if (!parametersReady(setup)) return <OfflinePilotSettings settings={offlineSettings(settings)} note={LOADING_PILOT_NOTE} offersLink={false} />;
  return <ReportedPilotSettings settings={settings} setup={setup} />;
};

const ReportedPilotSettings = ({ settings, setup }: { settings: readonly PilotSetting[]; setup: Json | null }) => {
  const shown = useProduceState<readonly ShownPilotSetting[]>([], [settings, setup], async (set) => {
    const found = await Promise.all(
      settings.map(async (setting) => {
        const reported = await reportedFacts(setting);
        const fact = firstReported(setting, (name) => reported.get(name) ?? null);
        return fact === null ? [] : [{ setting, fact }];
      }),
    );
    set(found.flat());
  });
  return (
    <>
      {[...groupBy(shown, (it) => it.setting.section)].map(([section, rows]) => (
        <Fragment key={section}>
          <SectionHeader text={section} />
          <SettingsCard>
            {rows.map((row) => (
              <PilotFactRow key={row.fact.name} shown={row.fact} setting={row.setting} />
            ))}
          </SettingsCard>
        </Fragment>
      ))}
    </>
  );
};

export const OFFLINE_PILOT_NOTE = 'Connect the aircraft to see and change these.';
export const LOADING_PILOT_NOTE = 'Loading these from the aircraft…';
const OFFLINE_VALUE = '—';
const OFFLINE_ALPHA = 0.38;

export const offlineSettings = (settings: readonly PilotSetting[]): readonly PilotSetting[] => settings.filter((it) => !it.reportedOnly);

export const pilotSections = (settings: readonly PilotSetting[]): ReadonlyMap<string, readonly PilotSetting[]> => groupBy(settings, (setting) => setting.section);

const OfflinePilotSettings = ({ settings, note, offersLink }: { settings: readonly PilotSetting[]; note: string; offersLink: boolean }) => {
  const navigation = useContext(LocalAppNavigation);
  return (
    <>
      <div className="flex min-h-12 w-full items-center pt-3 pr-2 pl-4">
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="flex-1">
          {note}
        </Text>
        {offersLink ? <TextButton onClick={() => (navigation.settingsPage.value = CONNECTIONS_PAGE)}>Add a link</TextButton> : null}
      </div>
      {[...pilotSections(settings)].map(([section, rows]) => (
        <Fragment key={section}>
          <SectionHeader text={section} />
          <SettingsCard>
            {rows.map((setting) => (
              <OfflineRow key={setting.label} label={setting.label} hint={setting.hint} />
            ))}
          </SettingsCard>
        </Fragment>
      ))}
    </>
  );
};

const OfflineRow = ({ label, hint }: { label: string; hint: string | null }) => (
  <div className="flex min-h-14 w-full items-center gap-3 px-4 py-2">
    <div className="min-w-0 flex-1">
      <Text typography="bodyLarge" fontWeight={500} color={withAlpha(md('onSurface'), OFFLINE_ALPHA)} className="block">
        {label}
      </Text>
      {hint !== null ? (
        <Text typography="bodySmall" color={withAlpha(md('onSurfaceVariant'), OFFLINE_ALPHA)} className="block">
          {hint}
        </Text>
      ) : null}
    </div>
    <Text typography="bodyLarge" color={withAlpha(md('onSurfaceVariant'), OFFLINE_ALPHA)}>
      {OFFLINE_VALUE}
    </Text>
  </div>
);

const OFFLINE_SENSORS: readonly string[] = ['Compass', 'Accelerometer', 'Gyroscope'];

const SENSOR_CHECKS: readonly string[] = ['compass', 'accelerometer', 'gyro'];
const SENSORS_SECTION = 'Sensors';
const CALIBRATED = 'Calibrated';

export const sensorChecks = (state: CalibrationState | null): readonly CalibrationRoutine[] =>
  SENSOR_CHECKS.flatMap((id) => {
    const routine = state?.routines.find((it: CalibrationRoutine) => it.id === id);
    return routine === undefined ? [] : [routine];
  });

export const sensorHealthy = (routine: CalibrationRoutine): boolean => routine.status === CALIBRATED;

export const SensorChecks = ({ onCalibrate }: { onCalibrate: () => void }) => {
  const json = useQgcPath(CALIBRATION);
  const vehicle = useHasVehicle();
  if (!vehicle) {
    return (
      <>
        <SectionHeader text={SENSORS_SECTION} />
        <SettingsCard>
          {OFFLINE_SENSORS.map((sensor) => (
            <OfflineRow key={sensor} label={sensor} hint={null} />
          ))}
        </SettingsCard>
      </>
    );
  }
  const checks = sensorChecks(calibrationState(json));
  if (checks.length === 0) return null;
  return (
    <>
      <SectionHeader text={SENSORS_SECTION} />
      <SettingsCard>
        {checks.map((routine) => (
          <div key={routine.id} className="flex min-h-14 w-full items-center gap-2 pr-1 pl-4">
            <Text typography="bodyLarge" fontWeight={500} className="flex-1">
              {sentenceCase(routine.title)}
            </Text>
            <Text typography="bodyMedium" color={sensorHealthy(routine) ? md('onSurfaceVariant') : md('error')}>
              {sensorHealthy(routine) ? 'Normal' : routine.status.trim() === '' ? 'Not checked' : routine.status}
            </Text>
            {!sensorHealthy(routine) ? (
              <TextButton onClick={onCalibrate} enabled={routine.enabled}>
                Calibrate
              </TextButton>
            ) : null}
          </div>
        ))}
        <SetupRow title="Calibrate sensors" status="" onClick={onCalibrate} />
      </SettingsCard>
    </>
  );
};

const useFenceReading = (): ParameterReading | null => {
  const enable = useQgcPath(parameterPath('FENCE_ENABLE'));
  const type = useQgcPath(parameterPath('FENCE_TYPE'));
  const action = useQgcPath(parameterPath('GF_ACTION'));
  const read = useProduceState<ReadonlyMap<string, number | null> | null>(null, [enable, type, action], async (set) => {
    const facts = await Promise.all(FENCE_PARAMETERS.map(async (name) => [name, await parameterFact(name)] as const));
    set(new Map(facts.map(([name, fact]) => [name, fact === null ? null : factNumber(fact)])));
  });
  return read === null ? null : (name) => read.get(name) ?? null;
};

const PilotFactRow = ({ shown, setting }: { shown: Fact; setting: PilotSetting }) =>
  setting.fenceBit === null ? <PilotValueRow shown={shown} setting={setting} /> : <FencedPilotRow shown={shown} setting={setting} bit={setting.fenceBit} />;

const FencedPilotRow = ({ shown, setting, bit }: { shown: Fact; setting: PilotSetting; bit: number }) => {
  const fence = useFenceReading();
  return fence !== null && fenceOff(bit, fence) === true ? <FenceOffRow setting={setting} bit={bit} read={fence} /> : <PilotValueRow shown={shown} setting={setting} />;
};

const writeParameter: ParameterWriter = (name, value) => Qgc.writeRefusal(parameterPath(name), value);

const armedOf = (json: Json | null): boolean => flyState(json)?.armed === true;

const FenceOffRow = ({ setting, bit, read }: { setting: PilotSetting; bit: number; read: ParameterReading }) => {
  const notice = useContext(LocalChangeNotice);
  const armed = useQgcDerived(FLY_STATE, armedOf);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const turnOn = () =>
    void (async () => {
      const writes = fenceTurnOn(bit, read);
      const undo = fenceRestore(writes, read);
      const refused = await writeAllOrNone(writes, read, writeParameter);
      setRefusal(refused);
      if (refused === null) notice.show(`${setting.label}: on`, () => writeAllOrNone(undo, readingOf(writes), writeParameter));
    })();
  return (
    <div className="flex min-h-14 w-full items-center gap-3 py-2 pr-2 pl-4">
      <div className="min-w-0 flex-1">
        <Text typography="bodyLarge" fontWeight={500} className="block">
          {setting.label}
        </Text>
        <Text typography="bodySmall" color={refusal === null ? md('onSurfaceVariant') : md('error')} className="block">
          {refusal ?? FENCE_OFF_NOTE}
        </Text>
      </div>
      <Text typography="bodyLarge" color={md('onSurfaceVariant')}>
        {VALUE_OFF}
      </Text>
      <TextButton onClick={() => (armed ? setConfirming(true) : turnOn())}>Turn on</TextButton>
      {confirming ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title={fenceArmedTitle(setting.label)}
          text={FENCE_ARMED_NOTE}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(false);
                turnOn();
              }}
            >
              Turn on
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
        />
      ) : null}
    </div>
  );
};

const PilotValueRow = ({ shown, setting }: { shown: Fact; setting: PilotSetting }) => {
  const name = shown.name;
  const [revision, setRevision] = useState(0);
  const live = useQgcPath(parameterPath(name, setting.quantity));
  const fact = useProduceState<Fact>(shown, [name, live, revision], async (set) => {
    const read = await parameterFact(name, setting.quantity);
    set(read === null ? shown : makeFact({ ...read, shortLabel: shown.shortLabel }));
  });
  const hold = useHoldWord();
  return (
    <FactRow
      fact={pilotChoices(fact, hold)}
      subtitle={setting.hint}
      warning={setting.whenUnlimited !== null && factNumber(fact) === 0 ? setting.whenUnlimited : null}
      onWrite={() => setRevision((it) => it + 1)}
    />
  );
};

const DO_NOTHING = 'Do nothing';

export const PilotLinkRow = ({ name, owner }: { name: string; owner: PilotOwner }) => {
  const navigation = useContext(LocalAppNavigation);
  const live = useQgcPath(parameterPath(name, owner.setting.quantity));
  const fact = useProduceState<Fact | null>(null, [name, live], async (set) => set(await parameterFact(name, owner.setting.quantity)));
  const hold = useHoldWord();
  const status = fact === null ? '' : fact.isEnum ? pilotChoice(fact.enumStrings[fact.enumIndex] ?? fact.valueString, hold) : valueText(fact);
  return (
    <SetupRow
      title={owner.setting.label}
      subtitle={`In Settings › ${owner.group.title}`}
      status={status}
      state={status === DO_NOTHING ? SetupState.NeedsAttention : SetupState.Neutral}
      onClick={() => navigation.openSettingsTab(owner.group)}
    />
  );
};

const PILOT_CHOICES: ReadonlyMap<string, string> = new Map([
  ['Disabled', DO_NOTHING],
  ['Disabled/NoAction', DO_NOTHING],
  ['Return mode', 'Return home'],
  ['Land mode', 'Land'],
  ['Warning', 'Warn only'],
  ['Return at critical level, land at emergency level', 'Return home, land if critical'],
  ['Terminate', 'Stop motors'],
  ['RTL', 'Return home'],
  ['Enabled always RTL', 'Return home'],
  ['Enabled always Land', 'Land'],
  ['SmartRTL or RTL', 'Retrace the path home, or return home'],
  ['Enabled always SmartRTL or RTL', 'Retrace the path home, or return home'],
  ['SmartRTL or Land', 'Retrace the path home, or land'],
  ['Enabled always SmartRTL or Land', 'Retrace the path home, or land'],
  ['Brake or Land', 'Brake, or land'],
  ['Enabled always Brake or Land', 'Brake, or land'],
  ['Auto DO_LAND_START or RTL', "Fly the mission's landing, or return home"],
  ['Enabled Auto DO_LAND_START or RTL', "Fly the mission's landing, or return home"],
  ['Auto DO_LAND_START/DO_RETURN_PATH_START or RTL', "Fly the mission's landing or return path, or return home"],
  ['Enabled Auto DO_LAND_START/DO_RETURN_PATH_START or RTL', "Fly the mission's landing or return path, or return home"],
]);

const HOLD_CHOICES: ReadonlySet<string> = new Set(['Hold mode', 'Loiter mode']);

export type VehicleShape = { fixedWing: boolean; vtol: boolean; rover: boolean; sub: boolean };

export const holdWord = (shape: VehicleShape): string =>
  shape.vtol ? 'Hover, or circle in forward flight' : shape.fixedWing ? 'Circle' : shape.rover ? 'Stop' : shape.sub ? 'Hold position' : 'Hover';

export const useHoldWord = (): string =>
  holdWord({ fixedWing: useQgcBool('vehicle.fixedWing'), vtol: useQgcBool('vehicle.vtol'), rover: useQgcBool('vehicle.rover'), sub: useQgcBool('vehicle.sub') });

export const pilotChoice = (label: string, hold: string): string => (HOLD_CHOICES.has(label) ? hold : (PILOT_CHOICES.get(label) ?? label));

export const pilotChoices = (fact: Fact, hold: string): Fact => makeFact({ ...fact, enumStrings: fact.enumStrings.map((it) => pilotChoice(it, hold)) });

const PILOT_LABELS: ReadonlyMap<string, string> = new Map([
  ['guidedMinimumAltitude', 'Lowest altitude for takeoff and fly-to'],
  ['guidedMaximumAltitude', 'Highest altitude for takeoff and fly-to'],
  ['forwardFlightGoToLocationLoiterRad', 'Circle radius after fly-to (planes)'],
  ['goToLocationRequiresConfirmInGuided', 'Confirm fly-to first'],
  ['updateHomePosition', 'Home follows this device'],
  ['adsbServerConnectEnabled', 'Show nearby aircraft (ADS-B)'],
  ['adsbServerHostAddress', 'ADS-B server address'],
  ['adsbServerPort', 'ADS-B server port'],
  ['displayPresetsTabFirst', 'Open Plan on templates'],
  ['useConditionGate', 'Use gate commands in survey patterns'],
  ['takeoffItemNotRequired', 'Missions can start without a takeoff'],
  ['allowMultipleLandingPatterns', 'Allow several landing paths'],
  ['vtolTransitionDistance', 'VTOL transition distance'],
  ['keepMapCenteredOnVehicle', 'Keep the aircraft centred on the map'],
  ['showAdditionalIndicatorsCompass', 'Extra compass markers'],
  ['lockNoseUpCompass', 'North-up compass'],
  ['showObstacleDistanceOverlay', 'Show obstacle distances'],
  ['showPhotoVideoControl', 'Show the shutter button'],
  ['showSimpleCameraControl', 'Simple camera trigger'],
  ['showLogReplayStatusBar', 'Show the log replay bar'],
  ['requestControlAllowTakeover', 'Let other stations take control'],
  ['requestControlTimeout', 'Control request timeout'],
  ['valueDisplay', 'Battery shows'],
  ['threshold1', 'Battery warning level'],
  ['threshold2', 'Battery critical level'],
  ['consolidateMultipleBatteries', 'Combine multiple batteries'],
  ['streamEnabled', 'Show video'],
  ['videoFit', 'Video fit'],
  ['gridLines', 'Grid lines'],
  ['recordingFormat', 'Recording format'],
  ['maxVideoSize', 'Storage limit'],
  ['rtspTimeout', 'Connection timeout'],
  ['forceVideoDecoder', 'Video decoder'],
  ['aspectRatio', 'Aspect ratio'],
  ['defaultMissionItemAltitude', 'Default waypoint altitude'],
  ['offlineEditingFirmwareClass', 'Plan offline for firmware'],
  ['offlineEditingVehicleClass', 'Plan offline for aircraft type'],
  ['offlineEditingCruiseSpeed', 'Planning cruise speed'],
  ['offlineEditingHoverSpeed', 'Planning hover speed'],
  ['offlineEditingAscentSpeed', 'Planning climb speed'],
  ['offlineEditingDescentSpeed', 'Planning descent speed'],
  ['followTarget', "Send this device's location (Follow Me)"],
  ['enableMultiVehiclePanel', 'Show the multi-aircraft panel'],
  ['androidDontSaveToSDCard', 'Save data to the SD card'],
  ['disableAllPersistence', "Don't save any data"],
  ['horizontalDistanceUnits', 'Distance'],
  ['verticalDistanceUnits', 'Altitude'],
  ['areaUnits', 'Area'],
  ['speedUnits', 'Speed'],
  ['temperatureUnits', 'Temperature'],
  ['minZoomLevelDownload', 'Lowest zoom to download'],
  ['maxZoomLevelDownload', 'Highest zoom to download'],
]);

export const pilotWorded = (fact: Fact): Fact => {
  const label = PILOT_LABELS.get(fact.name);
  return label === undefined ? fact : makeFact({ ...fact, shortLabel: label });
};
