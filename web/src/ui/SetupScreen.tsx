import { type CSSProperties, type HTMLAttributes, type ReactNode, type SyntheticEvent, useCallback, useContext, useMemo, useState } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useBackHandler, useLaunchedEffect, useProduceState } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { delay, format, same } from '@/kotlin/Std';
import { Button, OutlinedButton, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import type { DrawableName } from '@/res/drawables';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { ACTUATORS_SCREEN, ActuatorsScreen } from '@/ui/ActuatorsScreen';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { APM_AIRFRAME_SCREEN, ApmAirframeScreen } from '@/ui/ApmAirframeScreen';
import { APM_FOLLOW_SCREEN, ApmFollowScreen } from '@/ui/ApmFollowScreen';
import { APM_SERVOS_SCREEN, ApmServosScreen } from '@/ui/ApmServosScreen';
import { APM_SUB_FRAME_SCREEN, ApmSubFrameScreen } from '@/ui/ApmSubFrameScreen';
import { APM_SUB_MOTORS_SCREEN, ApmSubMotorsScreen } from '@/ui/ApmSubMotorsScreen';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { LOOKING_TITLE, useRememberLookingHint } from '@/ui/ConnectingCard';
import { ESP_BRIDGE_SCREEN, EspBridgeScreen } from '@/ui/EspBridgeScreen';
import { FlightModesSetup } from '@/ui/FlightModesSetup';
import { JOYSTICK_SCREEN, JoystickScreen } from '@/ui/JoystickScreen';
import { MotorsScreen } from '@/ui/MotorsScreen';
import { OPTICAL_FLOW_SCREEN, OpticalFlowScreen } from '@/ui/OpticalFlowScreen';
import { ParameterForm, type ParameterRows, SETUP_PAGE_OPENED, readPage, sectionMatches } from '@/ui/ParameterForm';
import { ParametersScreen, parameterNames } from '@/ui/ParametersScreen';
import { PowerLiveCard } from '@/ui/PowerLiveCard';
import { PX4_AIRFRAME_SCREEN, Px4AirframeScreen } from '@/ui/Px4AirframeScreen';
import { Px4TuningScreen } from '@/ui/Px4TuningScreen';
import { RadioScreen } from '@/ui/RadioScreen';
import { RemoteSupportScreen } from '@/ui/RemoteSupportScreen';
import { SCRIPTING_SCREEN, ScriptingScreen } from '@/ui/ScriptingScreen';
import { SensorsScreen } from '@/ui/SensorsScreen';
import { AIRCRAFT_SETUP, DETAIL_PANE_MAX_WIDTH, LIST_DETAIL_MIN_WIDTH, LIST_PANE_WIDTH } from '@/ui/SettingsScreen';
import { FLIGHT_MODES_PAGE, MOTORS, NOT_SUPPORTED_SCREEN, PX4_TUNING_SCREEN, RADIO, REMOTE_SUPPORT, SENSORS, headCanOpen, headPage } from '@/ui/SetupPages';
import { EmptyState, LocalPageHeading, LocalTwoPane, OverridePageHeading, PageTopBar, SearchPill, SectionHeader, SetupRow, SetupState, type SummaryLine } from '@/ui/SetupRow';
import { FIRMWARE, NOTHING_TO_CONFIGURE, NOTHING_TO_CONFIGURE_TEXT, SETUP, type SetupPage, type SetupReadiness, parametersReady, setupPage, setupReadiness } from '@/ui/SetupView';
import { SYSLINK_SCREEN, SyslinkScreen } from '@/ui/SyslinkScreen';

export type FirmwareLine = { summary: string; vehicleType: string };

export const firmwareLine = (view: Json | null): FirmwareLine => ({ summary: optText(view, 'summary'), vehicleType: optText(view, 'vehicleType') });

export type SetupComponent = {
  index: number;
  name: string;
  known: string | null;
  className: string;
  needsAttention: boolean;
  blockedReason: string | null;
  prerequisite: string | null;
};

export const SetupComponent = (fields: {
  index: number;
  name: string;
  needsAttention: boolean;
  known?: string | null;
  className?: string;
  blockedReason?: string | null;
  prerequisite?: string | null;
}): SetupComponent => ({ known: null, className: '', blockedReason: null, prerequisite: null, ...fields });

export const setupIcon = (known: string | null, className: string = ''): DrawableName =>
  known === 'radio' || known === 'joystick'
    ? 'ic_gamepad'
    : known === 'flightModes'
      ? 'ic_toggle_on'
      : known === 'sensors'
        ? 'ic_sensors'
        : known === 'safety'
          ? 'ic_shield'
          : known === 'power'
            ? 'ic_bolt'
            : known === 'esc'
              ? 'ic_tune'
              : (COMPONENT_ICONS.find(([token]) => className.includes(token))?.[1] ?? 'ic_build');

export const setupSubtitle = (vehicle: string, firmware: string): string => [firmware, vehicle].filter((it) => it.trim() !== '').join(' · ');

export const readinessNote = (readiness: SetupReadiness | null, listed: boolean): string | null => {
  if (readiness === null || readiness.setupComplete === true || listed) return null;
  const note = [readiness.headline, readiness.detail].filter((it) => it.trim() !== '').join('. ');
  return note.trim() === '' ? null : note;
};

export const parameterCountText = (count: number): string | null => (count > 0 ? format('%,d parameters', count) : null);

export const firstToOpen = (components: readonly SetupComponent[], canOpen: (component: SetupComponent) => boolean): SetupComponent | undefined => {
  const openable = components.filter(canOpen);
  return openable.find((it) => it.needsAttention) ?? openable[0];
};

export const attentionAction = (className: string): string => (['Sensors', 'Radio'].some((it) => className.includes(it)) ? 'Calibrate' : 'Set up');

export const setupNote = (className: string): string => COMPONENT_NOTES.find(([token]) => className.includes(token))?.[1] ?? '';

const COMPONENT_NOTES: readonly (readonly [string, string])[] = [
  ['Failsafe', 'What it does when the battery, radio or link fails'],
  ['FlightSafety', 'Return altitude, landing speed and geofence'],
  ['Safety', 'Return altitude, landing speed and geofence'],
  ['FlightModes', 'Which mode each switch position selects'],
  ['Airframe', 'The frame type the autopilot flies'],
  ['SubFrame', 'The frame type the autopilot flies'],
  ['Gimbal', 'Camera mount axes and their limits'],
  ['Joystick', 'A gamepad as the stick instead of a radio'],
  ['Logging', 'What the autopilot records and when'],
  ['Motor', 'Spin each motor to check order and direction'],
  ['Power', 'Battery monitor, capacity and voltage'],
  ['Radio', 'Calibrate the sticks and switches'],
  ['RemoteSupport', 'Share telemetry with a support engineer'],
  ['Scripting', 'Lua scripts running on the autopilot'],
  ['Sensors', 'Compass, accelerometer and level'],
  ['AdvancedTuning', 'Every rate and filter, per axis'],
  ['Tuning', 'How it responds to the sticks'],
  ['Actuator', 'Outputs and what drives them'],
  ['Servo', 'Outputs and what drives them'],
  ['Follow', 'Follow a target or this phone'],
  ['Camera', 'Camera trigger and gimbal'],
];

const COMPONENT_ICONS: readonly (readonly [string, DrawableName])[] = [
  ['Failsafe', 'ic_warning'],
  ['Airframe', 'ic_flight'],
  ['SubFrame', 'ic_flight'],
  ['Gimbal', 'ic_photo_camera'],
  ['Logging', 'ic_description'],
  ['Motor', 'ic_speed'],
  ['RemoteSupport', 'ic_link'],
  ['Scripting', 'ic_terminal'],
  ['Tuning', 'ic_tune'],
  ['Actuator', 'ic_speed'],
  ['Servo', 'ic_tune'],
  ['Follow', 'ic_my_location'],
  ['Airspeed', 'ic_speed'],
  ['Lights', 'ic_bolt'],
  ['ESP8266', 'ic_wifi'],
  ['Syslink', 'ic_wifi'],
];

export const prerequisiteText = (first: string, wanted: string): string => `${sentenceCase(first)} has to be set up before ${sentenceCase(wanted)}.`;

export type ParameterWait = { title: string; body: string; downloadOffered: boolean };

export const ParameterWait = (fields: { title: string; body: string; downloadOffered?: boolean }): ParameterWait => ({ downloadOffered: false, ...fields });

export const PARAMETER_REFRESH = 'parameterTools.refresh';

export const PARAMETERS_STOPPED = 'Setup needs them. Disconnect and connect the link to ask again.';

export const parameterWait = (view: Json | null): ParameterWait | null => {
  if (view === null || optBoolean(view, 'parametersReady')) return null;
  const reason = optText(view, 'parametersReason');
  if (reason === '' || reason === 'noVehicle') return null;
  if (reason === 'loading') return ParameterWait({ title: 'Loading parameters from the vehicle.', body: '' });
  if (reason === 'skipped') return ParameterWait({ title: optText(view, 'parametersText'), body: '', downloadOffered: true });
  const text = optText(view, 'parametersText');
  return ParameterWait({ title: text.trim() === '' ? `This vehicle has not sent its parameters (${reason}).` : text, body: PARAMETERS_STOPPED });
};

export const parametersIncomplete = (view: Json | null): string | null =>
  view !== null && optText(view, 'parametersReason') === 'incomplete' ? `Parameters Incomplete. ${optText(view, 'parametersText')}` : null;

export const SETUP_PARAMETERS_PAGE = 'Parameters';
export const SETUP_OVERVIEW_PAGE = '';
const NAVIGATION_SETTLE_MS = 500;
export const SETUP_SUMMARY = 'view.setupSummary';
export const SETUP_SUMMARY_PAGE = 'Summary';
const SETUP_SUMMARY_POLL_MS = 2000;

export const setupSummaries = (view: Json | null): ReadonlyMap<string, readonly SummaryLine[]> => {
  const listed = optJSONArray(view, 'components');
  if (listed === null) return new Map();
  return new Map(
    jsonObjects(listed).map((component) => [
      optText(component, 'name'),
      jsonObjects(optJSONArray(component, 'rows')).map((it) => ({ label: optText(it, 'label'), value: optText(it, 'value'), warn: optBoolean(it, 'warn') })),
    ]),
  );
};

export const setupMatches = (name: string, search: string): boolean => search.trim() === '' || name.toLowerCase().includes(search.trim().toLowerCase());

export const splitDesktopOnly = (components: readonly SetupComponent[], opensHere: (component: SetupComponent) => boolean): readonly [SetupComponent[], SetupComponent[]] => [
  components.filter(opensHere),
  components.filter((it) => !opensHere(it)),
];

export const remainingSetup = (components: readonly SetupComponent[]): SetupComponent[] => components.filter((it) => !it.needsAttention);

export const useHasSetupPage = (name: string): boolean => {
  const setupJson = useQgcPath(SETUP);
  return useMemo(() => setupComponents(setupJson).some((it) => it.name === name), [setupJson, name]);
};

const textOrNull = (json: Json, key: string): string | null => {
  const text = optText(json, key);
  return isNull(json, key) || text.trim() === '' ? null : text;
};

export const setupComponents = (view: Json | null): readonly SetupComponent[] =>
  (optJSONArray(view, 'components') ?? []).flatMap((element, index) => {
    if (typeof element !== 'object' || element === null || Array.isArray(element)) return [];
    const json = element as Json;
    const name = optText(json, 'name');
    return name.trim() === ''
      ? []
      : [
          SetupComponent({
            index,
            name,
            known: textOrNull(json, 'known'),
            className: optText(json, 'className'),
            needsAttention: optBoolean(json, 'needsAttention'),
            blockedReason: textOrNull(json, 'blockedReason'),
            prerequisite: textOrNull(json, 'prerequisite'),
          }),
        ];
  });

const OWN_SCREEN_HEADS: ReadonlySet<string> = new Set([SENSORS, RADIO, REMOTE_SUPPORT, MOTORS, FLIGHT_MODES_PAGE]);

const SectionHits = ({ titles, onOpen }: { titles: readonly string[]; onOpen: (title: string) => void }) =>
  titles.length === 0 ? null : (
    <div className="flex flex-col items-start">
      {titles.map((title) => (
        <TextButton key={title} onClick={() => onOpen(title)} className="ml-14">
          {sentenceCase(title)}
        </TextButton>
      ))}
    </div>
  );

export const disabledWhile = (reason: string): string => `Disabled while the vehicle is ${reason}`;

const consume = (event: SyntheticEvent): void => {
  event.stopPropagation();
  event.preventDefault();
};

export const swallowTouches = (className: string = ''): HTMLAttributes<HTMLDivElement> => ({
  className,
  onPointerDownCapture: consume,
  onPointerUpCapture: consume,
  onClickCapture: consume,
  onDoubleClickCapture: consume,
  onContextMenuCapture: consume,
});

const SetupNotice = ({ text, className }: { text: string; className?: string }) => (
  <Text typography="bodyLarge" textAlign="center" className={`w-full p-6 ${className ?? ''}`}>
    {text}
  </Text>
);

export const reportsOpening = (prerequisite: string | null, page: SetupPage | null): boolean =>
  prerequisite !== null || (page?.parameterSections !== true && (page?.screen ?? null) !== NOT_SUPPORTED_SCREEN);

const useMaxWidth = (): readonly [(node: HTMLDivElement | null) => (() => void) | undefined, number] => {
  const [maxWidth, setMaxWidth] = useState(0);
  const measure = useCallback((node: HTMLDivElement | null) => {
    if (node === null) return undefined;
    setMaxWidth(node.getBoundingClientRect().width);
    if (typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(([entry]) => setMaxWidth(entry?.contentRect.width ?? 0));
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  return [measure, maxWidth];
};

type Opened = readonly [SetupComponent, string | null];

export const SetupScreen = ({ className }: { className?: string }) => {
  const navigation = useContext(LocalAppNavigation);
  const requestedPage = useValue(navigation.setupPage);
  const outerTwoPane = useContext(LocalTwoPane);
  const setupJson = useQgcPath(SETUP);
  const setup = useMemo(() => setupReadiness(setupJson), [setupJson]);
  const hasVehicle = setup?.connected === true;
  const firmwareJson = useQgcPath(FIRMWARE);
  const line = useMemo(() => firmwareLine(firmwareJson), [firmwareJson]);
  const vehicleType = line.vehicleType;
  const [opened, setOpened] = useState<Opened | null>(null);
  const openComponent = opened?.[0] ?? null;
  const openSection = opened?.[1] ?? null;
  const [parametersOpen, setParametersOpen] = useState(false);
  const advanced = useAdvancedUiShown();
  const [setupSearch, setSetupSearch] = useState('');
  const [parametersSearch, setParametersSearch] = useState('');
  const lookingHint = useRememberLookingHint();
  const [measure, maxWidth] = useMaxWidth();

  useBackHandler(openComponent !== null, () => setOpened(null));

  const components = useMemo(() => setupComponents(setupJson), [setupJson]);
  const searching = setupSearch.trim() !== '';
  const formSections = useProduceState<ReadonlyMap<string, readonly ParameterRows[]>>(new Map(), [searching, components], async (set) => {
    if (!searching) return set(new Map());
    const read = await Promise.all(
      components
        .filter((it) => setupPage(setupJson, it.name)?.parameterSections === true && !OWN_SCREEN_HEADS.has(headPage(it)))
        .map(async (component) => [component.name, await readPage(component.name)] as const),
    );
    set(new Map(read));
  });
  const sectionHits = new Map([...formSections].map(([name, rows]) => [name, rows.filter((it) => sectionMatches(it, setupSearch)).map((it) => it.title)]));
  const hitsOf = (name: string): readonly string[] => sectionHits.get(name) ?? [];
  const searchHit = (it: SetupComponent): boolean => setupMatches(it.name, setupSearch) || hitsOf(it.name).length > 0;
  const openFromList = (component: SetupComponent, section: string | null) => {
    setParametersOpen(false);
    setOpened([component, section]);
  };
  const [summaries, setSummaries] = useState<ReadonlyMap<string, readonly SummaryLine[]>>(new Map());

  useLaunchedEffect([hasVehicle], () => {
    if (hasVehicle) return;
    setSummaries(new Map());
    setOpened(null);
  });

  useLaunchedEffect([requestedPage, components, hasVehicle], async (scope) => {
    if (requestedPage === null || !hasVehicle || components.length === 0) return;
    if (requestedPage === SETUP_PARAMETERS_PAGE) {
      setParametersSearch(navigation.parametersSearch.value);
      navigation.parametersSearch.value = '';
      setParametersOpen(true);
    } else {
      const requested = components.find((it) => it.name === requestedPage);
      if (requested !== undefined) setOpened([requested, null]);
    }
    await delay(NAVIGATION_SETTLE_MS);
    if (scope.isActive()) navigation.setupPage.value = null;
  });

  const waiting = parameterWait(setupJson);
  const wide = maxWidth >= LIST_DETAIL_MIN_WIDTH;
  const panesShown = hasVehicle && waiting === null;

  useLaunchedEffect([wide && panesShown, components, requestedPage], () => {
    if (!(wide && panesShown) || opened !== null || parametersOpen || requestedPage !== null) return;
    const first = firstToOpen(components, (it) => headCanOpen(setupPage(setupJson, it.name), it.name));
    if (first !== undefined) setOpened([first, null]);
  });

  if (!hasVehicle) {
    return (
      <div className={`flex w-full flex-col items-center ${className ?? ''}`}>
        <EmptyState icon="ic_build" title={LOOKING_TITLE} text={lookingHint} />
        <OutlinedButton
          onClick={() => {
            navigation.settingsPage.value = 'Connections';
          }}
        >
          Set up connection
        </OutlinedButton>
      </div>
    );
  }

  if (waiting !== null) {
    return (
      <div className={`flex h-full w-full flex-col ${className ?? ''}`}>
        <SetupNotice text={waiting.title} />
        {waiting.body.trim() !== '' ? (
          <Text typography="bodyMedium" textAlign="center" color={md('onSurfaceVariant')} className="w-full px-6">
            {waiting.body}
          </Text>
        ) : null}
        {waiting.downloadOffered ? (
          <Button className="self-center" onClick={() => void Qgc.invoke(PARAMETER_REFRESH)}>
            Download parameters
          </Button>
        ) : null}
      </div>
    );
  }

  const componentOpens = openComponent !== null && headCanOpen(setupPage(setupJson, openComponent.name), openComponent.name);
  const shownDetail: ReactNode = parametersOpen ? (
    <ParametersPage initialSearch={parametersSearch} onBack={() => setParametersOpen(false)} />
  ) : componentOpens && openComponent !== null ? (
    <ComponentPage open={openComponent} section={openSection} setupJson={setupJson} onBack={() => setOpened(null)} onOpen={setOpened} />
  ) : null;
  const detailKey = parametersOpen ? 'parameters' : (openComponent?.name ?? '');

  const overview = (paneClass: string, paneStyle?: CSSProperties) => (
    <SetupOverview
      className={paneClass}
      style={paneStyle}
      setup={setup}
      vehicleType={vehicleType}
      firmware={line.summary}
      setupJson={setupJson}
      components={components}
      needSetup={components.filter((it) => it.needsAttention && searchHit(it))}
      hitsOf={hitsOf}
      searchHit={searchHit}
      setupSearch={setupSearch}
      onSetupSearch={setSetupSearch}
      onSearchParameters={() => {
        if (advanced && setupSearch.trim() !== '') {
          setParametersSearch(setupSearch.trim());
          setParametersOpen(true);
        }
      }}
      advanced={advanced}
      openComponent={openComponent}
      onOpenFromList={openFromList}
      summaries={summaries}
      onSummaries={setSummaries}
      parametersOpen={parametersOpen}
      onOpenParameters={() => {
        setParametersSearch('');
        setOpened(null);
        setParametersOpen(true);
      }}
    />
  );

  return (
    <div ref={measure} className={`flex h-full w-full ${className ?? ''}`}>
      {wide || shownDetail === null ? (wide ? overview('h-full shrink-0', { width: LIST_PANE_WIDTH, background: md('surfaceContainerLow') }) : overview('h-full w-full')) : null}
      {wide || shownDetail !== null ? (
        <div className="relative h-full min-w-0 flex-1">
          <LocalTwoPane.Provider value={wide || outerTwoPane}>
            {shownDetail !== null ? (
              <div key={detailKey} className="flex h-full w-full flex-col" style={wide ? { maxWidth: DETAIL_PANE_MAX_WIDTH } : undefined}>
                {shownDetail}
              </div>
            ) : (
              <EmptyState icon="ic_build" title={AIRCRAFT_SETUP} text="Choose a component on the left." />
            )}
          </LocalTwoPane.Provider>
        </div>
      ) : null}
    </div>
  );
};

const ParametersPage = ({ initialSearch, onBack }: { initialSearch: string; onBack: () => void }) => (
  <>
    <SetupPageBar title="Parameters" onBack={onBack} />
    <ParametersScreen className="min-h-0 flex-1" initialSearch={initialSearch} />
  </>
);

const Prerequisite = ({ first, wanted, setupJson, onOpen }: { first: string; wanted: string; setupJson: Json | null; onOpen: (opened: Opened | null) => void }) => {
  const firstComponent = setupComponents(setupJson).find((it) => it.name === first) ?? null;
  return (
    <div className="flex min-h-0 w-full flex-1 flex-col items-center">
      <EmptyState icon={setupIcon(firstComponent?.known ?? null, firstComponent?.className ?? '')} title={`${sentenceCase(first)} first`} text={prerequisiteText(first, wanted)} />
      <Button
        onClick={() => {
          const found = setupComponents(setupJson).find((it) => it.name === first);
          onOpen(found === undefined ? null : [found, null]);
        }}
      >
        {`Set up ${sentenceCase(first)}`}
      </Button>
    </div>
  );
};

const FILL = 'min-h-0 flex-1';

const ComponentPage = ({
  open,
  section,
  setupJson,
  onBack,
  onOpen,
}: {
  open: SetupComponent;
  section: string | null;
  setupJson: Json | null;
  onBack: () => void;
  onOpen: (opened: Opened | null) => void;
}) => {
  const nativePage = setupPage(setupJson, open.name);
  const blocked = open.blockedReason;
  const first = open.prerequisite;
  const reportsLookups = reportsOpening(first, nativePage);
  useLaunchedEffect([open.name, reportsLookups], async () => {
    if (reportsLookups) await Qgc.invoke(SETUP_PAGE_OPENED, open.name);
  });
  const head = headPage(open);
  const screen = nativePage?.screen ?? null;
  const routes: readonly (readonly [boolean, () => ReactNode])[] = [
    [first !== null, () => (first === null ? null : <Prerequisite first={first} wanted={open.name} setupJson={setupJson} onOpen={onOpen} />)],
    [head === SENSORS, () => <SensorsScreen className={FILL} />],
    [head === RADIO, () => <RadioScreen className={FILL} />],
    [head === REMOTE_SUPPORT, () => <RemoteSupportScreen className={FILL} />],
    [screen === APM_SUB_MOTORS_SCREEN, () => <ApmSubMotorsScreen className={FILL} />],
    [head === MOTORS, () => <MotorsScreen className={FILL} />],
    [head === FLIGHT_MODES_PAGE, () => <FlightModesSetup className={FILL} />],
    [screen === PX4_TUNING_SCREEN, () => <Px4TuningScreen className={FILL} />],
    [screen === PX4_AIRFRAME_SCREEN, () => <Px4AirframeScreen className={FILL} />],
    [screen === ACTUATORS_SCREEN, () => <ActuatorsScreen className={FILL} />],
    [screen === APM_SERVOS_SCREEN, () => <ApmServosScreen className={FILL} />],
    [screen === APM_FOLLOW_SCREEN, () => <ApmFollowScreen className={FILL} />],
    [screen === SCRIPTING_SCREEN, () => <ScriptingScreen className={FILL} />],
    [screen === JOYSTICK_SCREEN, () => <JoystickScreen className={FILL} />],
    [screen === ESP_BRIDGE_SCREEN, () => <EspBridgeScreen className={FILL} />],
    [screen === SYSLINK_SCREEN, () => <SyslinkScreen className={FILL} />],
    [screen === APM_SUB_FRAME_SCREEN, () => <ApmSubFrameScreen className={FILL} />],
    [screen === APM_AIRFRAME_SCREEN, () => <ApmAirframeScreen className={FILL} />],
    [screen === OPTICAL_FLOW_SCREEN, () => <OpticalFlowScreen className={FILL} />],
    [screen === NOT_SUPPORTED_SCREEN, () => <SetupNotice text="Not supported" className={FILL} />],
    [
      nativePage?.parameterSections === true,
      () => (
        <>
          {open.known === 'power' ? <PowerLiveCard /> : null}
          <ParameterForm page={open.name} className={FILL} section={section} />
        </>
      ),
    ],
  ];
  const body = routes.find(([matches]) => matches)?.[1]() ?? <SetupNotice text={`${open.name} is set up on the desktop.`} className={FILL} />;
  return (
    <>
      <SetupPageBar title={sentenceCase(open.name)} onBack={onBack} />
      {blocked === null ? (
        <div className="flex min-h-0 w-full flex-1 flex-col">{body}</div>
      ) : (
        <>
          <Text fontWeight="bold" color={aircast('warning')} className="block w-full px-4 py-2">
            {disabledWhile(blocked)}
          </Text>
          <div className="relative min-h-0 w-full flex-1">
            <div inert className="flex h-full w-full flex-col">
              {body}
            </div>
            <div {...swallowTouches('absolute inset-0')} style={{ background: withAlpha(Color.Black, 0.5) }} />
          </div>
        </>
      )}
    </>
  );
};

const ComponentRow = ({
  component,
  status,
  state,
  onClick,
  selected,
  summary,
  hits,
  onOpenSection,
}: {
  component: SetupComponent;
  status: string;
  state: SetupState;
  onClick: (() => void) | null;
  selected: boolean;
  summary: readonly SummaryLine[];
  hits: readonly string[];
  onOpenSection: (section: string) => void;
}) => (
  <div className="flex flex-col">
    <SetupRow
      title={sentenceCase(component.name)}
      status={status}
      state={state}
      onClick={onClick}
      selected={selected}
      summary={summary}
      icon={setupIcon(component.known, component.className)}
      subtitle={summary.length === 0 ? setupNote(component.className) : ''}
    />
    <SectionHits titles={hits} onOpen={onOpenSection} />
  </div>
);

const SetupOverview = ({
  className,
  style,
  setup,
  vehicleType,
  firmware,
  setupJson,
  components,
  needSetup,
  hitsOf,
  searchHit,
  setupSearch,
  onSetupSearch,
  onSearchParameters,
  advanced,
  openComponent,
  onOpenFromList,
  summaries,
  onSummaries,
  parametersOpen,
  onOpenParameters,
}: {
  className: string;
  style?: CSSProperties;
  setup: SetupReadiness | null;
  vehicleType: string;
  firmware: string;
  setupJson: Json | null;
  components: readonly SetupComponent[];
  needSetup: readonly SetupComponent[];
  hitsOf: (name: string) => readonly string[];
  searchHit: (component: SetupComponent) => boolean;
  setupSearch: string;
  onSetupSearch: (search: string) => void;
  onSearchParameters: () => void;
  advanced: boolean;
  openComponent: SetupComponent | null;
  onOpenFromList: (component: SetupComponent, section: string | null) => void;
  summaries: ReadonlyMap<string, readonly SummaryLine[]>;
  onSummaries: (summaries: ReadonlyMap<string, readonly SummaryLine[]>) => void;
  parametersOpen: boolean;
  onOpenParameters: () => void;
}) => {
  useLaunchedEffect([], async (scope) => {
    await Qgc.invoke(SETUP_PAGE_OPENED, SETUP_SUMMARY_PAGE);
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const read = setupSummaries(await Qgc.get(SETUP_SUMMARY));
      if (scope.isActive()) onSummaries(read);
      await delay(SETUP_SUMMARY_POLL_MS);
      return poll();
    };
    await poll();
  });
  const parametersAreReady = useMemo(() => parametersReady(setupJson), [setupJson]);
  const parameterCount = useProduceState(0, [parametersAreReady], async (set) => set(parametersAreReady ? (await parameterNames()).length : 0));

  const summaryOf = (name: string): readonly SummaryLine[] => summaries.get(name) ?? [];
  const remaining = remainingSetup(components).filter(searchHit);
  const [usable, desktopOnly] = splitDesktopOnly(remaining, (it) => headCanOpen(setupPage(setupJson, it.name), headPage(it)));
  const incomplete = parametersIncomplete(setupJson);

  const readyRow = (component: SetupComponent) => {
    const openable = headCanOpen(setupPage(setupJson, component.name), headPage(component));
    const blocked = component.blockedReason;
    return (
      <ComponentRow
        key={component.index}
        component={component}
        status={blocked !== null ? `Not while ${blocked}` : component.needsAttention ? attentionAction(component.className) : !openable ? 'On desktop' : ''}
        state={blocked !== null ? SetupState.Unavailable : component.needsAttention ? SetupState.NeedsAttention : !openable ? SetupState.Unavailable : SetupState.Neutral}
        onClick={openable ? () => onOpenFromList(component, null) : null}
        selected={same(component, openComponent)}
        summary={summaryOf(component.name)}
        hits={hitsOf(component.name)}
        onOpenSection={(it) => onOpenFromList(component, it)}
      />
    );
  };

  return (
    <div className={`overflow-y-auto ${className}`} style={style}>
      <ReadinessHeader
        vehicle={vehicleType.trim() === '' ? 'Vehicle' : vehicleType}
        firmware={firmware}
        vehicleId={setup?.vehicleId ?? null}
        note={readinessNote(setup, components.some((it) => it.needsAttention))}
      />

      <SearchPill value={setupSearch} onValueChange={onSetupSearch} placeholder="Search" keyboardOptions={{ imeAction: 'Search' }} keyboardActions={{ onSearch: onSearchParameters }} />

      {needSetup.length > 0 ? (
        <>
          <SectionHeader text="Needs attention" />
          {needSetup.map((component) => {
            const blocked = component.blockedReason;
            const page = setupPage(setupJson, component.name);
            return (
              <ComponentRow
                key={`a${component.index}`}
                component={component}
                status={blocked !== null ? `Not while ${blocked}` : attentionAction(component.className)}
                state={blocked !== null ? SetupState.Unavailable : SetupState.NeedsAttention}
                onClick={headCanOpen(page, component.name) ? () => onOpenFromList(component, null) : null}
                selected={same(component, openComponent)}
                summary={summaryOf(component.name)}
                hits={hitsOf(component.name)}
                onOpenSection={(it) => onOpenFromList(component, it)}
              />
            );
          })}
        </>
      ) : null}

      {components.length === 0 ? (
        incomplete !== null ? (
          <SetupNotice text={incomplete} />
        ) : (
          <EmptyState icon="ic_build" title={NOTHING_TO_CONFIGURE} text={NOTHING_TO_CONFIGURE_TEXT} />
        )
      ) : remaining.length > 0 ? (
        <>
          {usable.length > 0 ? <SectionHeader text="Ready" /> : null}
          {usable.map(readyRow)}
          {desktopOnly.length > 0 ? (
            <>
              <SectionHeader text="Advanced" />
              {desktopOnly.map(readyRow)}
            </>
          ) : null}
        </>
      ) : null}

      {advanced && setupMatches('Parameters', setupSearch) ? (
        <>
          {desktopOnly.length === 0 ? <SectionHeader text="Advanced" /> : null}
          <SetupRow
            title="Parameters"
            subtitle={parameterCountText(parameterCount) ?? 'Every setting the vehicle has'}
            state={SetupState.Neutral}
            icon="ic_tune"
            onClick={onOpenParameters}
            selected={parametersOpen}
          />
        </>
      ) : null}
    </div>
  );
};

const SetupPageBar = ({ title, onBack }: { title: string; onBack: () => void }) => {
  const heading = useContext(LocalPageHeading);
  const twoPane = useContext(LocalTwoPane);
  return heading !== null && !twoPane ? <OverridePageHeading title={title} onBack={onBack} /> : <PageTopBar title={title} backLabel={`Back to ${AIRCRAFT_SETUP}`} onBack={onBack} />;
};

const ReadinessHeader = ({ vehicle, firmware, vehicleId, note }: { vehicle: string; firmware: string; vehicleId: number | null; note: string | null }) => {
  const heading = useContext(LocalPageHeading);
  return (
    <div className="flex w-full flex-col px-4 pt-5 pb-2">
      {heading === null ? <Text typography="headlineMedium">{AIRCRAFT_SETUP}</Text> : null}
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {setupSubtitle(vehicle, firmware)}
      </Text>
      {vehicleId !== null ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {`Vehicle ${vehicleId}`}
        </Text>
      ) : null}
      {note !== null ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {note}
        </Text>
      ) : null}
    </div>
  );
};
