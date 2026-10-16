import { type CSSProperties, Fragment, type ReactNode, createContext, useCallback, useContext, useMemo, useState } from 'react';
import { type Json, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useQgcFacts, useQgcPath } from '@/bridge/QgcState';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useTicker } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { coerceIn, format, roundToInt } from '@/kotlin/Std';
import { AlertDialog, ListItem, Surface, Text, TextButton } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { useSilentSeconds } from '@/map/Silence';
import { Drawable, MaterialIcon } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { AircastLinkCell } from '@/ui/AircastLinkIndicator';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { EscIndicatorCell } from '@/ui/EscIndicator';
import { LocalFlyScreenState, OpenOnRequest } from '@/ui/FlyScreenState';
import { useFlyIsPortrait } from '@/ui/FlyShell';
import { FLY_STATE, type FlyState, type TelemetryLink, flyState, useHasVehicle } from '@/ui/FlyState';
import { GcsBatteryCell, type GcsBatteryReading, useRememberGcsBattery } from '@/ui/GcsBatteryIndicator';
import { GimbalIndicatorCell } from '@/ui/GimbalIndicator';
import { GpsResilienceCell } from '@/ui/GpsResilienceIndicator';
import { GUIDED_ACTIONS, guidedOffers } from '@/ui/GuidedActions';
import {
  BATTERY_VIEW,
  type BatteryHeadline,
  type DetailRow,
  RETURN_NOW_SPOKEN,
  SEVERITY_SECONDARY,
  batteryDetail,
  batteryHeadline,
  batteryReturnOffered,
  gpsDetail,
  gpsStatus,
  linkDetail,
  silenceText,
  totalCurrent,
  totalDraw,
} from '@/ui/InstrumentDetail';
import { JoystickIndicatorCell } from '@/ui/JoystickIndicator';
import { useOsdBackdrop } from '@/ui/Osd';
import { LayoutWidget, movedKey, orderedKeys } from '@/ui/OverlayLayout';
import { ParameterForm } from '@/ui/ParameterForm';
import { RemoteIdIndicatorCell } from '@/ui/RemoteIdIndicator';
import { RtkIndicatorCell, RtkSettingsSheetContent, type RtkStatus, useRememberRtkStatus } from '@/ui/RtkIndicator';
import { FactRow } from '@/ui/SettingsScreen';
import { FootNote } from '@/ui/SetupRow';
import { useHasSetupPage } from '@/ui/SetupScreen';
import { SETUP, parametersReady } from '@/ui/SetupView';
import { SupportForwardingCell } from '@/ui/SupportForwardingCell';
import { VEHICLE_LINKS, linkCell, linkNames, vehicleLinks } from '@/ui/VehicleLinks';

const VEHICLE_FLIGHT_TIME = 'vehicle.flightTime';
const GPS_VIEW = 'view.gps';

export type BatteryLevel = { readonly name: 'Normal' | 'Caution' | 'Warning' | 'Critical'; readonly ordinal: number };
export const BatteryLevel = {
  Normal: { name: 'Normal', ordinal: 0 },
  Caution: { name: 'Caution', ordinal: 1 },
  Warning: { name: 'Warning', ordinal: 2 },
  Critical: { name: 'Critical', ordinal: 3 },
} as const satisfies Record<string, BatteryLevel>;
export const BatteryLevelEntries: readonly BatteryLevel[] = [BatteryLevel.Normal, BatteryLevel.Caution, BatteryLevel.Warning, BatteryLevel.Critical];

export type BatteryReading = { text: string; level: BatteryLevel; packs: number };
export const BatteryReading = (fields: Omit<BatteryReading, 'packs'> & Partial<Pick<BatteryReading, 'packs'>>): BatteryReading => ({ packs: 1, ...fields });

export const packCountText = (packs: number): string => (packs > 1 ? `×${packs}` : '');

export const batteryLevelOf = (name: string | null): BatteryLevel => {
  switch (name) {
    case 'critical':
      return BatteryLevel.Critical;
    case 'warning':
      return BatteryLevel.Warning;
    case 'caution':
      return BatteryLevel.Caution;
    default:
      return BatteryLevel.Normal;
  }
};

export const batteryReadings = (view: Json | null): readonly BatteryReading[] => {
  const packs = view !== null && optBoolean(view, 'available') ? optJSONArray(view, 'indicatorPacks') : null;
  return jsonObjects(packs).flatMap((pack) => {
    const lines = jsonStrings(optJSONArray(pack, 'indicatorLines')).filter((line) => line.trim() !== '');
    if (lines.length === 0) return [];
    const label = optText(pack, 'indicatorLabel');
    return [
      BatteryReading({
        text: [label === '' ? null : label, lines.join(' · ')].filter((part): part is string => part !== null).join(' '),
        level: batteryLevelOf(optText(pack, 'level')),
        packs: optInt(pack, 'packCount', 1),
      }),
    ];
  });
};

