import { type CSSProperties, Fragment, type ReactNode, createContext, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { type Fact, type FactSlider, Qgc } from '@/bridge/Qgc';
import { settingControl, useQgcPath } from '@/bridge/QgcState';
import { type Json, jsonObjects, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';
import { useBackHandler, useKeyedState, useLaunchedEffect, useProduceState, useUpdatedRef } from '@/kotlin/Compose';
import type { MutableState } from '@/kotlin/State';
import { coerceIn, delay, format } from '@/kotlin/Std';
import { sentenceCase as mapSentenceCase } from '@/map/SentenceCase';
import { AlertDialog, Checkbox, DropdownMenu, DropdownMenuItem, HorizontalDivider, IconButton, OutlinedTextField, SegmentedButton, SingleChoiceSegmentedButtonRow, Slider, Switch, Text, TextButton, stepsFor } from '@/m3';
import type { KeyboardType } from '@/m3';
import { Drawable, MaterialIcon } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { aircast, md, withAlpha } from '@/theme/Colors';
import { AboutPage, qgcVersion } from '@/ui/AboutPage';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { CAMERAS_VIEW, VIDEO_SOURCES_PAGE, camerasGlance, camerasReading } from '@/ui/Cameras';
import { CamerasEditor } from '@/ui/CamerasEditor';
import { FLASHER_PAGE, FlasherScreen } from '@/ui/FlasherScreen';
import { useHasVehicle } from '@/ui/FlyState';
import { GCS_LOCATION_BLOCK, GcsPositionStatus, REMOTE_ID_GROUP } from '@/ui/GcsPositionStatus';
import { LinkStatusSection } from '@/ui/LinkStatusSection';
import { LinksScreen } from '@/ui/LinksScreen';
import { MAVLINK_ACTIONS_GROUP, MavlinkActionsSection } from '@/ui/MavlinkActions';
import { NtripMountpointBrowser, NtripStatusSection } from '@/ui/NtripStatusSection';
import { OFFLINE_MAPS_GROUP, OfflineMapsSection } from '@/ui/OfflineMaps';
import { OsmFilePicker } from '@/ui/OsmFilePicker';
import { PacketRadioSection } from '@/ui/PacketRadioSection';
import { paletteNamed } from '@/ui/Palette';
import { bitmaskRaw, bitmaskSummary, controlIsUnderstood, factFromControl, sliderSnapped, sliderValue } from '@/ui/ParameterForm';
import { ValueDetailsSheet } from '@/ui/ParameterLinks';
import { Px4LogTransferPage } from '@/ui/Px4LogTransfer';
import { RcControlsEditor } from '@/ui/RcControlsEditor';
import { FLIGHT_MODES_PAGE, SAFETY_SETUP_PAGE, SENSORS } from '@/ui/SetupPages';
import { AdvancedToggle, FootNote, LocalPageHeading, type PageHeading, SectionHeader, SetupRow } from '@/ui/SetupRow';
import { SETUP_PARAMETERS_PAGE, type SetupComponent, setupComponents, setupIcon, setupMatches } from '@/ui/SetupScreen';
import { SETUP } from '@/ui/SetupView';
import { MAVLINK_GROUP, SIGNING_AFTER_BLOCK, SigningKeysSection } from '@/ui/SigningKeysSection';
import { LocalChangeNotice, SafeChoiceMenu, SliderValueRow, inlineSlider, opensAsValue, valueText } from '@/ui/ValueRow';
import { AppLogPage } from '@/ui/AppLogPage';
import { PilotSettings, SensorChecks, pilotSearchHits, pilotWorded } from '@/ui/PilotSettings';
import { GENERAL_PAGE, ResetAllSettingsRow } from '@/ui/ResetAllSettings';
import { UnitsSection, unitSystemLabel } from '@/ui/UnitsSection';

const SETTINGS_VIEW = 'view.settings';
const SEARCH_SETTLE_MS = 250;

export const UNITS_GROUP = 'unitsSettings';
export const FLY_VIEW_GROUP = 'flyViewSettings';
export const VIEWER_3D_GROUP = 'viewer3DSettings';

export const GROUPS_WITH_A_HEAD_EDITOR: ReadonlySet<string> = new Set([FLY_VIEW_GROUP]);

export const PAGES_WITHOUT_A_SCREEN: ReadonlyMap<string, string> = new Map([
  ['Firmware Upgrade', 'QGC has no such settings page; the firmware screen reads these settings itself'],
  ['Flight Modes', 'twelve comma-separated lists of hidden mode names, one per airframe. The mode ' + 'picker reads what they produce; the raw lists are worse than nothing'],
]);

export const SECTIONS_DRAWN_BY_THE_HEAD: ReadonlySet<string> = new Set([MAVLINK_ACTIONS_GROUP]);

export const CONNECTIONS_PAGE = 'Connections';

const PAGE_GLANCES: ReadonlyMap<string, readonly string[]> = new Map([['Maps', ['settings.flightMapSettings.mapProvider', 'settings.flightMapSettings.mapType']]]);

const notBlank = (text: string): boolean => text.trim() !== '';

const useServed = (paths: readonly string[], watching: boolean): readonly (Json | null)[] => {
  useEffect(() => {
    if (!watching) return undefined;
    Qgc.watch([...paths]);
    return () => Qgc.unwatch([...paths]);
  }, [paths, watching]);
  const [served, setServed] = useState<readonly (Json | null)[]>(() => paths.map((path) => Qgc.path(path)));
  useEffect(() => {
    const read = () => setServed(paths.map((path) => Qgc.path(path)));
    read();
    const releases = paths.map((path) => Qgc.subscribe(path, read));
    return () => {
      releases.map((release) => release());
    };
  }, [paths]);
  return served;
};

export const glanceText = (displays: readonly string[]): string => displays.filter(notBlank).map(sentenceCase).join(' · ');

const glancePaths = (title: string): readonly string[] => {
  if (title === 'About') return [];
  if (title === GENERAL_PAGE) return ['settings.unitsSettings.unitSystem'];
  if (title === VIDEO_SOURCES_PAGE) return [CAMERAS_VIEW];
  return (PAGE_GLANCES.get(title) ?? []).map(settingControl);
};

const usePageGlance = (title: string): string => {
  const paths = useMemo(() => glancePaths(title), [title]);
  const served = useServed(paths, true);
  if (title === 'About') return `Aircast ${qgcVersion()}`;
  if (title === GENERAL_PAGE) {
    const system = optDouble(served[0] ?? null, 'value');
    return Number.isNaN(system) ? '' : unitSystemLabel(Math.trunc(system));
  }
  if (title === VIDEO_SOURCES_PAGE) return camerasGlance(camerasReading(served[0] ?? null));
  return PAGE_GLANCES.has(title) ? glanceText(served.map((json) => optText(json, 'display'))) : '';
};

const PAGE_TITLES: ReadonlyMap<string, string> = new Map([
  [CONNECTIONS_PAGE, 'Links'],
  ['ADSB Server', 'ADS-B server'],
]);

export const pageTitle = (title: string): string => PAGE_TITLES.get(title) ?? sentenceCase(title);

const linksOf = (view: Json | null): Json[] => jsonObjects(optJSONArray(view, 'links'));

export const activeLinkCount = (view: Json | null): number => linksOf(view).filter((link) => optBoolean(link, 'connected')).length;

export const activeLinksGlance = (view: Json | null): string => [
  ...new Set(
    linksOf(view)
      .filter((link) => optBoolean(link, 'connected'))
      .map((link) => (notBlank(optText(link, 'summary')) ? optText(link, 'summary') : optText(link, 'name')))
      .filter(notBlank),
  ),
].join(' · ');

export const activeLinksText = (count: number): string => (count > 0 ? `${count} active` : '');

export type SettingsGroup = { readonly name: 'Safety' | 'Control' | 'Camera' | 'Transmission' | 'General'; readonly ordinal: number; readonly title: string };
export const SettingsGroup = {
  Safety: { name: 'Safety', ordinal: 0, title: 'Safety' },
  Control: { name: 'Control', ordinal: 1, title: 'Control' },
  Camera: { name: 'Camera', ordinal: 2, title: 'Camera' },
  Transmission: { name: 'Transmission', ordinal: 3, title: 'Transmission' },
  General: { name: 'General', ordinal: 4, title: 'General' },
} as const satisfies Record<string, SettingsGroup>;
export const SettingsGroupEntries: readonly SettingsGroup[] = [SettingsGroup.Safety, SettingsGroup.Control, SettingsGroup.Camera, SettingsGroup.Transmission, SettingsGroup.General];

export type PageLook = { group: SettingsGroup; icon: DrawableName; inline: boolean; tabHome: boolean };
export const PageLook = (fields: { group: SettingsGroup; icon: DrawableName; inline?: boolean; tabHome?: boolean }): PageLook => ({ inline: false, tabHome: false, ...fields });

export const PAGE_LOOKS: ReadonlyMap<string, PageLook> = new Map<string, PageLook>([
  ['ADSB Server', PageLook({ group: SettingsGroup.Safety, icon: 'ic_navigation', inline: true })],
  ['Remote ID', PageLook({ group: SettingsGroup.Safety, icon: 'ic_shield' })],
  ['Fly View', PageLook({ group: SettingsGroup.Control, icon: 'ic_flight', inline: true })],
  ['Flight Modes', PageLook({ group: SettingsGroup.Control, icon: 'ic_toggle_on' })],
  ['Plan View', PageLook({ group: SettingsGroup.Control, icon: 'ic_route', inline: true })],
  ['Maps', PageLook({ group: SettingsGroup.Control, icon: 'ic_map' })],
  ['3D Viewer', PageLook({ group: SettingsGroup.Control, icon: 'ic_explore' })],
  ['Video', PageLook({ group: SettingsGroup.Camera, icon: 'ic_videocam', inline: true })],
  ['Connections', PageLook({ group: SettingsGroup.Transmission, icon: 'ic_link' })],
  [VIDEO_SOURCES_PAGE, PageLook({ group: SettingsGroup.Transmission, icon: 'ic_videocam' })],
  ['MAVLink', PageLook({ group: SettingsGroup.Transmission, icon: 'ic_swap_horiz' })],
  ['Packet Radio', PageLook({ group: SettingsGroup.Transmission, icon: 'ic_wifi' })],
  ['RTK GPS', PageLook({ group: SettingsGroup.Transmission, icon: 'ic_satellite_alt' })],
  ['NTRIP / RTK', PageLook({ group: SettingsGroup.Transmission, icon: 'ic_satellite_alt' })],
  ['General', PageLook({ group: SettingsGroup.General, icon: 'ic_tune', inline: true, tabHome: true })],
  ['PX4 Log Transfer', PageLook({ group: SettingsGroup.General, icon: 'ic_download' })],
  ['Firmware Upgrade', PageLook({ group: SettingsGroup.General, icon: 'ic_developer_board' })],
  [FLASHER_PAGE, PageLook({ group: SettingsGroup.General, icon: 'ic_usb' })],
  ['App Logging', PageLook({ group: SettingsGroup.General, icon: 'ic_description' })],
  ['Console', PageLook({ group: SettingsGroup.General, icon: 'ic_terminal' })],
  ['About', PageLook({ group: SettingsGroup.General, icon: 'ic_help' })],
]);

export const BLOCK_HOMES: ReadonlyArray<readonly [readonly [string, string], SettingsGroup]> = [[['Fly View', 'Guided Commands'], SettingsGroup.Safety]];

export const pageLook = (title: string): PageLook => PAGE_LOOKS.get(title) ?? PageLook({ group: SettingsGroup.General, icon: 'ic_settings' });

export const blockGroup = (page: string, block: SettingsBlock): SettingsGroup =>
  BLOCK_HOMES.find(([[home, title]]) => home === page && title === block.title)?.[1] ?? pageLook(page).group;

export const tabPages = (group: SettingsGroup, pages: readonly SettingsPageEntry[]): readonly SettingsPageEntry[] => {
  const lends = BLOCK_HOMES.filter(([, home]) => home === group).map(([[page]]) => page);
  const order = [...PAGE_LOOKS.keys()];
  const rank = (title: string) => (order.indexOf(title) >= 0 ? order.indexOf(title) : Number.MAX_SAFE_INTEGER);
  const own = pages.filter((page) => pageLook(page.title).group === group).toSorted((a, b) => rank(a.title) - rank(b.title));
  return [...pages.filter((page) => lends.includes(page.title) && pageLook(page.title).group !== group), ...own];
};

export const sectionsIn = (group: SettingsGroup, page: string, sections: readonly SettingsSectionRows[]): readonly SettingsSectionRows[] =>
  sections.map((section) => ({ ...section, blocks: section.blocks.filter((block) => blockGroup(page, block) === group) })).filter((section) => section.blocks.length > 0);

export type HelpLink = { name: string; url: string; host: string };

export type SettingsPageEntry = {
  title: string;
  showsLinks: boolean;
  showsVideoSources: boolean;
  sectionCount: number;
  showsAbout: boolean;
  showsConsole: boolean;
  showsNtrip: boolean;
  showsPx4Logs: boolean;
  showsPacketRadio: boolean;
  showsFlasher: boolean;
  helpLinks: readonly HelpLink[];
  keywords: string;
};
export const SettingsPageEntry = (
  fields: Pick<SettingsPageEntry, 'title' | 'showsLinks' | 'showsVideoSources' | 'sectionCount'> & Partial<SettingsPageEntry>,
): SettingsPageEntry => ({
  showsAbout: false,
  showsConsole: false,
  showsNtrip: false,
  showsPx4Logs: false,
  showsPacketRadio: false,
  showsFlasher: false,
  helpLinks: [],
  keywords: '',
  ...fields,
});

export type SettingsBlock = { title: string; facts: readonly Fact[] };

export type SettingsSectionRows = { title: string; group: string; note: string; blocks: readonly SettingsBlock[] };

export const settingsPagePath = (title: string): string => `${SETTINGS_VIEW}(${title})`;

export const useRememberSettingsPages = (): readonly SettingsPageEntry[] =>
  useProduceState<readonly SettingsPageEntry[]>([], [], async (set) => set([...settingsPages(await Qgc.get(SETTINGS_VIEW)), FLASHER_SETTINGS_PAGE]));

export const FLASHER_SETTINGS_PAGE = SettingsPageEntry({ title: FLASHER_PAGE, showsLinks: false, showsVideoSources: false, sectionCount: 0, showsFlasher: true, keywords: 'sd card, image, flash, aircast os, wifi, hostname, ssh, tailscale' });

export const editOnDesktop = (fact: Fact): boolean => !controlIsUnderstood(fact.controlKind);

export const inertNote = (fact: Fact): string => (!fact.enabled ? (notBlank(fact.disabledReason) ? fact.disabledReason : 'Has no effect yet') : 'Read-only');

export const settingsPages = (view: Json | null): readonly SettingsPageEntry[] =>
  jsonObjects(optJSONArray(view, 'pages'))
    .map((page) =>
      SettingsPageEntry({
        title: optText(page, 'title'),
        showsLinks: optBoolean(page, 'showsLinks'),
        showsVideoSources: optBoolean(page, 'showsVideoSources'),
        sectionCount: optJSONArray(page, 'sections')?.length ?? 0,
        showsAbout: optBoolean(page, 'showsAbout'),
        showsConsole: optBoolean(page, 'showsConsole'),
        showsNtrip: optBoolean(page, 'showsNtrip'),
        showsPx4Logs: optBoolean(page, 'showsPx4Logs'),
        showsPacketRadio: optBoolean(page, 'showsPacketRadio'),
        keywords: optText(page, 'keywords'),
        helpLinks: jsonObjects(optJSONArray(page, 'helpLinks')).map((link) => ({ name: optText(link, 'name'), url: optText(link, 'url'), host: optText(link, 'host') })),
      }),
    )
    .filter(
      (it) =>
        notBlank(it.title) &&
        !PAGES_WITHOUT_A_SCREEN.has(it.title) &&
        (it.sectionCount > 0 || it.showsLinks || it.showsAbout || it.showsConsole || it.showsPx4Logs || it.showsVideoSources),
    );

const blockFacts = (block: Json): readonly Fact[] =>
  jsonObjects(optJSONArray(block, 'controls')).flatMap((control) => {
    const fact = factFromControl(control);
    return fact === null ? [] : [pilotWorded(paletteNamed(fact))];
  });

export const settingsSections = (page: Json | null): readonly SettingsSectionRows[] =>
  jsonObjects(optJSONArray(page, 'sections'))
    .map((section) => ({
      title: optText(section, 'title'),
      group: optText(section, 'group'),
      note: optText(section, 'note'),
      blocks: jsonObjects(optJSONArray(section, 'subsections'))
        .map((block) => ({ title: optText(block, 'title'), facts: blockFacts(block) }))
        .filter((block) => block.facts.length > 0),
    }))
    .map((section) => (SECTIONS_DRAWN_BY_THE_HEAD.has(section.group) ? { ...section, blocks: [{ title: '', facts: [] }] } : section))
    .filter((section) => section.blocks.length > 0);

export const blockHeading = (pageTitle: string, section: SettingsSectionRows, block: SettingsBlock): string =>
  notBlank(block.title) ? block.title : section.title !== pageTitle ? section.title : '';

export const shownBreadcrumb = (title: string): string => title.split(' › ').map(sentenceCase).join(' › ');

export const pageMatches = (page: SettingsPageEntry, needle: string): boolean => {
  const wanted = needle.trim().toLowerCase();
  return wanted !== '' && (page.title.toLowerCase().includes(wanted) || page.keywords.includes(wanted));
};

export const matchesIn = (pageTitle: string, sections: readonly SettingsSectionRows[], needle: string): readonly SettingsSectionRows[] => {
  const wanted = needle.trim().toLowerCase();
  if (wanted === '') return [];
  return sections
    .filter((section) => section.group !== UNITS_GROUP)
    .flatMap((section) => {
      const hits = section.blocks
        .flatMap((block) => block.facts)
        .filter((fact) => fact.title.toLowerCase().includes(wanted) || fact.name.toLowerCase().includes(wanted) || fact.keywords.includes(wanted));
      if (hits.length === 0) return [];
      const title = notBlank(section.title) && section.title.toLowerCase() !== pageTitle.toLowerCase() ? `${pageTitle} › ${section.title}` : pageTitle;
      return [{ ...section, title, note: '', blocks: [{ title: '', facts: hits }] }];
    });
};

export const AIRCRAFT_SETUP = 'Aircraft setup';

export const TAB_SETUP_TITLES: ReadonlyMap<string, string> = new Map([[SAFETY_SETUP_PAGE, 'More failsafe settings']]);

export const tabSetupPages = (group: SettingsGroup): readonly string[] =>
  group === SettingsGroup.Safety ? [SAFETY_SETUP_PAGE] : group === SettingsGroup.Control ? [FLIGHT_MODES_PAGE] : [];

export const tabSetupComponents = (group: SettingsGroup, components: readonly SetupComponent[]): readonly SetupComponent[] =>
  tabSetupPages(group).flatMap((name) => {
    const found = components.find((component) => component.name === name);
    return found === undefined ? [] : [found];
  });

export const setupSearchHits = (components: readonly SetupComponent[], query: string): readonly SetupComponent[] =>
  query.trim() === '' ? [] : components.filter((component) => setupMatches(component.name, query));

const TabSetupRows = ({ group, onOpenSetup }: { group: SettingsGroup; onOpenSetup: (component: string | null) => void }) => {
  const setupJson = useQgcPath(SETUP);
  const components = useMemo(() => setupComponents(setupJson), [setupJson]);
  return (
    <>
      {tabSetupComponents(group, components).map((component) => (
        <SetupRow
          key={component.name}
          title={TAB_SETUP_TITLES.get(component.name) ?? sentenceCase(component.name)}
          status=""
          icon={setupIcon(component.known, component.className)}
          onClick={() => onOpenSetup(component.name)}
        />
      ))}
    </>
  );
};

const AircraftSetupRow = ({ onOpenSetup }: { onOpenSetup: (component: string | null) => void }) => (
  <SetupRow title={AIRCRAFT_SETUP} status="" icon="ic_build" onClick={() => onOpenSetup(null)} />
);

export const SettingsScreen = ({
  group,
  everyPage,
  open,
  onOpen,
  onClose,
  heading,
  className,
  onOpenSetup = () => undefined,
}: {
  group: SettingsGroup;
  everyPage: readonly SettingsPageEntry[];
  open: string | null;
  onOpen: (title: string) => void;
  onClose: () => void;
  heading: MutableState<PageHeading | null>;
  className?: string;
  onOpenSetup?: (component: string | null) => void;
}) => {
  useBackHandler(open !== null, onClose);
  const current = everyPage.find((it) => it.title === open) ?? null;
  return (
    <LocalPageHeading.Provider value={heading}>
      <LocalSettingsList.Provider value>
        <div className={`flex h-full w-full justify-center ${className ?? ''}`}>
          <div className="h-full w-full" style={{ maxWidth: DETAIL_PANE_MAX_WIDTH }}>
            {current === null ? (
              <SettingsTab group={group} everyPage={everyPage} className="h-full w-full" onOpenSetup={onOpenSetup} onOpen={onOpen} />
            ) : (
              <SettingsPageBody page={current} className="h-full w-full" />
            )}
          </div>
        </div>
      </LocalSettingsList.Provider>
    </LocalPageHeading.Provider>
  );
};

export const LIST_DETAIL_MIN_WIDTH = 840;
export const LIST_PANE_WIDTH = 380;
export const DETAIL_PANE_MAX_WIDTH = 720;

const PageEntryRow = ({ page, linksJson, onOpen }: { page: SettingsPageEntry; linksJson: Json | null; onOpen: (title: string) => void }) => {
  const glance = usePageGlance(page.title);
  return (
    <SetupRow
      title={pageTitle(page.title)}
      status={page.title === CONNECTIONS_PAGE ? activeLinksGlance(linksJson) : glance}
      icon={pageLook(page.title).icon}
      onClick={() => onOpen(page.title)}
    />
  );
};

const SettingsTab = ({
  group,
  everyPage,
  className,
  onOpenSetup,
  onOpen,
}: {
  group: SettingsGroup;
  everyPage: readonly SettingsPageEntry[];
  className: string;
  onOpenSetup: (component: string | null) => void;
  onOpen: (title: string) => void;
}) => {
  const linksJson = useQgcPath('view.links');
  const pages = useMemo(() => tabPages(group, everyPage).filter((it) => !it.showsFlasher), [group, everyPage]);
  const [advancedOpen, setAdvancedOpen] = useKeyedState([group], () => false);
  const foldsInline = group === SettingsGroup.Safety;
  const drawnInline = (page: SettingsPageEntry): boolean => pageLook(page.title).inline || pageLook(page.title).group !== group;
  const pageEntry = (page: SettingsPageEntry): ReactNode =>
    drawnInline(page) ? <InlinePage key={page.title} page={page} group={group} /> : <PageEntryRow key={page.title} page={page} linksJson={linksJson} onOpen={onOpen} />;
  const folded = pages.filter(drawnInline);
  const shown = pages.filter((page) => !drawnInline(page));
  return (
    <div className={`overflow-y-auto ${className}`}>
      {group === SettingsGroup.General ? (
        <>
          <AircraftSetupRow onOpenSetup={onOpenSetup} />
          <SetupRow title={FLASHER_PAGE} status="Set up an SD card for a new drone" icon="ic_usb" onClick={() => onOpen(FLASHER_PAGE)} />
        </>
      ) : null}
      <PilotSettings group={group} />
      {group === SettingsGroup.Safety ? <SensorChecks onCalibrate={() => onOpenSetup(SENSORS)} /> : null}
      {foldsInline ? (
        <>
          {shown.map(pageEntry)}
          <TabSetupRows group={group} onOpenSetup={onOpenSetup} />
          {folded.length > 0 ? <AdvancedToggle open={advancedOpen} label={ADVANCED_SAFETY} onToggle={() => setAdvancedOpen(!advancedOpen)} /> : null}
          {advancedOpen ? folded.map(pageEntry) : null}
        </>
      ) : (
        <>
          {pages.map(pageEntry)}
          <TabSetupRows group={group} onOpenSetup={onOpenSetup} />
        </>
      )}
    </div>
  );
};

const InlinePage = ({ page, group }: { page: SettingsPageEntry; group: SettingsGroup }) => {
  const [sections, setSections] = useKeyedState<readonly SettingsSectionRows[] | null>([page.title], () => null);
  const [reloads, setReloads] = useKeyedState([page.title], () => 0);
  useLaunchedEffect([page.title, reloads], async (scope) => {
    const read = settingsSections(await Qgc.get(settingsPagePath(page.title)));
    if (scope.isActive()) setSections(read);
  });
  const shown = sections === null ? [] : sectionsIn(group, page.title, sections);
  if (shown.length === 0) return null;
  const look = pageLook(page.title);
  const home = look.group === group;
  return (
    <>
      {!(home && look.tabHome) ? <PageHeader icon={look.icon} title={home ? pageTitle(page.title) : borrowedTitle(shown)} /> : null}
      <SettingsControls page={page} sections={shown} home={home} blockHeadings={home} onWrite={() => setReloads((it) => it + 1)} />
      {home && page.title === GENERAL_PAGE ? <ResetAllSettingsRow /> : null}
    </>
  );
};

export const borrowedTitle = (sections: readonly SettingsSectionRows[]): string => [
  ...new Set(
    sections
      .flatMap((section) => section.blocks)
      .map((block) => sentenceCase(block.title))
      .filter(notBlank),
  ),
].join(' · ');

const PageHeader = ({ icon, title }: { icon: DrawableName; title: string }) => (
  <div className="flex w-full items-center gap-3 px-4 pt-7 pb-1">
    <Drawable name={icon} size={24} style={{ color: md('primary') }} />
    <Text typography="titleMedium">{title}</Text>
  </div>
);

export const SettingsSearch = ({
  query,
  className,
  onOpenSetup = () => undefined,
  onOpen,
}: {
  query: string;
  className?: string;
  onOpenSetup?: (component: string | null) => void;
  onOpen: (title: string) => void;
}) => {
  const setupJson = useQgcPath(SETUP);
  const setupHits = useMemo(() => setupSearchHits(setupComponents(setupJson), query), [setupJson, query]);
  const pilotHits = useMemo(() => pilotSearchHits(query), [query]);
  const navigation = useContext(LocalAppNavigation);
  const vehicle = useHasVehicle();
  const advanced = useAdvancedUiShown();
  const searchesParameters = vehicle && advanced;
  const parameterSearch = () => {
    navigation.parametersSearch.value = query.trim();
    onOpenSetup(SETUP_PARAMETERS_PAGE);
  };
  const [pages, setPages] = useState<readonly SettingsPageEntry[]>([]);
  useLaunchedEffect([], async (scope) => {
    const read = settingsPages(await Qgc.get(SETTINGS_VIEW));
    if (scope.isActive()) setPages(read);
  });
  const [hits, setHits] = useState<readonly SettingsSectionRows[]>([]);
  const searching = notBlank(query);
  const pagePaths = useMemo(() => pages.map((page) => settingsPagePath(page.title)), [pages]);
  const served = useServed(pagePaths, searching);

  useLaunchedEffect([query, pages, served], async (scope) => {
    if (!searching) {
      setHits([]);
      return;
    }
    await delay(SEARCH_SETTLE_MS);
    if (!scope.isActive()) return;
    const zipped = pages.flatMap((page, at) => (at < served.length ? [{ page, json: served[at] ?? null }] : []));
    const found = await Promise.all(zipped.map(async ({ page, json }) => matchesIn(page.title, settingsSections(json ?? (await Qgc.get(settingsPagePath(page.title)))), query)));
    if (scope.isActive()) setHits(found.flat());
  });

  const pageHits = pages.filter((page) => pageMatches(page, query));
  const nothing = hits.length === 0 && pageHits.length === 0 && setupHits.length === 0 && pilotHits.length === 0;

  return (
    <div className={`flex h-full w-full justify-center ${className ?? ''}`}>
      <div className="h-full w-full overflow-y-auto" style={{ maxWidth: DETAIL_PANE_MAX_WIDTH }}>
        {!searching ? null : nothing ? (
          <>
            <FootNote text={`No settings match “${query.trim()}”.`} />
            {searchesParameters ? <ParameterSearchRow query={query} onClick={parameterSearch} /> : null}
          </>
        ) : (
          <>
            {pilotHits.length > 0 ? (
              <div style={{ minHeight: TOP_HIT_ANCHOR }}>
                <PilotSettings settings={pilotHits} />
              </div>
            ) : null}
            {setupHits.length > 0 ? <SectionHeader text={AIRCRAFT_SETUP} /> : null}
            {setupHits.map((component) => (
              <SetupRow key={`setup${component.name}`} title={sentenceCase(component.name)} icon={setupIcon(component.known, component.className)} onClick={() => onOpenSetup(component.name)} />
            ))}
            {pageHits.length > 0 ? <SectionHeader text="Pages" /> : null}
            {pageHits.map((entry) => (
              <SetupRow key={`page${entry.title}`} title={pageTitle(entry.title)} icon={pageLook(entry.title).icon} onClick={() => onOpen(entry.title)} />
            ))}
            {hits.map((section) => (
              <Fragment key={`head${section.title}`}>
                <SectionHeader text={shownBreadcrumb(section.title)} />
                {section.blocks.flatMap((block) => block.facts).map((fact) => (
                  <FactRow key={fact.path} fact={fact} />
                ))}
              </Fragment>
            ))}
            {searchesParameters ? <ParameterSearchRow query={query} onClick={parameterSearch} /> : null}
          </>
        )}
      </div>
    </div>
  );
};

const TOP_HIT_ANCHOR = 1;

const ParameterSearchRow = ({ query, onClick }: { query: string; onClick: () => void }) => (
  <SetupRow title={`Search parameters for “${query.trim()}”`} status="" icon="ic_search" onClick={onClick} />
);

const SettingsPageBody = ({ page, className }: { page: SettingsPageEntry; className: string }) => {
  const [sections, setSections] = useKeyedState<readonly SettingsSectionRows[]>([page.title], () => []);
  const [loaded, setLoaded] = useKeyedState([page.title], () => false);
  const [reloads, setReloads] = useKeyedState([page.title], () => 0);
  const reload = () => setReloads((it) => it + 1);

  useLaunchedEffect([page.title, reloads], async (scope) => {
    const read = settingsSections(await Qgc.get(settingsPagePath(page.title)));
    if (!scope.isActive()) return;
    setSections(read);
    setLoaded(true);
  });

  if (page.showsLinks) return <LinksScreen className={className} footer={<SettingsControls page={page} sections={sections} onWrite={reload} />} />;
  if (page.showsAbout) return <AboutPage links={page.helpLinks} className={className} />;
  if (page.showsFlasher) return <FlasherScreen className={className} />;
  if (page.showsConsole) return <AppLogPage className={className} />;
  if (page.showsPx4Logs) return <Px4LogTransferPage className={className} />;
  if (page.showsVideoSources) return <CamerasEditor className={className} />;
  if (!loaded) return <Text className={`block p-4 ${className}`}>Reading settings.</Text>;
  if (sections.length === 0) return <Text className={`block p-4 ${className}`}>No settings exposed here.</Text>;
  return (
    <div className={`overflow-y-auto ${className}`}>
      <SettingsControls page={page} sections={sections} onWrite={reload} />
      {page.title === GENERAL_PAGE ? <ResetAllSettingsRow /> : null}
    </div>
  );
};

const SettingsControls = ({
  page,
  sections,
  home = true,
  blockHeadings = true,
  onWrite,
}: {
  page: SettingsPageEntry;
  sections: readonly SettingsSectionRows[];
  home?: boolean;
  blockHeadings?: boolean;
  onWrite: () => void;
}) => (
  <>
    {home && page.showsNtrip ? <NtripStatusSection onWrite={onWrite} /> : null}
    {home && page.showsPacketRadio ? <PacketRadioSection onWrite={onWrite} /> : null}
    {sections.map((section, at) =>
      section.group === UNITS_GROUP ? (
        <Fragment key={`${section.group}#${at}`}>
          <SectionHeader text={section.title} />
          <UnitsSection />
        </Fragment>
      ) : (
        <Fragment key={`${section.group}#${at}`}>
          {section.blocks.map((block, index) => {
            if (block.title === ADVANCED_BLOCK) {
              return (
                <AdvancedBlock key={index} blockKey={`${section.group}#${block.title}`}>
                  <FactRuns facts={block.facts} onWrite={onWrite} />
                </AdvancedBlock>
              );
            }
            const heading = blockHeading(page.title, section, block);
            return (
              <Fragment key={index}>
                {blockHeadings && notBlank(heading) ? <SectionHeader text={sentenceCase(heading)} /> : null}
                <FactRuns facts={block.facts} onWrite={onWrite} />
                {page.showsNtrip && block.title === NTRIP_MOUNTPOINT_BLOCK ? <NtripMountpointBrowser onWrite={onWrite} /> : null}
                {section.group === REMOTE_ID_GROUP && block.title === GCS_LOCATION_BLOCK ? <GcsPositionStatus /> : null}
                {section.group === MAVLINK_GROUP && block.title === SIGNING_AFTER_BLOCK ? <SigningKeysSection /> : null}
              </Fragment>
            );
          })}
          {home ? (
            <>
              {notBlank(section.note) && !GROUPS_WITH_A_HEAD_EDITOR.has(section.group) ? <FootNote text={section.note} /> : null}
              {section.group === FLY_VIEW_GROUP ? <RcControlsEditor /> : null}
              {section.group === VIEWER_3D_GROUP ? <OsmFilePicker onWrite={onWrite} /> : null}
              {section.group === OFFLINE_MAPS_GROUP ? <OfflineMapsSection /> : null}
              {section.group === MAVLINK_GROUP ? <LinkStatusSection /> : null}
              {section.group === MAVLINK_ACTIONS_GROUP ? <MavlinkActionsSection onWrite={onWrite} /> : null}
            </>
          ) : null}
        </Fragment>
      ),
    )}
  </>
);

export const ADVANCED_BLOCK = 'Advanced';

const AdvancedBlock = ({ blockKey, children }: { blockKey: string; children: ReactNode }) => {
  const [open, setOpen] = useKeyedState([blockKey], () => false);
  return (
    <>
      <button type="button" aria-expanded={open} onClick={() => setOpen(!open)} className="flex w-full items-center px-4 pt-5 pb-2 text-left">
        <Text typography="labelLarge" color={md('onSurfaceVariant')} className="flex-1">
          {ADVANCED_BLOCK}
        </Text>
        <MaterialIcon
          name="keyboard_arrow_down"
          filled
          label={open ? 'Hide advanced settings' : 'Show advanced settings'}
          style={{ color: md('onSurfaceVariant'), transform: open ? 'rotate(180deg)' : undefined }}
        />
      </button>
      {open ? children : null}
    </>
  );
};

export const FactRuns = ({ facts, onWrite = () => undefined }: { facts: readonly Fact[]; onWrite?: () => void }) => {
  const shared = sharedRebootNote(facts);
  const inert = blockInertNote(facts);
  return (
    <>
      {shared !== null ? (
        <Text typography="bodySmall" color={aircast('warning')} className="block px-4 py-1">
          {shared}
        </Text>
      ) : null}
      <LocalBlockRebootNote.Provider value={shared}>
        <LocalRunInertNote.Provider value={inert}>
          {facts.map((fact, at) => (
            <FactRow key={`${fact.path}#${at}`} fact={fact} onWrite={onWrite} />
          ))}
        </LocalRunInertNote.Provider>
      </LocalBlockRebootNote.Provider>
      {inert !== null ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-4 py-1">
          {inert}
        </Text>
      ) : null}
    </>
  );
};

export const LocalBlockRebootNote = createContext<string | null>(null);

export const LocalRunInertNote = createContext<string | null>(null);

export const LocalSettingsList = createContext(false);

export const LocalTypedDiscarded = createContext<() => boolean>(() => false);

export const SUBTITLE_SEPARATOR = ' · ';

export type ShownSubtitle = { text: string; hasHelp: boolean };

export const shownSubtitle = (subtitle: string, detail: string, helpBehind: boolean, helpOpen: boolean): ShownSubtitle => {
  const hasHelp = helpBehind && notBlank(detail) && (subtitle === detail || subtitle.startsWith(detail + SUBTITLE_SEPARATOR));
  const withoutDetail = subtitle.startsWith(detail) ? subtitle.slice(detail.length) : subtitle;
  const text = hasHelp && !helpOpen ? (withoutDetail.startsWith(SUBTITLE_SEPARATOR) ? withoutDetail.slice(SUBTITLE_SEPARATOR.length) : withoutDetail) : subtitle;
  return { text, hasHelp };
};

const FactTitle = ({
  title,
  color,
  helpOpen,
  onHelp,
  className,
  maxLines,
}: {
  title: string;
  color?: string;
  helpOpen: boolean | null;
  onHelp: () => void;
  className?: string;
  maxLines?: number;
}) => {
  const settingsList = useContext(LocalSettingsList);
  return (
    <div className={`flex items-center ${className ?? ''}`}>
      <Text typography="bodyLarge" fontWeight={settingsList ? 500 : undefined} color={color} maxLines={maxLines} ellipsis className="min-w-0">
        {title}
      </Text>
      {helpOpen !== null ? (
        <span className="flex shrink-0" onClick={(event) => event.stopPropagation()}>
          <IconButton onClick={onHelp} label={helpOpen ? 'Hide help' : 'Help'}>
            <Drawable name="ic_help" size={16} style={{ color: helpOpen ? md('primary') : md('onSurfaceVariant') }} />
          </IconButton>
        </span>
      ) : null}
    </div>
  );
};

export const blockInertNote = (facts: readonly Fact[]): string | null => {
  const inert = facts.filter((fact) => !fact.enabled);
  if (inert.length <= 1) return null;
  const notes = [...new Set(inert.map(inertNote))];
  return notes.length === 1 ? (notes[0] ?? null) : null;
};

export const sharedRebootNote = (facts: readonly Fact[]): string | null => {
  const notes = facts.flatMap((fact) => {
    const note = factRebootNote(fact);
    return note === null ? [] : [note];
  });
  if (notes.length <= 1) return null;
  const distinct = [...new Set(notes)];
  return distinct.length === 1 ? (distinct[0] ?? null) : null;
};

export const NTRIP_MOUNTPOINT_BLOCK = 'Mountpoint';

export const isSecret = (fact: Fact): boolean => fact.name.toLowerCase().endsWith('password');

export const enumLabel = (fact: Fact): string => fact.enumStrings[fact.enumIndex] ?? fact.valueString;

export const shownEnumLabel = (fact: Fact): string => {
  const label = fact.enumStrings[fact.enumIndex];
  return label === undefined ? fact.valueString : sentenceCase(label);
};

export const factSubtitle = (fact: Fact): string => (fact.enumStrings.length > 0 || fact.bitmaskStrings.length > 0 || fact.isBool ? '' : fact.units);

export const FieldSlider = ({ value, slider, label, enabled, onWrite }: { value: number | null; slider: FactSlider; label: string; enabled: boolean; onWrite: (value: number) => void }) => {
  const held = value ?? slider.from;
  const [shown, setShown] = useKeyedState([slider.from, slider.to, slider.decimals, slider.hint, held], () => coerceIn(held, slider.from, slider.to));
  const dragged = useRef<number | null>(null);
  return (
    <>
      <Slider
        value={shown}
        onValueChange={(it) => {
          const snapped = sliderSnapped(it, slider.decimals);
          dragged.current = snapped;
          setShown(snapped);
        }}
        valueRange={[slider.from, slider.to]}
        steps={stepsFor(slider.from, slider.to, 10 ** -slider.decimals)}
        label={label}
        enabled={enabled}
        onValueChangeFinished={() => {
          onWrite(dragged.current ?? shown);
          dragged.current = null;
        }}
      />
      <div className="flex w-full justify-between">
        <Text typography="labelSmall" color={md('onSurfaceVariant')}>
          {sliderValue(slider.from, slider.decimals, '')}
        </Text>
        <Text typography="labelSmall" color={md('onSurfaceVariant')}>
          {sliderValue(slider.to, slider.decimals, '')}
        </Text>
      </div>
    </>
  );
};

const factNumberOrNull = (fact: Fact): number | null => (typeof fact.value === 'number' ? fact.value : toDoubleOrNull(fact.valueString));

export const FactRow = ({
  fact,
  title = sentenceCase(fact.heading),
  subtitle = [fact.detail, factSubtitle(fact)].filter(notBlank).join(' · '),
  titleColor,
  fieldModifier = 'w-full px-4 py-2',
  warning = null,
  onRejected = () => undefined,
  onWrite = () => undefined,
}: {
  fact: Fact;
  title?: string;
  subtitle?: string;
  titleColor?: string;
  fieldModifier?: string;
  warning?: string | null;
  onRejected?: () => void;
  onWrite?: () => void;
}) => {
  const settingsList = useContext(LocalSettingsList);
  const runInert = useContext(LocalRunInertNote);
  const [refusal, setRefusal] = useKeyedState<string | null>([fact.path], () => null);
  const [helpOpen, setHelpOpen] = useKeyedState([fact.path], () => false);
  const [editing, setEditing] = useKeyedState([fact.path], () => false);
  const shown = shownSubtitle(subtitle, fact.detail, settingsList, helpOpen);
  const helpToggle = shown.hasHelp ? helpOpen : null;
  const toggleHelp = () => setHelpOpen(!helpOpen);

  const segmented = !editOnDesktop(fact) && !fact.isBitmask && showsAsSegments(fact.isEnum, fact.valueIsOffTheEnumList, fact.acceptsWrite, fact.enumStrings);
  const asField = !segmented && showsAsField(fact);

  const write = (block: () => Promise<boolean>) =>
    void (async () => {
      const accepted = await block();
      setRefusal(writeRefusal(accepted));
      if (accepted) onWrite();
    })();

  if (asField && settingsList && opensAsValue(fact)) {
    const note = shown.text
      .split(SUBTITLE_SEPARATOR)
      .filter((it) => notBlank(it) && it !== fact.units && it !== fact.detail)
      .join(SUBTITLE_SEPARATOR);
    const slider = inlineSlider(fact);
    const heading = (
      <span className="block">
        <FactTitle title={title} color={titleColor} helpOpen={null} onHelp={() => undefined} />
        {notBlank(note) ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block">
            {note}
          </Text>
        ) : null}
        {warning !== null ? (
          <Text typography="bodySmall" color={aircast('warning')} className="block">
            {warning}
          </Text>
        ) : null}
      </span>
    );
    const details = editing ? <ValueDetailsSheet fact={fact} title={title} onWrite={onWrite} onDismiss={() => setEditing(false)} /> : null;
    if (slider !== null) {
      return (
        <>
          <SliderValueRow fact={fact} label={title} slider={slider} title={heading} onOpen={() => setEditing(true)} onWrite={onWrite} />
          {details}
        </>
      );
    }
    return (
      <>
        <button
          type="button"
          onClick={() => setEditing(true)}
          className="flex min-h-14 w-full items-center gap-3 px-4 py-2.5 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]"
        >
          <span className="min-w-0 flex-1">{heading}</span>
          <Text typography="bodyLarge" color={md('onSurfaceVariant')}>
            {valueText(fact)}
          </Text>
          <Drawable name="ic_chevron_right" style={{ color: md('onSurfaceVariant') }} />
        </button>
        {details}
      </>
    );
  }

  if (asField) {
    const inert = inertNote(fact);
    const note = [...shown.text.split(SUBTITLE_SEPARATOR), ...(!fact.enabled && inert !== runInert ? [inert] : [])].filter((it) => notBlank(it) && it !== fact.units).join(' · ');
    return (
      <div className={fieldModifier}>
        {valueOnTheRight(fact) ? (
          <div className="flex items-center gap-4">
            <div className="min-w-0 flex-1">
              <FactTitle title={title} color={titleColor} helpOpen={helpToggle} onHelp={toggleHelp} />
              {notBlank(note) ? (
                <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block">
                  {note}
                </Text>
              ) : null}
            </div>
            {fact.isEnum && !fact.valueIsOffTheEnumList ? (
              <EnumField fact={fact} style={{ width: CHOICE_VALUE_WIDTH }} write={write} />
            ) : (
              <FactTextField fact={fact} onWrite={onWrite} onRejected={onRejected} onTheRight />
            )}
          </div>
        ) : (
          <>
            <FactTitle title={title} color={titleColor} helpOpen={helpToggle} onHelp={toggleHelp} className="pb-2" />
            {fact.isBitmask ? <BitmaskPicker fact={fact} write={write} /> : <FactTextField fact={fact} onWrite={onWrite} onRejected={onRejected} />}
          </>
        )}
        {fact.slider !== null && !fact.isEnum && !fact.isBitmask ? (
          <FieldSlider value={factNumberOrNull(fact)} slider={fact.slider} label={title} enabled={fact.acceptsWrite} onWrite={(value) => write(() => Qgc.set(fact.path, value))} />
        ) : null}
        {notBlank(note) && !valueOnTheRight(fact) ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block pt-1 pl-4">
            {note}
          </Text>
        ) : null}
        {refusal !== null ? (
          <Text typography="bodySmall" color={md('error')} className="block pt-1 pl-4">
            {refusal}
          </Text>
        ) : null}
      </div>
    );
  }

  const rowToggles = !segmented && fact.isBool && fact.acceptsWrite && !editOnDesktop(fact);
  const toggle = (checked: boolean) => write(() => Qgc.set(fact.path, checked !== fact.inverted));
  const rowNote = shown.text.split(SUBTITLE_SEPARATOR).filter((it) => notBlank(it) && it !== fact.units).join(' · ');
  const inert = inertNote(fact);
  const shownInert = !fact.acceptsWrite && !editOnDesktop(fact) && inert !== runInert ? inert : null;
  const rows = (
    <div>
      <div
        className={`flex min-h-16 w-full items-center gap-4 px-4 py-2.5 ${rowToggles ? 'cursor-pointer' : ''}`}
        onClick={rowToggles ? () => toggle(!fact.boolValue) : undefined}
      >
        <div className="min-w-0 flex-1">
          <FactTitle title={title} color={titleColor} helpOpen={helpToggle} onHelp={toggleHelp} maxLines={3} />
          {notBlank(rowNote) && rowNote.toLowerCase() !== title.toLowerCase() ? (
            <Text typography="bodySmall" color={md('onSurfaceVariant')} maxLines={2} ellipsis className="block">
              {rowNote}
            </Text>
          ) : null}
          {shownInert !== null ? (
            <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block">
              {shownInert}
            </Text>
          ) : null}
        </div>
        {segmented ? <Segments fact={fact} write={write} className="@max-[560px]:hidden" /> : null}
        {!segmented ? (
          <div className="flex max-w-[190px] items-center justify-end">
            {editOnDesktop(fact) ? (
              <div className="flex flex-col items-end text-right">
                <Text typography="bodyMedium" maxLines={2} ellipsis>
                  {shownEnumLabel(fact)}
                </Text>
                <Text typography="labelSmall" color={md('onSurfaceVariant')}>
                  Edit on desktop
                </Text>
              </div>
            ) : !fact.acceptsWrite ? (
              <div className="flex flex-col items-end text-right">
                {fact.isBool ? (
                  <Switch checked={fact.boolValue} onCheckedChange={null} enabled={false} label={title} />
                ) : (
                  <Text typography="bodyMedium" maxLines={2} ellipsis>
                    {[shownEnumLabel(fact), !fact.isEnum ? fact.units : ''].filter(notBlank).join(' ')}
                  </Text>
                )}
              </div>
            ) : fact.isBool ? (
              <span className="flex" onClick={(event) => event.stopPropagation()}>
                <Switch checked={fact.boolValue} onCheckedChange={toggle} label={title} />
              </span>
            ) : (
              <BitmaskPicker fact={fact} write={write} />
            )}
          </div>
        ) : null}
      </div>
      {segmented ? <Segments fact={fact} write={write} className="flex w-full px-4 pb-2.5 @min-[560px]:hidden" /> : null}
      {refusal !== null ? (
        <Text typography="bodySmall" color={md('error')} className="block pb-2 pl-4">
          {refusal}
        </Text>
      ) : null}
    </div>
  );
  return segmented ? <div className="@container">{rows}</div> : rows;
};

