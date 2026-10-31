import { Fragment, useContext, useMemo, useState } from 'react';
import { type Fact, makeFact } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import type { Json } from '@/bridge/Json';
import { useProduceState } from '@/kotlin/Compose';
import { Text, TextButton } from '@/m3';
import { md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { CALIBRATION, type CalibrationRoutine, type CalibrationState, calibrationState } from '@/ui/CalibrationView';
import { useHasVehicle } from '@/ui/FlyState';
import { parameterFact, parameterPath } from '@/ui/ParametersScreen';
import { factNumber } from '@/ui/Px4TuningScreen';
import { SectionHeader, SettingsCard, SetupRow } from '@/ui/SetupRow';
import { SETUP, parametersReady } from '@/ui/SetupView';
import { CONNECTIONS_PAGE, FactRow, SettingsGroup, SettingsGroupEntries, sentenceCase } from '@/ui/SettingsScreen';

export type PilotSetting = { label: string; parameters: readonly string[]; section: string; hint: string; whenUnlimited: string | null; quantity: string | null };
export const PilotSetting = (fields: { label: string; parameters: readonly string[]; section: string; hint: string; whenUnlimited?: string | null; quantity?: string | null }): PilotSetting => ({
  whenUnlimited: null,
  quantity: null,
  ...fields,
});

const groupBy = <T,>(items: readonly T[], keyOf: (item: T) => string): ReadonlyMap<string, readonly T[]> =>
  items.reduce((groups, item) => new Map(groups).set(keyOf(item), [...(groups.get(keyOf(item)) ?? []), item]), new Map<string, readonly T[]>());

const RETURN_HOME = 'Return to home';
const FLIGHT_PROTECTION = 'Flight protection';
const FAILSAFES = 'If something goes wrong';
const FLIGHT_LIMITS = 'Flight limits';
const ALTITUDE = 'altitude';
const DISTANCE = 'distance';
const SPEED = 'speed';

export const pilotSettings = (group: SettingsGroup): readonly PilotSetting[] => {
  if (group === SettingsGroup.Safety) {
    return [
      PilotSetting({ label: 'Return-to-home altitude', parameters: ['RTL_RETURN_ALT', 'RTL_ALT_M', 'RTL_ALT'], section: RETURN_HOME, hint: 'Climbs to this height before flying home.', quantity: ALTITUDE }),
      PilotSetting({ label: 'Max altitude', parameters: ['GF_MAX_VER_DIST', 'FENCE_ALT_MAX'], section: FLIGHT_PROTECTION, hint: 'The aircraft will not climb above this.', whenUnlimited: 'Most countries cap flights at 120 m.', quantity: ALTITUDE }),
      PilotSetting({ label: 'Max distance', parameters: ['GF_MAX_HOR_DIST', 'FENCE_RADIUS'], section: FLIGHT_PROTECTION, hint: 'The aircraft will not fly farther from home than this.', quantity: DISTANCE }),
      PilotSetting({ label: 'Signal lost', parameters: ['NAV_DLL_ACT', 'FS_GCS_ENABLE'], section: FAILSAFES, hint: 'When the link to this app drops.' }),
      PilotSetting({ label: 'Remote controller lost', parameters: ['NAV_RCL_ACT', 'FS_THR_ENABLE'], section: FAILSAFES, hint: 'When the remote controller drops.' }),
      PilotSetting({ label: 'Low battery', parameters: ['COM_LOW_BAT_ACT', 'BATT_FS_LOW_ACT'], section: FAILSAFES, hint: 'When the battery runs low.' }),
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
  if (!vehicle) return <OfflinePilotSettings settings={settings} note={OFFLINE_PILOT_NOTE} offersLink />;
  return <VehiclePilotSettings settings={settings} />;
};

const VehiclePilotSettings = ({ settings }: { settings: readonly PilotSetting[] }) => {
  const setup = useQgcPath(SETUP);
  if (!parametersReady(setup)) return <OfflinePilotSettings settings={settings} note={LOADING_PILOT_NOTE} offersLink={false} />;
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

const PilotFactRow = ({ shown, setting }: { shown: Fact; setting: PilotSetting }) => {
  const name = shown.name;
  const [revision, setRevision] = useState(0);
  const live = useQgcPath(parameterPath(name, setting.quantity));
  const fact = useProduceState<Fact>(shown, [name, live, revision], async (set) => {
    const read = await parameterFact(name, setting.quantity);
    set(read === null ? shown : makeFact({ ...read, shortLabel: shown.shortLabel }));
  });
  return (
    <FactRow
      fact={pilotChoices(fact)}
      subtitle={setting.hint}
      warning={setting.whenUnlimited !== null && factNumber(fact) === 0 ? setting.whenUnlimited : null}
      onWrite={() => setRevision((it) => it + 1)}
    />
  );
};

const PILOT_CHOICES: ReadonlyMap<string, string> = new Map([
  ['Disabled', 'Do nothing'],
  ['Hold mode', 'Hover'],
  ['Loiter mode', 'Hover'],
  ['Return mode', 'Return home'],
  ['Land mode', 'Land'],
  ['Warning', 'Warn only'],
  ['Return at critical level, land at emergency level', 'Return home, land if critical'],
  ['Terminate', 'Stop motors'],
]);

export const pilotChoice = (label: string): string => PILOT_CHOICES.get(label) ?? label;

export const pilotChoices = (fact: Fact): Fact => makeFact({ ...fact, enumStrings: fact.enumStrings.map(pilotChoice) });

const PILOT_LABELS: ReadonlyMap<string, string> = new Map([
  ['guidedMinimumAltitude', 'Lowest altitude for takeoff and fly-to'],
  ['guidedMaximumAltitude', 'Highest altitude for takeoff and fly-to'],
  ['forwardFlightGoToLocationLoiterRad', 'Circle radius after fly-to (planes)'],
  ['goToLocationRequiresConfirmInGuided', 'Confirm fly-to first'],
  ['updateHomePosition', 'Home follows this phone'],
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
  ['followTarget', "Send this phone's location (Follow Me)"],
  ['enableMultiVehiclePanel', 'Show the multi-aircraft panel'],
  ['androidDontSaveToSDCard', 'Save data to the SD card'],
  ['disableAllPersistence', "Don't save any data"],
]);

export const pilotWorded = (fact: Fact): Fact => {
  const label = PILOT_LABELS.get(fact.name);
  return label === undefined ? fact : makeFact({ ...fact, shortLabel: label });
};