export type RcCell = { text: string; lost: boolean };

export const rcCell = (state: FlyState | null): RcCell | null =>
  state === null || !state.rcSupported || state.rcSignalText.trim() === '' ? null : { text: `${state.rcSignalText} RC`, lost: state.rcSignal === 0 };

export type FixLevel = { readonly name: 'None' | 'TwoD' | 'Good'; readonly ordinal: number };
export const FixLevel = {
  None: { name: 'None', ordinal: 0 },
  TwoD: { name: 'TwoD', ordinal: 1 },
  Good: { name: 'Good', ordinal: 2 },
} as const satisfies Record<string, FixLevel>;
export const FixLevelEntries: readonly FixLevel[] = [FixLevel.None, FixLevel.TwoD, FixLevel.Good];

export const fixLevel = (lock: number): FixLevel | null => {
  if (Number.isNaN(lock)) return null;
  if (lock < 2) return FixLevel.None;
  return lock < 3 ? FixLevel.TwoD : FixLevel.Good;
};

export const NO_COUNT = '--';

export const satsText = (fix: FixLevel, count: string): string => {
  if (fix === FixLevel.None) return 'No fix';
  if (fix === FixLevel.TwoD) return count.trim() === '' ? '2D only' : `${count} sats · 2D only`;
  return count.trim() === '' ? NO_COUNT : `${count} sats`;
};

const batteryLevelColour = (level: BatteryLevel): string | null => {
  if (level === BatteryLevel.Caution) return aircast('warning');
  if (level === BatteryLevel.Warning) return aircast('alert');
  if (level === BatteryLevel.Critical) return md('error');
  return null;
};

const gpsColour = (fix: FixLevel): string | null => {
  if (fix === FixLevel.None) return md('error');
  return fix === FixLevel.TwoD ? aircast('warning') : null;
};

export const StatusPill = ({ className }: { className?: string }) => {
  const rtk = useRememberRtkStatus();
  const gcsBattery = useRememberGcsBattery();
  const vehicle = useHasVehicle();
  const portrait = useFlyIsPortrait();
  const backdrop = useOsdBackdrop(withAlpha(Color.Black, 0.45));
  const [maxWidth, setMaxWidth] = useState(Number.POSITIVE_INFINITY);
  const measure = useCallback((element: HTMLElement | null) => {
    const parent = element?.parentElement ?? null;
    if (parent === null) return undefined;
    const read = () => setMaxWidth(parent.clientWidth);
    read();
    const observer = new ResizeObserver(read);
    observer.observe(parent);
    return () => observer.disconnect();
  }, []);
  if (!statusPillShown(vehicle, rtk !== null, gcsBattery !== null)) return null;
  const narrow = portrait || maxWidth < NARROW_PILL_WIDTH;
  return (
    <div ref={measure} className={`min-w-0 ${className ?? ''}`}>
      <Surface shape="full" color={backdrop} contentColor={aircast('outdoorForeground')} className="max-w-full overflow-hidden">
        <LocalCompactStatus.Provider value>
          <LocalNarrowStatus.Provider value={narrow}>
            <StatusReadingsInline rtk={rtk} gcsBattery={gcsBattery} style={{ paddingInlineEnd: stripGap(narrow), paddingTop: 6, paddingBottom: 6 }} />
          </LocalNarrowStatus.Provider>
        </LocalCompactStatus.Provider>
      </Surface>
    </div>
  );
};

const STRIP_GAP = 14;
const NARROW_STRIP_GAP = 6;
const NARROW_PILL_WIDTH = 120;
const NARROW_STATUS_CELLS = 1;

export const LocalNarrowStatus = createContext(false);

const stripGap = (narrow: boolean): number => (narrow ? NARROW_STRIP_GAP : STRIP_GAP);

export const shownStatusCells = (narrow: boolean): number => (narrow ? NARROW_STATUS_CELLS : COMPACT_STATUS_CELLS);

const LeadingGap = ({ gap, children }: { gap: number; children: ReactNode }) => (
  <div className="flex shrink-0 items-center empty:hidden" style={{ marginInlineStart: gap }}>
    {children}
  </div>
);

export const statusPillShown = (vehicle: boolean, rtk: boolean, gcsBattery: boolean): boolean => vehicle || rtk || gcsBattery;

type ReadingsProps = { rtk: RtkStatus | null; gcsBattery: GcsBatteryReading | null; className?: string; style?: CSSProperties };