const Segments = ({ fact, write, className }: { fact: Fact; write: (block: () => Promise<boolean>) => void; className?: string }) => (
  <SingleChoiceSegmentedButtonRow className={className}>
    {fact.enumStrings.map((option, index) => {
      const raw = fact.enumValues[index];
      const boolOption = fact.isBool && (raw === 'true' || raw === 'false') ? raw === 'true' : null;
      return (
        <SegmentedButton
          key={index}
          index={index}
          count={fact.enumStrings.length}
          selected={boolOption !== null ? boolOption === fact.boolValue : index === fact.enumIndex}
          onClick={() => write(() => (boolOption !== null ? Qgc.set(fact.path, boolOption !== fact.inverted) : Qgc.set(`${fact.path}.enumIndex`, index)))}
          label={<span className="truncate">{sentenceCase(option)}</span>}
        />
      );
    })}
  </SingleChoiceSegmentedButtonRow>
);

export const SEGMENT_LABEL_BUDGET = 28;
export const SEGMENTS_BESIDE_MIN_WIDTH = 560;
export const NUMBER_VALUE_WIDTH = 160;
export const CHOICE_VALUE_WIDTH = 220;

export const sentenceCase = (label: string): string => mapSentenceCase(label);

const SECONDS: ReadonlySet<string> = new Set(['s', 'sec', 'secs', 'second', 'seconds']);

