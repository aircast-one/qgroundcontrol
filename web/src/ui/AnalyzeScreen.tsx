import { type CSSProperties, type ReactNode, Fragment, useContext, useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { useBackHandler } from '@/kotlin/Compose';
import { SnackbarHost, Surface, Text, useSnackbarHostState } from '@/m3';
import { VEHICLES_VIEW } from '@/map/VehiclePicker';
import type { DrawableName } from '@/res/drawables';
import { md } from '@/theme/Colors';
import { activeVehicleId } from '@/ui/ActiveVehicle';
import { LocalAppNavigation, navigationRefusal } from '@/ui/AppNavigation';
import { ConsoleScreen } from '@/ui/ConsoleScreen';
import { FirmwareScreen } from '@/ui/FirmwareScreen';
import { useElementSize } from '@/ui/FlyShell';
import { useHasVehicle } from '@/ui/FlyState';
import { GeoTagScreen } from '@/ui/GeoTagScreen';
import { INSPECTOR_VIEW, InspectorScreen, inspectorRateText } from '@/ui/InspectorScreen';
import { LogDownloadScreen } from '@/ui/LogDownloadScreen';
import { DETAIL_PANE_MAX_WIDTH, LIST_DETAIL_MIN_WIDTH, LIST_PANE_WIDTH } from '@/ui/SettingsScreen';
import { EmptyState, LocalTwoPane, PageTopBar, SectionHeader, SetupRow, SetupState } from '@/ui/SetupRow';
import { MESSAGES, type VehicleMessage, VehicleMessagesPage, severitySummary, unreadCount, vehicleMessages } from '@/ui/VehicleMessages';
import { VIBRATION_VIEW, type VibrationReading, VibrationScreen, severityLabel, vibrationCaveat, vibrationGlance, vibrationReading, worstSeverity } from '@/ui/VibrationScreen';

export type AnalyzeSection = { readonly name: 'FlightData' | 'Live' | 'Vehicle'; readonly ordinal: number; readonly title: string };

export const AnalyzeSection = {
  FlightData: { name: 'FlightData', ordinal: 0, title: 'Flight data' },
  Live: { name: 'Live', ordinal: 1, title: 'Live' },
  Vehicle: { name: 'Vehicle', ordinal: 2, title: 'Vehicle' },
} as const satisfies Record<string, AnalyzeSection>;

export const AnalyzeSectionEntries: readonly AnalyzeSection[] = [AnalyzeSection.FlightData, AnalyzeSection.Live, AnalyzeSection.Vehicle];

export type AnalyzePage = {
  readonly name: 'LogDownload' | 'GeoTag' | 'Vibration' | 'Inspector' | 'Console' | 'Messages' | 'Firmware';
  readonly ordinal: number;
  readonly label: string;
  readonly description: string;
  readonly section: AnalyzeSection;
  readonly icon: DrawableName;
  readonly requiresVehicle: boolean;
};

export const AnalyzePage = {
  LogDownload: { name: 'LogDownload', ordinal: 0, label: 'Flight logs', description: 'Download flight logs from the vehicle', section: AnalyzeSection.FlightData, icon: 'ic_download', requiresVehicle: true },
  GeoTag: { name: 'GeoTag', ordinal: 1, label: 'Geotag images', description: 'Match photos to the flight log', section: AnalyzeSection.FlightData, icon: 'ic_photo_camera', requiresVehicle: false },
  Vibration: { name: 'Vibration', ordinal: 2, label: 'Vibration', description: 'Accelerometer vibration levels and clipping', section: AnalyzeSection.Live, icon: 'ic_vibration', requiresVehicle: true },
  Inspector: { name: 'Inspector', ordinal: 3, label: 'MAVLink inspector', description: 'Live message rates and field values', section: AnalyzeSection.Live, icon: 'ic_analytics', requiresVehicle: true },
  Console: { name: 'Console', ordinal: 4, label: 'Console', description: 'Vehicle shell over MAVLink', section: AnalyzeSection.Live, icon: 'ic_terminal', requiresVehicle: true },
  Messages: { name: 'Messages', ordinal: 5, label: 'Messages', description: 'What the vehicle has said since it connected', section: AnalyzeSection.Live, icon: 'ic_description', requiresVehicle: false },
  Firmware: { name: 'Firmware', ordinal: 6, label: 'Firmware', description: 'Flash a board through its bootloader over USB', section: AnalyzeSection.Vehicle, icon: 'ic_developer_board', requiresVehicle: false },
} as const satisfies Record<string, AnalyzePage>;

export const AnalyzePageEntries: readonly AnalyzePage[] = [
  AnalyzePage.LogDownload,
  AnalyzePage.GeoTag,
  AnalyzePage.Vibration,
  AnalyzePage.Inspector,
  AnalyzePage.Console,
  AnalyzePage.Messages,
  AnalyzePage.Firmware,
];

export const analyzeStatus = (page: AnalyzePage, unread: number, vibration: string | null): readonly [string, SetupState] => {
  if (page === AnalyzePage.Messages && unread > 0) return [`${unread}`, SetupState.NeedsAttention];
  if (page === AnalyzePage.Vibration && (vibration === 'danger' || vibration === 'warning')) return [severityLabel(vibration), SetupState.NeedsAttention];
  if (page === AnalyzePage.Vibration && vibration === 'normal') return [severityLabel(vibration), SetupState.Done];
  return ['', SetupState.Neutral];
};

export const analyzeSubtitle = (page: AnalyzePage, messages: readonly VehicleMessage[], vibration: VibrationReading | null = null, rate: string | null = null): string => {
  if (page === AnalyzePage.Inspector) return rate ?? page.description;
  if (page === AnalyzePage.Messages && messages.length > 0) return severitySummary(messages);
  if (page === AnalyzePage.Vibration) return (vibration !== null ? vibrationGlance(vibration) : null) ?? page.description;
  return page.description;
};

export const REQUIRES_VEHICLE = 'Requires a connected vehicle';

export const analyzeGate = (page: AnalyzePage, connected: boolean): string | null => (page.requiresVehicle && !connected ? REQUIRES_VEHICLE : null);

export const analyzeNote = (page: AnalyzePage, connected: boolean, vibration: string | null): string | null => {
  if (!connected) return null;
  if (page === AnalyzePage.Vibration) return vibration;
  return null;
};

const ANALYZE_SECTIONS: readonly AnalyzeSection[] = [...new Set(AnalyzePageEntries.map((it) => it.section))];

const AnalyzePageList = ({ onSelect, className, style, selected = null }: { onSelect: (page: AnalyzePage) => void; className?: string; style?: CSSProperties; selected?: AnalyzePage | null }) => {
  const vibrationJson = useQgcPath(VIBRATION_VIEW);
  const connected = useHasVehicle();
  const caveat = useMemo(() => vibrationCaveat(vibrationJson), [vibrationJson]);
  const messagesJson = useQgcPath(MESSAGES);
  const vibration = useMemo(() => vibrationReading(vibrationJson), [vibrationJson]);
  const inspectorJson = useQgcPath(INSPECTOR_VIEW);
  const rate = useMemo(() => inspectorRateText(inspectorJson), [inspectorJson]);
  const vibrationLevel = vibration !== null ? worstSeverity(vibration) : null;
  const unread = useMemo(() => unreadCount(messagesJson), [messagesJson]);
  const messages = useMemo(() => vehicleMessages(messagesJson), [messagesJson]);

  return (
    <div className={`flex flex-col overflow-y-auto ${className ?? ''}`} style={style}>
      <div className="flex w-full flex-col px-4 pt-5 pb-2">
        <Text typography="headlineMedium">Analyze</Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          Logs and tools for the connected vehicle
        </Text>
      </div>
      {ANALYZE_SECTIONS.map((section) => (
        <Fragment key={section.name}>
          <SectionHeader text={section.title} />
          {AnalyzePageEntries.filter((page) => page.section === section).map((page) => {
            const [status, state] = analyzeStatus(page, unread, vibrationLevel);
            return (
              <SetupRow
                key={page.name}
                title={page.label}
                status={status}
                state={state}
                subtitle={[analyzeSubtitle(page, messages, vibration, rate), analyzeNote(page, connected, caveat)].filter((it) => it !== null).join('\n')}
                onClick={() => onSelect(page)}
                icon={page.icon}
                selected={page === selected}
              />
            );
          })}
        </Fragment>
      ))}
    </div>
  );
};

export const AnalyzeScreen = ({ page, onSelect, className }: { page: AnalyzePage | null; onSelect: (page: AnalyzePage | null) => void; className?: string }) => {
  const navigation = useContext(LocalAppNavigation);
  const outerTwoPane = useContext(LocalTwoPane);
  const toasts = useSnackbarHostState();
  const { ref, size } = useElementSize();
  const switchTo = (next: AnalyzePage | null) => {
    const refused = page !== null ? navigationRefusal(navigation.blockedReason.value, true) : null;
    if (refused !== null) void toasts.showSnackbar(refused);
    else onSelect(next);
  };
  const leave = () => switchTo(null);
  useBackHandler(page !== null, leave);

  const wide = size.width >= LIST_DETAIL_MIN_WIDTH;
  return (
    <div ref={ref} className={`relative size-full ${className ?? ''}`}>
      <div className="flex size-full">
        {wide ? <AnalyzePageList onSelect={switchTo} className="h-full shrink-0 bg-surface-container-low" style={{ width: LIST_PANE_WIDTH }} selected={page} /> : null}
        {page === null ? (
          wide ? (
            <Surface className="h-full min-w-0 flex-1">
              <EmptyState icon="ic_analytics" title="Analyze" text="Choose a tool on the left." />
            </Surface>
          ) : (
            <Surface className="flex size-full flex-col">
              <AnalyzePageList onSelect={onSelect} className="size-full" />
            </Surface>
          )
        ) : (
          <div className={wide ? 'h-full min-w-0 flex-1 bg-surface' : 'size-full'}>
            <LocalTwoPane.Provider value={wide || outerTwoPane}>
              <AnalyzePageBody key={page.name} page={page} leave={leave} style={wide ? { maxWidth: DETAIL_PANE_MAX_WIDTH } : undefined} />
            </LocalTwoPane.Provider>
          </div>
        )}
      </div>
      <div className="pointer-events-none absolute inset-x-0 bottom-4 flex justify-center">
        <SnackbarHost hostState={toasts} />
      </div>
    </div>
  );
};

const PAGE_BODIES: Readonly<Record<AnalyzePage['name'], () => ReactNode>> = {
  LogDownload: () => <LogDownloadScreen />,
  Console: () => <ConsoleScreen />,
  Inspector: () => <InspectorScreen />,
  Vibration: () => <VibrationScreen />,
  GeoTag: () => <GeoTagScreen />,
  Firmware: () => <FirmwareScreen />,
  Messages: () => <VehicleMessagesPage />,
};

const AnalyzePageBody = ({ page, leave, style }: { page: AnalyzePage; leave: () => void; style?: CSSProperties }) => {
  const gate = analyzeGate(page, useHasVehicle());
  const vehiclesJson = useQgcPath(VEHICLES_VIEW);
  const reloadKey = useMemo(() => (page.requiresVehicle ? activeVehicleId(vehiclesJson) : null), [vehiclesJson, page]);
  return (
    <div className="flex size-full flex-col" style={style}>
      <PageTopBar title={page.label} backLabel="Back to Analyze" onBack={leave} />
      <Surface className="flex min-h-0 w-full flex-1 flex-col">
        {gate !== null ? <EmptyState icon={page.icon} title={gate} text="" /> : <Fragment key={String(reloadKey)}>{PAGE_BODIES[page.name]()}</Fragment>}
      </Surface>
    </div>
  );
};