export const StatusReadingsInline = (props: ReadingsProps) => (useHasVehicle() ? <VehicleReadings {...props} /> : <NoVehicleReadings {...props} />);

const NoVehicleReadings = ({ rtk, gcsBattery, className, style }: ReadingsProps) => (
  <div className={`flex items-center ${className ?? ''}`} style={style}>
    <LeadingGap gap={STRIP_GAP}>
      <RtkIndicatorCell status={rtk} />
    </LeadingGap>
    <LeadingGap gap={STRIP_GAP}>
      <GcsBatteryCell reading={gcsBattery} />
    </LeadingGap>
  </div>
);

const BatteryCells = ({ batteries, live, timeLeft, current, onOpen }: { batteries: readonly BatteryReading[]; live: boolean; timeLeft: string; current: string; onOpen: () => void }) => {
  const compact = useContext(LocalCompactStatus);
  const narrow = useContext(LocalNarrowStatus);
  if (batteries.length === 0) return null;
  const extras = narrow ? [] : ([[timeLeft, 'flight time left'], [current, 'current draw']] as const).filter(([value]) => value !== '');
  return (
    <div className="flex items-center" style={{ gap: BATTERY_GAP }}>
      {batteries.map((it, at) => {
        const colour = live ? batteryLevelColour(it.level) : md('outline');
        return compact ? (
          <BatteryRing key={at} text={it.text} colour={colour} count={narrow ? '' : packCountText(it.packs)} onClick={onOpen} />
        ) : (
          <InlineCell key={at} text={[it.text, packCountText(it.packs)].filter((part) => part !== '').join(' ')} colour={colour} icon="ic_battery_5_bar" onClick={onOpen} />
        );
      })}
      {extras.map(([value, meaning]) => (
        <button
          key={meaning}
          type="button"
          aria-label={`${value} ${meaning}`}
          aria-description="Battery"
          onClick={onOpen}
          className="text-label-large tabular-nums"
          style={{ color: live ? aircast('outdoorForeground') : md('outline') }}
        >
          {value}
        </button>
      ))}
    </div>
  );
};

const FlightTimeSlot = () => (useContext(LocalCompactStatus) ? <FlightTimeCell /> : null);