export const shownUnits = (units: string): string => (SECONDS.has(units.toLowerCase()) ? 's' : units);

export const showsAsField = (fact: Fact): boolean => !fact.readOnly && !editOnDesktop(fact) && !fact.isBool;

export const valueOnTheRight = (fact: Fact): boolean => !fact.isString && !fact.isBitmask;

export const showsAsSegments = (isEnum: boolean, offList: boolean, writable: boolean, options: readonly string[]): boolean =>
  isEnum && !offList && writable && options.length >= 2 && options.length <= 4 && options.reduce((total, option) => total + option.length, 0) <= SEGMENT_LABEL_BUDGET;

const hasBit = (raw: number, bit: number): boolean => (BigInt(Math.trunc(raw)) & BigInt(Math.trunc(bit))) !== 0n;

export const bitmaskToggled = (fact: Fact, raw: number, index: number): number => {
  const value = BigInt(Math.trunc(raw));
  const bit = BigInt(Math.trunc(fact.bitmaskValues[index] ?? 0));
  if ((value & bit) !== 0n) return Number(value & ~bit);
  if (fact.firstEntryIsAll && index === 0) return Number(fact.bitmaskValues.slice(1).reduce((all, other) => all & ~BigInt(Math.trunc(other)), value) | bit);
  return Number(value | bit);
};

export const bitmaskEntryEnabled = (fact: Fact, raw: number, index: number): boolean =>
  !(fact.firstEntryIsAll && index > 0 && fact.bitmaskValues.length > 0 && hasBit(raw, fact.bitmaskValues[0] ?? 0));

const GROWING_THREE_LINES = '[&_textarea]:[field-sizing:content] [&_textarea]:max-h-[calc(3lh+1.5rem)]';
const GROWING_FOUR_LINES = '[&_textarea]:[field-sizing:content] [&_textarea]:max-h-[calc(4lh+1.5rem)]';

const BitmaskPicker = ({ fact, write }: { fact: Fact; write: (block: () => Promise<boolean>) => void }) => {
  const [editing, setEditing] = useKeyedState([fact.path], () => false);
  const raw = bitmaskRaw(fact);
  const toggle = (index: number) => write(() => Qgc.set(fact.path, String(bitmaskToggled(fact, raw, index))));
  return (
    <>
      <div className="relative w-full">
        <div inert aria-hidden>
          <OutlinedTextField
            value={bitmaskSummary(fact)}
            onValueChange={() => undefined}
            readOnly
            singleLine={false}
            minLines={1}
            trailingIcon={<MaterialIcon name="keyboard_arrow_down" filled />}
            className={`w-full ${GROWING_THREE_LINES}`}
          />
        </div>
        <button type="button" aria-label={`${fact.title}, ${bitmaskSummary(fact)}`} className="absolute inset-0 cursor-pointer" onClick={() => setEditing(true)} />
      </div>
      {editing ? (
        <AlertDialog
          onDismissRequest={() => setEditing(false)}
          title={fact.title}
          text={
            <div className="max-h-[60vh] overflow-y-auto">
              {fact.bitmaskStrings.map((label, index) => {
                const live = fact.enabled && bitmaskEntryEnabled(fact, raw, index);
                return (
                  <label key={index} className={`flex w-full items-center gap-3 py-1.5 ${live ? 'cursor-pointer' : ''}`}>
                    <Checkbox checked={hasBit(raw, fact.bitmaskValues[index] ?? 0)} enabled={live} onCheckedChange={() => toggle(index)} label={label} />
                    <span className="flex-1">{label}</span>
                  </label>
                );
              })}
            </div>
          }
          confirmButton={<TextButton onClick={() => setEditing(false)}>Done</TextButton>}
        />
      ) : null}
    </>
  );
};