const VehicleReadings = ({ rtk, gcsBattery, className, style }: ReadingsProps) => {
  const navigation = useContext(LocalAppNavigation);
  const stateJson = useQgcPath(FLY_STATE);
  const state = useMemo(() => flyState(stateJson), [stateJson]);
  const live = (state?.staleNotice ?? '').trim() === '';
  const batteryJson = useQgcPath(BATTERY_VIEW);
  const batteries = useMemo(() => batteryReadings(batteryJson), [batteryJson]);
  const timeLeft = useMemo(() => batteryHeadline(batteryJson)?.timeLeft ?? '', [batteryJson]);
  const current = useMemo(() => totalCurrent(batteryJson) ?? '', [batteryJson]);
  const linksJson = useQgcPath(VEHICLE_LINKS);
  const links = useMemo(() => linkCell(vehicleLinks(linksJson)), [linksJson]);
  const gpsJson = useQgcPath(GPS_VIEW);
  const gps = useMemo(() => gpsStatus(gpsJson), [gpsJson]);
  const fix = fixLevel(gps?.lock ?? Number.NaN);
  const satellites = gps?.satellites === null || gps?.satellites === undefined ? '' : String(gps.satellites);
  const [detail, setDetail] = useState<StripDetail | null>(null);
  const [batterySettings, setBatterySettings] = useState(false);
  const [allStatus, setAllStatus] = useState(false);
  const [batteryDisplay, setBatteryDisplay] = useState(false);
  const [rtkSettings, setRtkSettings] = useState(false);
  const layout = useContext(LocalFlyScreenState).layout;
  const editing = useValue(layout.editing);
  const indicatorOrder = useValue(layout.indicatorOrder);
  const compactStatus = useContext(LocalCompactStatus);
  const narrow = useContext(LocalNarrowStatus);
  const silentFor = useSilentSeconds(vehicleLinks(linksJson)?.contactLost === true);
  const advancedUi = useAdvancedUiShown();
  const powerPage = useHasSetupPage(POWER_SETUP_PAGE);
  const open = (shown: StripDetail) => () => setDetail(shown);
  const rc = rcCell(state);
  const override = overrideCell(state);
  const telemetry = telemetryCell(state);

  const cells: readonly (readonly [string, ReactNode])[] = [
    ['battery', <BatteryCells batteries={batteries} live={live} timeLeft={timeLeft} current={current} onOpen={open(StripDetail.Battery)} />],
    ['flightTime', <FlightTimeSlot />],
    ['gps', gps === null ? null : <InlineCell text={fix === null ? NO_COUNT : satsText(fix, satellites)} colour={fix === null ? null : gpsColour(fix)} icon="ic_satellite_alt" onClick={open(StripDetail.Gps)} />],
    ['rc', rc === null ? null : <InlineCell text={rc.text} colour={rc.lost ? md('error') : null} icon="ic_gamepad" onClick={open(StripDetail.Rc)} />],
    ['rcOverride', override === null ? null : <InlineCell text={override.text} colour={aircast('warning')} onClick={() => void Qgc.invoke(CLEAR_RC_OVERRIDES)} />],
    ['telemetry', telemetry === null ? null : <InlineCell text={telemetry} colour={null} icon="ic_sensors" onClick={open(StripDetail.Telemetry)} />],
    ['links', links === null ? null : <InlineCell text={links.text} colour={links.degraded ? aircast('warning') : null} icon="ic_signal_cellular_alt" onClick={open(StripDetail.Links)} />],
    ['aircastLink', <AircastLinkCell />],
    ['esc', <EscIndicatorCell />],
    ['joystick', <JoystickIndicatorCell />],
    ['remoteId', <RemoteIdIndicatorCell />],
    ['gpsResilience', <GpsResilienceCell />],
    ['rtk', <RtkIndicatorCell status={rtk} />],
    ['gcsBattery', <GcsBatteryCell reading={gcsBattery} />],
    ['gimbal', <GimbalIndicatorCell />],
    ['supportForwarding', <SupportForwardingCell />],
  ];
  const byKey = new Map(cells);
  const ordered = orderedKeys(
    cells.map(([key]) => key),
    indicatorOrder,
  );
  const compact = compactStatus && !editing;
  const keys = compact ? ordered.slice(0, shownStatusCells(narrow)) : ordered;
  const silence = silentFor === null ? null : silenceText(silentFor);
  const battery = detail === StripDetail.Battery;
  const rows = (() => {
    if (detail === StripDetail.Battery) {
      const draw = totalDraw(batteryJson);
      return [...(draw === null ? [] : [{ label: 'Total draw', value: draw, severity: 0 }]), ...batteryDetail(batteryJson)];
    }
    if (detail === StripDetail.Gps) return gpsDetail(gps);
    if (detail === StripDetail.Telemetry) return telemetryDetail(state?.telemetry ?? null);
    if (detail === StripDetail.Rc) return rcDetail(state);
    return linkDetail(vehicleLinks(linksJson), linkNames(linksJson), linksJson === null ? null : optText(linksJson, 'primary'));
  })();

  return (
    <>
      <OpenOnRequest name="status-all" open={() => setAllStatus(true)} />
      <div className={`flex items-center overflow-x-auto ${className ?? ''}`} style={{ ...style, opacity: live ? 1 : 0.45 }}>
        {keys.map((key) => (
          <LayoutWidget key={key} widgetKey={`indicator-${key}`} movable={false}>
            <LeadingGap gap={stripGap(narrow)}>
              {editing ? (
                <TextButton
                  onClick={() => {
                    const moved = movedKey(keys, key, -1);
                    if (moved !== null) layout.saveIndicatorOrder(moved);
                  }}
                >
                  {'‹'}
                </TextButton>
              ) : null}
              {byKey.get(key) ?? null}
              {editing ? (
                <TextButton
                  onClick={() => {
                    const moved = movedKey(keys, key, 1);
                    if (moved !== null) layout.saveIndicatorOrder(moved);
                  }}
                >
                  {'›'}
                </TextButton>
              ) : null}
            </LeadingGap>
          </LayoutWidget>
        ))}
        {compact && !narrow ? (
          <LeadingGap gap={STRIP_GAP}>
            <button type="button" aria-label="All status" onClick={() => setAllStatus(true)} className="flex">
              <MaterialIcon name="keyboard_arrow_right" filled className="rtl:-scale-x-100" />
            </button>
          </LeadingGap>
        ) : null}
      </div>
      {allStatus ? (
        <AircastSheet onDismissRequest={() => setAllStatus(false)}>
          <LocalCompactStatus.Provider value={false}>
            <div className="flex flex-col gap-[14px] px-6 py-2">
              <Text typography="titleLarge">Status</Text>
              {ordered.map((key) => (
                <Fragment key={key}>{byKey.get(key) ?? null}</Fragment>
              ))}
            </div>
          </LocalCompactStatus.Provider>
        </AircastSheet>
      ) : null}
      {detail !== null ? (
        <InstrumentSheet
          title={instrumentTitle(detail)}
          rows={rows}
          headline={battery ? batteryHeadline(batteryJson) : null}
          silence={silence}
          footer={
            battery ? (
              <>
                <TextButton
                  className="mx-3"
                  onClick={() => {
                    setDetail(null);
                    setBatterySettings(true);
                  }}
                >
                  Battery failsafes
                </TextButton>
                <TextButton
                  className="mx-3"
                  onClick={() => {
                    setDetail(null);
                    setBatteryDisplay(true);
                  }}
                >
                  Battery display
                </TextButton>
                {advancedUi && powerPage ? (
                  <TextButton
                    className="mx-3"
                    onClick={() => {
                      setDetail(null);
                      navigation.setupPage.value = POWER_SETUP_PAGE;
                    }}
                  >
                    Power setup
                  </TextButton>
                ) : null}
              </>
            ) : null
          }
          action={
            battery ? (
              batteryReturnOffered(batteryJson) ? <BatteryReturnButton onClosed={() => setDetail(null)} /> : null
            ) : detail === StripDetail.Gps ? (
              <TextButton
                className="mx-3"
                onClick={() => {
                  setDetail(null);
                  setRtkSettings(true);
                }}
              >
                RTK GPS settings
              </TextButton>
            ) : null
          }
          onDismiss={() => setDetail(null)}
        />
      ) : null}
      {rtkSettings ? (
        <AircastSheet onDismissRequest={() => setRtkSettings(false)}>
          <RtkSettingsSheetContent status={rtk} />
        </AircastSheet>
      ) : null}
      {batterySettings ? (
        <AircastSheet onDismissRequest={() => setBatterySettings(false)}>
          <BatterySettingsContent />
        </AircastSheet>
      ) : null}
      {batteryDisplay ? (
        <AircastSheet onDismissRequest={() => setBatteryDisplay(false)}>
          <BatteryDisplaySettings />
        </AircastSheet>
      ) : null}
    </>
  );
};