const toLongOrNull = (text: string): number | null => (/^[+-]?\d+$/.test(text) ? Number(text) : null);

const EnumField = ({ fact, style, write }: { fact: Fact; style: CSSProperties; write: (block: () => Promise<boolean>) => void }) => {
  const notice = useContext(LocalChangeNotice);
  const choose = (index: number): Promise<boolean> => {
    const raw = fact.enumValues[index];
    const rawChoice = fact.rawChoice && raw !== undefined ? toLongOrNull(raw) : null;
    return rawChoice !== null ? Qgc.set(fact.path, rawChoice) : Qgc.set(`${fact.path}.enumIndex`, index);
  };
  return (
    <ChoiceField
      label={null}
      value={shownEnumLabel(fact)}
      options={fact.enumStrings.map(sentenceCase)}
      style={style}
      enabled={fact.enabled}
      groups={fact.enumGroups}
      onPick={(index) => {
        const previous = fact.enumIndex;
        write(async () => {
          const accepted = await choose(index);
          if (accepted && index !== previous && previous >= 0) {
            notice.show(`${sentenceCase(fact.heading)}: ${sentenceCase(fact.enumStrings[index] ?? '')}`, async () => {
              write(() => choose(previous));
              return null;
            });
          }
          return accepted;
        });
      }}
    />
  );
};

export const ADVANCED_SAFETY = 'Advanced safety settings';

export const rowChoiceLabel = (label: string): string => {
  const at = label.indexOf(', ');
  return at < 0 ? label : label.slice(0, at);
};

const DISABLED_VALUE_ALPHA = 0.5;

export const ChoiceField = ({
  label,
  value,
  options,
  className,
  style,
  enabled = true,
  groups = [],
  onPick,
}: {
  label: string | null;
  value: string;
  options: readonly string[];
  className?: string;
  style?: CSSProperties;
  enabled?: boolean;
  groups?: readonly string[];
  onPick: (index: number) => void;
}) => {
  const [expanded, setExpanded] = useState(false);
  const plain = useContext(LocalSettingsList) && label === null;
  return (
    <div className={`relative flex items-center justify-end ${className ?? ''}`} style={style}>
      {plain ? (
        <div className="flex min-h-12 min-w-0 items-center">
          <Text typography="bodyLarge" color={withAlpha(md('onSurfaceVariant'), enabled ? 1 : DISABLED_VALUE_ALPHA)} maxLines={1} ellipsis>
            {rowChoiceLabel(value)}
          </Text>
          <Drawable name="ic_chevron_right" style={{ color: md('onSurfaceVariant') }} />
        </div>
      ) : (
        <div inert aria-hidden className="w-full">
          <OutlinedTextField
            value={value}
            onValueChange={() => undefined}
            readOnly
            enabled={enabled}
            label={label === null ? undefined : <span className="truncate">{label}</span>}
            trailingIcon={<MaterialIcon name="keyboard_arrow_down" filled />}
            className="w-full"
          />
        </div>
      )}
      <button type="button" aria-label={label === null ? value : `${label}, ${value}`} aria-haspopup="menu" disabled={!enabled} className="absolute inset-0 cursor-pointer disabled:cursor-default" onClick={() => setExpanded(true)} />
      {plain ? (
        <SafeChoiceMenu expanded={expanded} options={options} onDismiss={() => setExpanded(false)} onPick={onPick} />
      ) : (
        <DropdownMenu expanded={expanded} onDismissRequest={() => setExpanded(false)}>
          {options.map((option, index) => {
            const group = groups[index];
            const header = group !== undefined && notBlank(group) && group !== groups[index - 1] ? group : null;
            return (
              <Fragment key={index}>
                {header !== null ? (
                  <>
                    {index > 0 ? <HorizontalDivider /> : null}
                    <Text typography="labelMedium" color={md('primary')} className="block px-3 py-2">
                      {header}
                    </Text>
                  </>
                ) : null}
                <DropdownMenuItem
                  text={option}
                  onClick={() => {
                    setExpanded(false);
                    onPick(index);
                  }}
                />
              </Fragment>
            );
          })}
        </DropdownMenu>
      )}
    </div>
  );
};