const BatterySettingsContent = () => {
  const setupJson = useQgcPath(SETUP);
  const wait = indicatorParameterWait(setupJson);
  return wait === null ? <ParameterForm page={BATTERY_SETTINGS_PAGE} /> : <Text className="block px-6 py-4">{wait}</Text>;
};

export type BarTone = { readonly name: 'Fine' | 'Low' | 'ReturnNow' | 'Reserve'; readonly ordinal: number };
export const BarTone = {
  Fine: { name: 'Fine', ordinal: 0 },
  Low: { name: 'Low', ordinal: 1 },
  ReturnNow: { name: 'ReturnNow', ordinal: 2 },
  Reserve: { name: 'Reserve', ordinal: 3 },
} as const satisfies Record<string, BarTone>;
export const BarToneEntries: readonly BarTone[] = [BarTone.Fine, BarTone.Low, BarTone.ReturnNow, BarTone.Reserve];

export const barTone = (headline: BatteryHeadline): BarTone => {
  if ((headline.percent ?? 100) <= headline.reserve) return BarTone.Reserve;
  if (headline.returnNow) return BarTone.ReturnNow;
  return headline.level.ordinal >= BatteryLevel.Warning.ordinal ? BarTone.Low : BarTone.Fine;
};

export const batteryBarDescription = (headline: BatteryHeadline): string =>
  [
    headline.percent === null ? null : `Battery ${roundToInt(headline.percent)}%`,
    headline.returnAt === null ? null : `return home needed at ${roundToInt(headline.returnAt)}%`,
    headline.returnNow ? RETURN_NOW_SPOKEN : null,
  ]
    .filter((part): part is string => part !== null)
    .join(', ');

const BATTERY_BAR_HEIGHT = 3;
const RETURN_MARKER = 12;
const BATTERY_BAR_TRACK_ALPHA = 0.25;
const PERCENT = 100;