export const factValueLines = (fact: Fact): number => (fact.isString ? 4 : 1);

const toDoubleOrNull = (text: string): number | null => {
  const trimmed = text.trim();
  if (trimmed === '' || /^[+-]?0x/i.test(trimmed)) return null;
  const parsed = Number(trimmed);
  return Number.isNaN(parsed) && trimmed !== 'NaN' ? null : parsed;
};

export const truncationRefusal = (fact: Fact, text: string): string | null => {
  if (!fact.wholeNumbersOnly) return null;
  const typed = toDoubleOrNull(text);
  if (typed === null || !Number.isFinite(typed)) return null;
  return typed === Math.floor(typed) ? null : 'Invalid number';
};

export const typedValue = (text: string): string => text.replaceAll('\n', '');

export const ASPECT_RATIO = 'aspectRatio';
const RATIO_TOLERANCE = 0.005;
const COMMON_RATIOS: readonly (readonly [number, number])[] = [
  [16, 9],
  [4, 3],
  [21, 9],
  [16, 10],
  [3, 2],
  [5, 4],
  [1, 1],
];

export const ratioText = (value: number): string | null => {
  const found = COMMON_RATIOS.find(([width, height]) => Math.abs(width / height - value) < RATIO_TOLERANCE);
  return found === undefined ? null : `${found[0]}:${found[1]}`;
};