export const BatteryReturnBar = ({ className }: { className?: string }) => {
  const batteryJson = useQgcPath(BATTERY_VIEW);
  const stateJson = useQgcPath(FLY_STATE);
  const headline = useMemo(() => batteryHeadline(batteryJson), [batteryJson]);
  const live = useMemo(() => (flyState(stateJson)?.staleNotice ?? '').trim() === '', [stateJson]);
  if (headline === null || headline.percent === null) return null;
  const tone = barTone(headline);
  const fill = (() => {
    if (!live) return md('outline');
    if (tone === BarTone.Fine) return aircast('success');
    if (tone === BarTone.Low) return batteryLevelColour(headline.level) ?? 'transparent';
    return tone === BarTone.ReturnNow ? aircast('warning') : md('error');
  })();
  const track = withAlpha(aircast('outdoorForeground'), BATTERY_BAR_TRACK_ALPHA);
  const at = (share: number) => `${coerceIn(share / PERCENT, 0, 1) * 100}%`;
  return (
    <svg role="img" aria-label={batteryBarDescription(headline)} width="100%" height={RETURN_MARKER} overflow="visible" className={`block ${className ?? ''}`}>
      <rect x={0} y={0} width="100%" height={BATTERY_BAR_HEIGHT} style={{ fill: track }} />
      <rect x={0} y={0} width={at(headline.percent)} height={BATTERY_BAR_HEIGHT} style={{ fill }} />
      <rect x={0} y={0} width={at(headline.reserve)} height={BATTERY_BAR_HEIGHT} style={{ fill: md('error') }} />
      {headline.returnAt !== null ? (
        <>
          <circle cx={at(headline.returnAt)} cy={RETURN_MARKER / 2} r={RETURN_MARKER / 2} style={{ fill: aircast('success') }} />
          <text x={at(headline.returnAt)} y={RETURN_MARKER / 2} textAnchor="middle" dominantBaseline="central" fontSize={8} fontWeight="bold" style={{ fill: aircast('onSuccess') }}>
            H
          </text>
        </>
      ) : null}
    </svg>
  );
};

export const BATTERY_SETTINGS_PAGE = 'Battery Settings';

export const indicatorParameterWait = (setup: Json | null): string | null => {
  if (parametersReady(setup)) return null;
  return setup !== null && optText(setup, 'parametersReason') === 'skipped' ? 'Parameters not available' : 'Waiting for parameters…';
};

const BATTERY_INDICATOR_SETTINGS = 'settings.batteryIndicatorSettings';
export const BATTERY_DISPLAY_FACTS: readonly string[] = ['valueDisplay', 'threshold1', 'threshold2'];

export const batteryDisplayFacts = (facts: readonly Fact[]): readonly Fact[] =>
  BATTERY_DISPLAY_FACTS.flatMap((name) => {
    const fact = facts.find((it) => it.name === name);
    return fact === undefined ? [] : [fact];
  });

const BatteryDisplaySettings = () => {
  const facts = useQgcFacts(BATTERY_INDICATOR_SETTINGS);
  return (
    <div className="flex w-full flex-col pb-6">
      <Text typography="titleMedium" className="block px-6 py-2">
        Battery display
      </Text>
      {batteryDisplayFacts(facts).map((fact) => (
        <FactRow key={fact.path} fact={fact} />
      ))}
    </div>
  );
};

export const CLEAR_RC_OVERRIDES = 'vehicle.clearRcChannelOverrides';
export const POWER_SETUP_PAGE = 'Power';

export type StripDetail = { readonly name: 'Battery' | 'Gps' | 'Links' | 'Telemetry' | 'Rc'; readonly ordinal: number };
export const StripDetail = {
  Battery: { name: 'Battery', ordinal: 0 },
  Gps: { name: 'Gps', ordinal: 1 },
  Links: { name: 'Links', ordinal: 2 },
  Telemetry: { name: 'Telemetry', ordinal: 3 },
  Rc: { name: 'Rc', ordinal: 4 },
} as const satisfies Record<string, StripDetail>;
export const StripDetailEntries: readonly StripDetail[] = [StripDetail.Battery, StripDetail.Gps, StripDetail.Links, StripDetail.Telemetry, StripDetail.Rc];

export const rcDetail = (state: FlyState | null): readonly DetailRow[] =>
  state === null || state.rcSignalText.trim() === '' ? [] : [{ label: 'RSSI', value: state.rcSignalText, severity: 0 }];

export const instrumentTitle = (instrument: StripDetail): string => {
  switch (instrument.name) {
    case 'Battery':
      return 'Battery';
    case 'Gps':
      return 'Vehicle GPS status';
    case 'Links':
      return 'Links to this aircraft';
    case 'Telemetry':
      return 'Telemetry RSSI status';
    case 'Rc':
      return 'RC RSSI status';
  }
};

type InstrumentSheetProps = {
  title: string;
  rows: readonly DetailRow[];
  headline?: BatteryHeadline | null;
  silence?: string | null;
  action?: ReactNode;
  footer?: ReactNode;
  onDismiss: () => void;
};

const InstrumentSheet = ({ title, rows, headline = null, silence = null, action = null, footer = null, onDismiss }: InstrumentSheetProps) => {
  const stale = silence !== null;
  const muted = md('onSurfaceVariant');
  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <Text typography="titleMedium" className="block px-6 py-2">
        {title}
      </Text>
      {silence !== null ? (
        <Text typography="bodyMedium" fontWeight="bold" color={md('error')} className="block px-6 py-1">
          {silence}
        </Text>
      ) : null}
      {headline !== null ? (
        <div className="flex flex-col px-6 py-1">
          <Text typography="headlineMedium" fontWeight="bold" color={stale ? muted : headlineColour(headline)}>
            {headline.text}
          </Text>
          {headline.detail.trim() !== '' ? (
            <Text typography="bodyMedium" color={muted}>
              {headline.detail}
            </Text>
          ) : null}
          {headline.margin.trim() !== '' ? (
            <Text typography="bodyMedium" color={stale ? muted : md('onSurface')}>
              {headline.margin}
            </Text>
          ) : null}
        </div>
      ) : null}
      {action === null ? null : <div className="flex flex-col items-start">{action}</div>}
      {rows.length === 0 ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block px-6 py-3">
          The vehicle has not reported anything else about this yet.
        </Text>
      ) : (
        rows.map((row, at) => (
          <ListItem
            key={at}
            headlineContent={<span style={{ color: row.severity === SEVERITY_SECONDARY ? md('onSurfaceVariant') : undefined }}>{row.label}</span>}
            trailingContent={<span style={{ color: stale ? muted : severityColour(row.severity) }}>{row.value}</span>}
          />
        ))
      )}
      {footer === null ? null : <div className="flex flex-col items-start">{footer}</div>}
      <FootNote text="Readings come from the aircraft and stop updating when it stops answering." />
    </AircastSheet>
  );
};

const headlineColour = (headline: BatteryHeadline): string =>
  headline.severity > 0 ? severityColour(headline.severity) : (batteryLevelColour(headline.level) ?? md('onSurface'));

const severityColour = (severity: number): string => {
  if (severity >= 2) return md('error');
  if (severity === 1) return aircast('warning');
  return severity === SEVERITY_SECONDARY ? md('onSurfaceVariant') : md('onSurface');
};

const InlineCell = ({ text, colour, icon = null, onClick = null }: { text: string; colour: string | null; icon?: DrawableName | null; onClick?: (() => void) | null }) => {
  const compact = useContext(LocalCompactStatus);
  const tint = colour ?? md('onSurfaceVariant');
  const shown = compact && icon !== null ? compactStatusText(text) : text;
  const body = (
    <>
      {icon !== null ? <Drawable name={icon} size={18} style={{ color: tint }} /> : null}
      {shown !== '' ? (
        <Text typography="labelMedium" color={tint} maxLines={1}>
          {shown}
        </Text>
      ) : null}
    </>
  );
  return onClick === null ? (
    <div className="flex items-center gap-1">{body}</div>
  ) : (
    <button type="button" aria-label={text} onClick={onClick} className="flex items-center gap-1">
      {body}
    </button>
  );
};

export const LocalCompactStatus = createContext(false);

const BATTERY_RING_SIZE = 30;
const BATTERY_RING_STROKE = 3;
const BATTERY_RING_TRACK_ALPHA = 0.25;

export const batteryPercent = (text: string): number | null => {
  const reading = compactStatusText(text);
  const digits = reading.endsWith('%') ? reading.slice(0, -1) : reading;
  return /^[+-]?\d+$/.test(digits) ? coerceIn(Number(digits), 0, 100) : null;
};

export const batteryPercentNow = async (): Promise<number | null> => {
  const first = batteryReadings(await Qgc.get(BATTERY_VIEW))[0];
  return first === undefined ? null : batteryPercent(first.text);
};

export const flightTimeNow = async (): Promise<number | null> => flightTimeSeconds(await Qgc.get(VEHICLE_FLIGHT_TIME));

export const flightTimeText = (seconds: number | null): string => {
  const whole = seconds !== null && Number.isFinite(seconds) && seconds > 0 ? Math.trunc(seconds) : 0;
  return format('%02d\'%02d"', Math.trunc(whole / 60), whole % 60);
};

const FLIGHT_TIME_POLL_MS = 1000;

export const flightTimeSeconds = (view: Json | null): number | null => (typeof view?.value === 'number' ? view.value : null);

const FlightTimeCell = () => {
  const [seconds, setSeconds] = useState<number | null>(null);
  useTicker(FLIGHT_TIME_POLL_MS, [], async () => setSeconds(flightTimeSeconds(await Qgc.get(VEHICLE_FLIGHT_TIME))));
  return (
    <span role="timer" aria-label="Flight time" className="text-title-small tabular-nums" style={{ color: aircast('outdoorForeground') }}>
      {flightTimeText(seconds)}
    </span>
  );
};