export const ratioValue = (text: string): string | null => {
  const parts = text.split(':');
  if (parts.length !== 2) return null;
  const [width, height] = parts.map((part) => toDoubleOrNull(part));
  return width !== null && width !== undefined && height !== null && height !== undefined && width > 0 && height > 0 ? format('%.6f', width / height) : null;
};

export const isAddress = (fact: Fact): boolean => fact.isString && (fact.name.endsWith('Url') || fact.name.endsWith('URL'));

export const factKeyboard = (fact: Fact): KeyboardType => {
  const minimum = toDoubleOrNull(fact.minString);
  if (isAddress(fact)) return 'uri';
  if (fact.isString || fact.isBool) return 'text';
  if (fact.wholeNumbersOnly && minimum !== null && minimum >= 0) return 'number';
  if (minimum === null || minimum < 0) return 'text';
  return 'decimal';
};

export const fieldText = (fact: Fact): string => (fact.isString || isSecret(fact) ? fact.valueString : plainNumber(fact.valueString));

const expandedExponent = (text: string): string => {
  const [mantissa = '', exponent = '0'] = text.split('e');
  const negative = mantissa.startsWith('-');
  const [whole = '', fraction = ''] = mantissa.replace('-', '').split('.');
  const point = whole.length + Number(exponent);
  const digits = whole + fraction;
  const body = point <= 0 ? `0.${'0'.repeat(-point)}${digits}` : `${digits.slice(0, point).padEnd(point, '0')}${digits.length > point ? `.${digits.slice(point)}` : ''}`;
  return `${negative ? '-' : ''}${body}`;
};

export const plainNumber = (text: string): string => {
  const parsed = toDoubleOrNull(text);
  if (parsed === null) return text;
  if (!Number.isFinite(parsed) || Math.abs(parsed) >= 1e15) return text;
  if (parsed === Math.floor(parsed)) return String(Math.trunc(parsed) + 0);
  const shortest = String(parsed);
  return shortest.includes('e') ? expandedExponent(shortest) : shortest;
};

export const factConstraintNote = (fact: Fact): string | null => {
  const parts = [
    notBlank(fact.minString) && !fact.minIsDefaultForType ? `Min ${plainNumber(fact.minString)}` : null,
    notBlank(fact.maxString) && !fact.maxIsDefaultForType ? `Max ${plainNumber(fact.maxString)}` : null,
    notBlank(fact.defaultValueString) ? `Default ${plainNumber(fact.defaultValueString)}` : null,
  ].filter((part): part is string => part !== null);
  return parts.length > 0 ? parts.join(' · ') : null;
};

export const factRebootNote = (fact: Fact): string | null =>
  fact.vehicleRebootRequired ? 'Reboot vehicle for changes to take effect.' : fact.qgcRebootRequired ? 'Restart Aircast for this to take effect.' : null;

export const writeRefusal = (accepted: boolean): string | null => (accepted ? null : 'That change was not accepted.');

export const validationMessage = (result: unknown): string | null => (typeof result === 'string' && notBlank(result) ? result : null);

const blockingRejection = async (fact: Fact, text: string): Promise<string | null> =>
  truncationRefusal(fact, text) ?? validationMessage(await Qgc.invokeResult(`${fact.path}.validate`, text, false));

export const storedText = (fact: Fact, typed: string): string => (fact.name === ASPECT_RATIO ? (ratioValue(typed) ?? typed) : typed);

export const shownText = (fact: Fact): string => {
  if (fact.name !== ASPECT_RATIO) return fieldText(fact);
  const value = toDoubleOrNull(fact.valueString);
  return (value === null ? null : ratioText(value)) ?? fieldText(fact);
};

const blurFocused = () => {
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
};

const FactTextField = ({ fact, onWrite, onRejected = () => undefined, onTheRight = false }: { fact: Fact; onWrite: () => void; onRejected?: () => void; onTheRight?: boolean }) => {
  const [editing, setEditing] = useKeyedState<string | null>([fact.path], () => null);
  const [rejection, setRejection] = useKeyedState<string | null>([fact.path], () => null);
  const [revealed, setRevealed] = useKeyedState([fact.path], () => false);
  const [focused, setFocused] = useState(false);
  const blockRebootNote = useContext(LocalBlockRebootNote);
  const discarded = useContext(LocalTypedDiscarded);
  const secret = isSecret(fact);
  const pending = editing !== null && editing !== shownText(fact) ? editing : null;

  const commit = () => {
    if (pending === null) return;
    const committed = storedText(fact, pending);
    void (async () => {
      const refused = await blockingRejection(fact, committed);
      if (refused !== null) {
        setRejection(refused);
        onRejected();
        return;
      }
      const refusal = await Qgc.writeRefusal(fact.path, committed);
      setRejection(refusal);
      if (refusal === null) {
        setEditing(null);
        onWrite();
      }
    })();
  };
  const latestCommit = useUpdatedRef(commit);
  const latestPending = useUpdatedRef(pending);

  useEffect(
    () => () => {
      const typed = latestPending.current;
      if (typed === null || discarded()) return;
      const committed = storedText(fact, typed);
      void (async () => {
        if ((await blockingRejection(fact, committed)) === null) await Qgc.writeRefusal(fact.path, committed);
      })();
    },
    [fact.path],
  );

  const constraint = factConstraintNote(fact);
  const reboot = factRebootNote(fact);
  return (
    <div
      className="flex flex-col"
      style={onTheRight ? { width: NUMBER_VALUE_WIDTH } : undefined}
      onFocus={() => setFocused(true)}
      onBlur={(event) => {
        if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
        setFocused(false);
        if (!discarded()) latestCommit.current();
      }}
    >
      <OutlinedTextField
        value={editing ?? shownText(fact)}
        ariaLabel={fact.title}
        enabled={fact.enabled}
        suffix={notBlank(fact.units) ? shownUnits(fact.units) : undefined}
        keyboardType={secret && !revealed ? 'password' : factKeyboard(fact)}
        autoCorrect={!isAddress(fact)}
        leadingIcon={secret ? <TextButton onClick={() => setRevealed(!revealed)}>{revealed ? 'Hide' : 'Show'}</TextButton> : undefined}
        onValueChange={(it) => {
          setEditing(typedValue(it));
          setRejection(null);
        }}
        singleLine={factValueLines(fact) === 1}
        minLines={1}
        isError={rejection !== null}
        onDone={blurFocused}
        className={`w-full ${onTheRight ? '[&_input]:text-right' : ''} ${GROWING_FOUR_LINES}`}
        trailingIcon={pending !== null ? <TextButton onClick={commit}>Set</TextButton> : undefined}
      />
      {rejection !== null ? (
        <Text typography="bodySmall" color={md('error')} className="block pt-1 pl-4">
          {rejection}
        </Text>
      ) : null}
      {rejection === null && notBlank(fact.problem) && (editing === null || !notBlank(editing)) ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block pt-1 pl-4">
          {fact.problem}
        </Text>
      ) : null}
      {rejection === null && focused && constraint !== null ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block pt-1 pl-4">
          {constraint}
        </Text>
      ) : null}
      {reboot !== null && reboot !== blockRebootNote ? (
        <Text typography="bodySmall" color={aircast('warning')} className="block pt-1 pl-4">
          {reboot}
        </Text>
      ) : null}
    </div>
  );
};