const BatteryRing = ({ text, colour, count, onClick }: { text: string; colour: string | null; count: string; onClick: () => void }) => {
  const percent = batteryPercent(text);
  if (percent === null) return <InlineCell text={text} colour={colour} icon="ic_battery_5_bar" onClick={onClick} />;
  const ring = colour ?? aircast('success');
  const track = withAlpha(colour ?? aircast('outdoorForeground'), BATTERY_RING_TRACK_ALPHA);
  const centre = BATTERY_RING_SIZE / 2;
  const radius = (BATTERY_RING_SIZE - BATTERY_RING_STROKE) / 2;
  const length = 2 * Math.PI * radius;
  return (
    <button
      type="button"
      aria-label={count === '' ? `Battery ${percent}%` : `Lowest of ${count} batteries ${percent}%`}
      aria-description="Battery"
      onClick={onClick}
      className="relative flex"
      style={{ paddingInlineEnd: count === '' ? 0 : BATTERY_COUNT_ROOM }}
    >
      <span className="relative grid place-items-center" style={{ width: BATTERY_RING_SIZE, height: BATTERY_RING_SIZE }}>
        <svg className="absolute inset-0" width={BATTERY_RING_SIZE} height={BATTERY_RING_SIZE} viewBox={`0 0 ${BATTERY_RING_SIZE} ${BATTERY_RING_SIZE}`} aria-hidden>
          <circle cx={centre} cy={centre} r={radius} fill="none" strokeWidth={BATTERY_RING_STROKE} style={{ stroke: track }} />
          <circle
            cx={centre}
            cy={centre}
            r={radius}
            fill="none"
            strokeWidth={BATTERY_RING_STROKE}
            strokeLinecap="round"
            strokeDasharray={`${(length * percent) / 100} ${length}`}
            transform={`rotate(-90 ${centre} ${centre})`}
            style={{ stroke: ring }}
          />
        </svg>
        <span className="relative text-label-small tabular-nums" style={{ color: colour ?? aircast('outdoorForeground') }}>
          {percent}
        </span>
      </span>
      {count !== '' ? (
        <span className="absolute end-0 bottom-0 text-label-small" style={{ fontSize: BATTERY_COUNT_SIZE, color: aircast('outdoorForeground') }}>
          {count}
        </span>
      ) : null}
    </button>
  );
};

const BATTERY_COUNT_SIZE = 9;
const BATTERY_COUNT_ROOM = 10;
const BATTERY_GAP = 6;

export const COMPACT_STATUS_CELLS = 4;

export const compactStatusText = (text: string): string => {
  const cut = text.indexOf(' · ');
  const head = cut < 0 ? text : text.slice(0, cut);
  return head.split(' ').findLast((token) => /\p{Nd}/u.test(token)) ?? '';
};

export type OverrideCell = { text: string };

export const overrideCell = (state: FlyState | null): OverrideCell | null => (state?.rcOverride === true ? { text: 'RC override' } : null);

export const telemetryCell = (state: FlyState | null): string | null => (state?.telemetry ? `${state.telemetry.localRssiDbm} dBm` : null);

const detailRow = (label: string, value: string): DetailRow => ({ label, value, severity: 0 });

export const telemetryDetail = (link: TelemetryLink | null): readonly DetailRow[] =>
  link === null
    ? []
    : [
        detailRow('Local RSSI:', `${link.localRssiDbm} dBm`),
        link.remoteRssiDbm === null ? null : detailRow('Remote RSSI:', `${link.remoteRssiDbm} dBm`),
        link.receiveErrors === null ? null : detailRow('RX Errors:', `${link.receiveErrors}`),
        link.errorsFixed === null ? null : detailRow('Errors Fixed:', `${link.errorsFixed}`),
        link.txBuffer === null ? null : detailRow('TX Buffer:', `${link.txBuffer}`),
        link.localNoise === null ? null : detailRow('Local Noise:', `${link.localNoise}`),
        link.remoteNoise === null ? null : detailRow('Remote Noise:', `${link.remoteNoise}`),
      ].filter((it): it is DetailRow => it !== null);

const BatteryReturnButton = ({ onClosed }: { onClosed: () => void }) => {
  const actionsJson = useQgcPath(GUIDED_ACTIONS);
  const rtl = useMemo(() => guidedOffers(actionsJson).get('rtl') ?? null, [actionsJson]);
  const [confirming, setConfirming] = useState(false);
  if (rtl?.shown !== true) return null;
  return (
    <>
      <TextButton enabled={rtl.ready} onClick={() => setConfirming(true)} className="mx-3" contentColor={md('error')}>
        Return
      </TextButton>
      {confirming ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title="Return"
          text="Return to the launch position of the vehicle"
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(false);
                onClosed();
                void VehicleCommands.returnToLaunch(false);
              }}
            >
              Return
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};
