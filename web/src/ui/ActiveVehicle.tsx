import { type ReactNode, useContext, useEffect, useMemo, useState } from 'react';
import { type Json, has, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { settingControl, truthy, useQgcDerived, useQgcPath, useQgcValue } from '@/bridge/QgcState';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { coerceIn, delay } from '@/kotlin/Std';
import { Checkbox, CircularProgressIndicator, DropdownMenu, DropdownMenuItem, HorizontalDivider, LinearProgressIndicator, ListItem, Surface, Switch, Text, TextButton } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { useSilentSeconds } from '@/map/Silence';
import { gotoLocation } from '@/map/GotoMarker';
import {
  CHOOSER_TITLE,
  FleetBridge,
  VEHICLES_VIEW,
  VehicleBridge,
  type VehicleChoice,
  type VehicleChoices,
  activeVehicleTitle,
  linkDistinguishes,
  lostVehicles,
  lostVehiclesText,
  vehicleChoiceLine,
  vehicleChoices,
  vehicleFlightModePath,
  vehicleTelemetryLine,
} from '@/map/VehiclePicker';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from './AppNavigation';
import { ConfirmTrack } from './ConfirmTrack';
import { StopPropagation, activate, useFlyIsPortrait } from './FlyShell';
import { LocalFlyScreenState, OpenOnRequest } from './FlyScreenState';
import { ChipTone, FLY_STATE, type FlyState, chipTone, flyState, offlineMainStatus, vehicleSubtitle } from './FlyState';
import { LOST_LINK_HINT, SIGNAL_LOST, signalLostTitle, silenceText } from './InstrumentDetail';
import { MAP_CLICK_PATH } from './MapClickMenu';
import { OFFLINE_SHEET, OFFLINE_STATUS_VIEW, type OfflineLink, OfflineStatusSheet, offlineStatus } from './OfflineStatusSheet';
import {
  ALLOW_TAKEOVER_SETTING,
  type ControlStation,
  GCS_SYSTEM_ID_PATH,
  OPERATOR_CONTROL_VIEW,
  REQUEST_TIMEOUT_PATH,
  acquireEnabled,
  acquireLabel,
  allowTakeoverEditable,
  askForControl,
  changeTakeover,
  controlIsElsewhere,
  controlLine,
  controlSectionTitle,
  controlStation,
  controlWaitLine,
  holderLine,
  inControlLine,
  requestSentLabel,
  requestTimeoutEditable,
  saveAllowTakeover,
  takeoverChangeable,
  takeoverLine,
} from './OperatorControl';
import { factFromControl } from './ParameterForm';
import { CONNECTIONS_PAGE, FactRow, factSubtitle } from './SettingsScreen';
import { FootNote } from './SetupRow';
import { VEHICLE_LINKS, vehicleLinks } from './VehicleLinks';
import { VehicleMessagesSheet, WARNINGS, armingBlocker, armingChecks, chipBlocker } from './VehicleMessages';
import { VehicleStatusSheet } from './VehicleStatusSheet';
import { FlightModeMenu, GuidedAction } from './VehicleUi';
import { TAP } from '@/kotlin/Platform';

const STATUS_BAR_SCRIM_ALPHA = 0.55;

const statusTitleColour = (tone: ChipTone, linkFailed: boolean): string => {
  if (linkFailed || tone === ChipTone.Error) return md('error');
  if (tone === ChipTone.Warning) return aircast('warning');
  if (tone === ChipTone.Success) return aircast('success');
  return aircast('outdoorForeground');
};

const noteColour = (tone: ChipTone): string => {
  if (tone === ChipTone.Error) return md('error');
  if (tone === ChipTone.Warning) return aircast('warning');
  if (tone === ChipTone.Success) return aircast('success');
  return aircast('outdoorForeground');
};

const MIDDLE_DOT = ' · ';

export const osdModeText = (title: string): string => {
  const at = title.indexOf(MIDDLE_DOT);
  return at < 0 ? title : title.slice(0, at);
};

export const osdStatusNote = (title: string): string | null => {
  const at = title.indexOf(MIDDLE_DOT);
  const note = at < 0 ? '' : title.slice(at + MIDDLE_DOT.length);
  return note.trim() === '' ? null : note;
};

export type MvAction = {
  readonly id: string;
  readonly title: string;
  readonly confirmTitle: string;
  readonly prompt: string;
  readonly offer: string;
  readonly reason: string;
  readonly ready: boolean;
};

export const MvAction = (fields: Omit<MvAction, 'ready'>): MvAction => ({ ...fields, ready: fields.offer === 'ready' });

export const mvActions = (view: Json | null): readonly MvAction[] =>
  jsonObjects(optJSONArray(view, 'actions'))
    .filter((entry) => optText(entry, 'id').trim() !== '')
    .map((entry) => {
      const confirmTitle = optText(entry, 'confirmTitle');
      return MvAction({
        id: optText(entry, 'id'),
        title: optText(entry, 'title'),
        confirmTitle: confirmTitle.trim() === '' ? optText(entry, 'title') : confirmTitle,
        prompt: optText(entry, 'prompt'),
        offer: optText(entry, 'offer'),
        reason: optText(entry, 'reason'),
      });
    });

export const mvReasonFor = (action: MvAction): string | null => (action.reason.trim() !== '' && !action.ready ? action.reason : null);

export const STATUS_SETTINGS_PAGE = 'Status Settings';
export const CLOSE_VEHICLE = 'vehicle.closeVehicle';

export const readinessSubtitle = (state: FlyState | null, blocker: string | null, failing: number): string | null =>
  blocker === null
    ? null
    : [state?.mode.trim() ? state.mode : null, chipBlocker(blocker) + (failing > 1 ? ` +${failing - 1}` : '')].filter((it): it is string => it !== null).join(MIDDLE_DOT);

export const VEHICLE_CONNECT_COMPLETE = 'vehicle.initialConnectComplete';
export const VEHICLE_LOAD_PROGRESS = 'vehicle.loadProgress';

export const loadingProgress = (complete: unknown, progress: unknown): number | null =>
  typeof complete === 'boolean' && !complete ? coerceIn(typeof progress === 'number' ? progress : 0, 0, 1) : null;

export const useRememberVehicleLoading = (): number | null => {
  const complete = useQgcValue(VEHICLE_CONNECT_COMPLETE);
  const progress = useQgcValue(VEHICLE_LOAD_PROGRESS);
  return loadingProgress(complete, progress);
};

export const VehicleStateChip = ({ className }: { className?: string }) => {
  const navigation = useContext(LocalAppNavigation);
  const flyScreen = useContext(LocalFlyScreenState);
  const flyJson = useQgcPath(FLY_STATE);
  const fly = useMemo(() => flyState(flyJson), [flyJson]);
  const vehiclesJson = useQgcPath(VEHICLES_VIEW);
  const choices = useMemo(() => vehicleChoices(vehiclesJson), [vehiclesJson]);
  const controlJson = useQgcPath(OPERATOR_CONTROL_VIEW);
  const station = useMemo(() => controlStation(controlJson), [controlJson]);
  const taken = controlIsElsewhere(station);
  const lost = fly?.contactLost === true;
  const silentFor = useSilentSeconds(lost);
  const linksJson = useQgcPath(VEHICLE_LINKS);
  const failsafe = useMemo(() => vehicleLinks(linksJson)?.failsafe ?? null, [linksJson]);
  const mapClickJson = useQgcPath(MAP_CLICK_PATH);
  const gotoProgress = useMemo(() => gotoLocation(mapClickJson)?.progressText ?? '', [mapClickJson]);
  const [lostMenu, setLostMenu] = useState(false);
  const offlineJson = useQgcPath(OFFLINE_STATUS_VIEW);
  const warningsJson = useQgcPath(WARNINGS);
  const blocker = useMemo(() => {
    const found = armingBlocker(warningsJson);
    return found !== null && fly?.connected === true && !fly.armed ? found : null;
  }, [warningsJson, fly]);
  const failing = useMemo(() => (armingChecks(warningsJson) ?? []).length, [warningsJson]);
  const offlineMain = useMemo(() => offlineMainStatus(offlineJson), [offlineJson]);
  const subtitle = readinessSubtitle(fly, blocker, failing) ?? vehicleSubtitle(fly, offlineMain);
  const [why, setWhy] = useState(false);
  const [picking, setPicking] = useState(false);
  const panelJson = useQgcPath(MULTI_VEHICLE_PANEL);
  const panelEnabled = multiVehiclePanelEnabled(panelJson);
  const panelFact = useQgcPath(MULTI_VEHICLE_PANEL_SETTING);
  const [offline, setOffline] = useState(false);
  const [statusSettings, setStatusSettings] = useState(false);
  const [modeMenu, setModeMenu] = useState(false);
  const offlineNow = useMemo(() => offlineStatus(offlineJson), [offlineJson]);
  const disconnected = fly?.connected !== true;
  const [refusal, setRefusal] = useState<string | null>(null);
  const narrow = useFlyIsPortrait();
  const pendingMode = useValue(flyScreen.pendingMode);
  const loading = useRememberVehicleLoading();

  useEffect(() => {
    if (!disconnected) setOffline(false);
  }, [disconnected]);

  const tone = blocker !== null ? ChipTone.Error : chipTone(fly, lost);
  const titleColour = statusTitleColour(tone, disconnected && (offlineNow?.links ?? []).some((it: OfflineLink) => it.failed));
  const searching = disconnected && offlineNow?.busy === true;
  const title = lost ? signalLostTitle(silentFor, failsafe, narrow) : gotoProgress !== '' && !choices.ambiguous ? gotoProgress : activeVehicleTitle(choices, subtitle);
  const note = narrow ? null : osdStatusNote(title);
  const alarm = lostVehiclesText(lostVehicles(choices)) ?? (taken ? controlLine(station) : null);

  const chipClick = () => {
    if (lost) setLostMenu(true);
    else if (choices.ambiguous || taken) setPicking(true);
    else if (disconnected) setOffline(true);
    else if (blocker !== null) setWhy(true);
    else setModeMenu(true);
  };

  const trailing: ReactNode =
    pendingMode !== null ? (
      <CircularProgressIndicator size={16} strokeWidth={2} />
    ) : choices.ambiguous || taken ? (
      <Drawable name={alarm === null ? 'ic_arrow_drop_down' : 'ic_warning'} label={alarm ?? 'Choose which vehicle to fly'} />
    ) : disconnected ? (
      <Drawable name="ic_chevron_right" label="How to connect" style={{ color: titleColour }} />
    ) : (
      <StopPropagation className="flex">
        <button type="button" className="flex" aria-label={lost ? SIGNAL_LOST : 'Change flight mode'} onClick={() => (lost ? setLostMenu(true) : setModeMenu(true))}>
          <Drawable name="ic_arrow_drop_down" />
        </button>
      </StopPropagation>
    );

  return (
    <>
      <OpenOnRequest name="status" open={() => setStatusSettings(true)} />
      <OpenOnRequest name="modes" open={() => setModeMenu(true)} />
      <OpenOnRequest
        name={OFFLINE_SHEET}
        open={() => {
          if (fly?.connected !== true) setOffline(true);
        }}
      />
      <div className={`relative ${className ?? ''}`}>
        <FlightModeMenu expanded={modeMenu && !disconnected} onDismiss={() => setModeMenu(false)} onStatus={() => setStatusSettings(true)} />
        <div
          role="button"
          tabIndex={0}
          className="rounded-shape-sm outline-none focus-visible:outline-2 focus-visible:outline-[var(--md-primary)]"
          style={{ color: aircast('outdoorForeground') }}
          onClick={chipClick}
          onKeyDown={activate(chipClick)}
        >
          <div className="flex min-h-11 items-center gap-2 px-3 py-1.5">
            {searching ? (
              <span className="flex size-6 shrink-0 items-center justify-center">
                <CircularProgressIndicator size={18} strokeWidth={2} color={titleColour} label="Searching" />
              </span>
            ) : (
              <Drawable name="ic_flight" size={24} className="shrink-0" style={{ color: titleColour }} />
            )}
            <Text typography="titleMedium" color={titleColour} maxLines={1} ellipsis className="min-w-0 shrink">
              {osdModeText(title)}
            </Text>
            {note !== null ? (
              <StopPropagation className="flex min-w-0 shrink">
                <Surface
                  onClick={() => (blocker !== null ? setWhy(true) : setStatusSettings(true))}
                  shape="small"
                  color={withAlpha(Color.Black, STATUS_BAR_SCRIM_ALPHA)}
                  contentColor={noteColour(tone)}
                  className="min-w-0 px-[14px] py-2"
                >
                  <Text typography="labelLarge" maxLines={1} ellipsis className="block">{note}</Text>
                </Surface>
              </StopPropagation>
            ) : null}
            {trailing}
            {loading !== null && !lost ? (
              <div className="flex flex-col pl-2">
                <Text typography="labelSmall">Loading vehicle</Text>
                <div className="w-[72px]">
                  <LinearProgressIndicator progress={loading} />
                </div>
              </div>
            ) : null}
          </div>
        </div>
        <DropdownMenu expanded={lostMenu && lost} onDismissRequest={() => setLostMenu(false)}>
          <DropdownMenuItem text={<Text typography="labelLarge" color={md('onSurfaceVariant')}>{silentFor !== null ? silenceText(silentFor) : SIGNAL_LOST}</Text>} onClick={() => undefined} enabled={false} />
          <DropdownMenuItem text={<Text typography="bodySmall" className="block max-w-[280px]">{LOST_LINK_HINT}</Text>} onClick={() => undefined} enabled={false} />
          <HorizontalDivider />
          <DropdownMenuItem
            text={<Text typography="labelLarge" color={md('error')}>Disconnect</Text>}
            onClick={() => {
              setLostMenu(false);
              void (async () => {
                flyScreen.refusal.value = await Qgc.refusalOf(CLOSE_VEHICLE);
              })();
            }}
          />
        </DropdownMenu>
      </div>
      {offline ? <OfflineStatusSheet onDismiss={() => setOffline(false)} /> : null}
      {statusSettings && !disconnected ? (
        <VehicleStatusSheet
          onMessages={() => {
            setStatusSettings(false);
            setWhy(true);
          }}
          onDismiss={() => setStatusSettings(false)}
        />
      ) : null}
      {why ? <VehicleMessagesSheet onDismiss={() => setWhy(false)} /> : null}
      {picking ? (
        <AircastSheet onDismissRequest={() => setPicking(false)}>
          <Text typography="titleMedium" className="block px-sheet py-2">{lostVehiclesText(lostVehicles(choices)) ?? CHOOSER_TITLE}</Text>
          <VehicleRows choices={choices} selectable={panelEnabled} onRefusal={setRefusal} onSwitched={() => setPicking(false)} />
          <ListItem
            headlineContent={<Text typography="bodyLarge" color={md('primary')}>Connect another vehicle</Text>}
            leadingContent={<Drawable name="ic_add" style={{ color: md('primary') }} />}
            onClick={() => {
              setPicking(false);
              navigation.settingsPage.value = CONNECTIONS_PAGE;
            }}
          />
          {refusal !== null ? <Text typography="bodySmall" color={md('error')} className="block px-sheet py-3">{refusal}</Text> : null}
          {panelToggleShown(panelFact) ? (
            <ListItem
              headlineContent="Enable multi-vehicle panel"
              trailingContent={
                <Switch
                  checked={panelEnabled}
                  onCheckedChange={(wanted) =>
                    void (async () => {
                      setRefusal(await Qgc.writeRefusal(MULTI_VEHICLE_PANEL_SETTING, wanted));
                    })()
                  }
                />
              }
            />
          ) : null}
          {!disconnected && !taken ? (
            <div className="flex px-4">
              <TextButton
                onClick={() => {
                  setPicking(false);
                  setModeMenu(true);
                }}
              >
                Flight mode
              </TextButton>
              <TextButton
                onClick={() => {
                  setPicking(false);
                  setStatusSettings(true);
                }}
              >
                Vehicle status
              </TextButton>
            </div>
          ) : null}
          <ControlHolderNote station={station} vehicleId={activeVehicleId(vehiclesJson)} onRefusal={setRefusal} />
          <FootNote text={`${TAP} a name to fly that aircraft. Arm, Takeoff and every action on the flight screen go to the one you pick.`} />
          {panelEnabled ? <FleetControls view={vehiclesJson} choices={choices} onRefusal={setRefusal} /> : null}
        </AircastSheet>
      ) : null}
    </>
  );
};

type FleetControlsProps = { view: Json | null; choices: VehicleChoices; onRefusal: (message: string | null) => void };

const FleetControls = (props: FleetControlsProps) => (props.choices.ambiguous ? <AmbiguousFleetControls {...props} /> : null);

const AmbiguousFleetControls = ({ view, choices, onRefusal }: FleetControlsProps) => {
  const actions = useMemo(() => mvActions(view), [view]);
  const [confirming, setConfirming] = useState<MvAction | null>(null);
  return (
    <>
      <Text typography="titleSmall" className="block px-sheet py-2">{fleetHeading(selectedIds(choices))}</Text>
      <div className="flex items-center px-4">
        <TextButton enabled={choices.canSelectAll} onClick={() => void FleetBridge.selectAll(choices)}>Select all</TextButton>
        <TextButton enabled={choices.canDeselectAll} onClick={() => void FleetBridge.deselectAll()}>Deselect all</TextButton>
      </div>
      <Text typography="titleSmall" className="block px-sheet py-2">Multi vehicle actions</Text>
      {actions.map((action) => (
        <div key={action.id}>
          <ListItem
            headlineContent={<Text typography="bodyLarge" color={action.ready ? md('onSurface') : md('onSurfaceVariant')}>{action.title}</Text>}
            supportingContent={confirming?.id !== action.id ? fleetActionLine(action) : undefined}
            onClick={action.ready ? () => setConfirming(action) : undefined}
          />
          {confirming?.id === action.id ? (
            <ConfirmTrack
              action={GuidedAction({
                name: action.confirmTitle,
                confirm: action.prompt,
                destructive: fleetIsDestructive(action),
                run: () => {
                  const confirmed: ReadonlySet<number> = new Set(choices.choices.filter((it) => it.selected).map((it) => it.id));
                  void (async () => {
                    const sent = await FleetBridge.command(action.id, confirmed);
                    onRefusal(sent ? null : `${action.title} did not reach every selected aircraft.`);
                  })();
                },
              })}
              onSent={() => setConfirming(null)}
              onCancel={() => setConfirming(null)}
              className="px-sheet py-2"
            />
          ) : null}
        </div>
      ))}
    </>
  );
};

const VehicleModeMenu = ({ choice, onRefusal }: { choice: VehicleChoice; onRefusal: (message: string | null) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <StopPropagation className="relative">
      <TextButton onClick={() => setOpen(true)}>Mode</TextButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {choice.flightModes.map((mode) => (
          <DropdownMenuItem
            key={mode}
            text={mode}
            onClick={() => {
              setOpen(false);
              void (async () => {
                onRefusal(await Qgc.writeForVehicleRefusal(vehicleFlightModePath(choice), mode, choice.id));
              })();
            }}
          />
        ))}
      </DropdownMenu>
    </StopPropagation>
  );
};

export const fleetPanelShown = (vehicleCount: number, panelEnabled: boolean): boolean => vehicleCount >= 2 && panelEnabled;

export const FleetPanel = ({ className }: { className?: string }) => {
  const vehiclesJson = useQgcPath(VEHICLES_VIEW);
  const choices = useMemo(() => vehicleChoices(vehiclesJson), [vehiclesJson]);
  const panelJson = useQgcPath(MULTI_VEHICLE_PANEL);
  const [refusal, setRefusal] = useState<string | null>(null);
  if (!fleetPanelShown(choices.choices.length, multiVehiclePanelEnabled(panelJson))) return null;
  return (
    <div className={`flex flex-col ${className ?? ''}`}>
      <VehicleRows choices={choices} selectable onRefusal={setRefusal} onSwitched={() => undefined} />
      <FleetControls view={vehiclesJson} choices={choices} onRefusal={setRefusal} />
      {refusal !== null ? <Text typography="bodySmall" color={md('error')} className="block px-sheet py-2">{refusal}</Text> : null}
    </div>
  );
};

const VehicleRows = ({ choices, selectable, onRefusal, onSwitched }: { choices: VehicleChoices; selectable: boolean; onRefusal: (message: string | null) => void; onSwitched: () => void }) => {
  const distinguishes = linkDistinguishes(choices.choices);
  return (
    <>
      {choices.choices.map((choice) => {
        const pick = () =>
          void (async () => {
            const switched = await VehicleBridge.askFor(choice.id);
            onRefusal(switched ? null : VehicleBridge.lastRefusal ?? 'That vehicle did not take control.');
            if (switched) onSwitched();
          })();
        const telemetry = vehicleTelemetryLine(choice);
        return (
          <div key={choice.id}>
            <div
              role="button"
              tabIndex={choice.active ? -1 : 0}
              aria-disabled={choice.active}
              className={`flex min-h-[72px] w-full items-center gap-4 py-2 pl-4 pr-6 outline-none focus-visible:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)] ${choice.active ? '' : 'cursor-pointer hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]'}`}
              onClick={() => {
                if (!choice.active) pick();
              }}
              onKeyDown={activate(() => {
                if (!choice.active) pick();
              })}
            >
              {selectable ? (
                <StopPropagation className="flex">
                  <Checkbox checked={choice.selected} onCheckedChange={(wanted) => void FleetBridge.setSelected(choice.id, wanted)} />
                </StopPropagation>
              ) : null}
              <div className="flex min-w-0 flex-1 flex-col">
                <Text typography="bodyLarge" maxLines={1} ellipsis>{choice.name}</Text>
                <Text typography="bodyMedium" color={md('onSurfaceVariant')} maxLines={1} ellipsis>{vehicleChoiceLine(choice, distinguishes)}</Text>
                {telemetry !== null ? <Text typography="bodySmall" color={md('onSurfaceVariant')} maxLines={1} ellipsis>{telemetry}</Text> : null}
              </div>
              <div className="flex items-center gap-2">
                {choice.flightModes.length > 0 && choice.index >= 0 ? <VehicleModeMenu choice={choice} onRefusal={onRefusal} /> : null}
                <VehicleRowCompass heading={choice.heading} armed={choice.armed} />
                {choice.active ? <MaterialIcon name="check" filled label="Flying this one" /> : null}
              </div>
            </div>
            <HorizontalDivider />
          </div>
        );
      })}
    </>
  );
};

export const selectedIds = (choices: VehicleChoices): readonly number[] =>
  choices.choices
    .filter((it) => it.selected)
    .map((it) => it.id)
    .toSorted((a, b) => a - b);

export const fleetHeading = (selectedIds: readonly number[]): string => `Vehicles Selected: ${selectedIds.length === 0 ? '-' : selectedIds.join(', ')}`;

export const fleetActionLine = (action: MvAction): string => mvReasonFor(action) ?? (action.prompt.trim() === '' ? action.title : action.prompt);

export const fleetIsDestructive = (action: MvAction): boolean => action.id !== 'mvPause';

const ControlHolderNote = ({ station, vehicleId, onRefusal }: { station: ControlStation | null; vehicleId: number | null; onRefusal: (message: string | null) => void }) => {
  if (station === null) return null;
  if (station.inControl === true) return <InControlNote holder={station} onRefusal={onRefusal} />;
  const line = holderLine(station);
  return line === null ? null : <HolderNote holder={station} line={line} vehicleId={vehicleId} onRefusal={onRefusal} />;
};

const HolderNote = ({ holder, line, vehicleId, onRefusal }: { holder: ControlStation; line: string; vehicleId: number | null; onRefusal: (message: string | null) => void }) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const deadlineKey = vehicleId ?? 0;
  const deadlines = useValue(flyScreen.controlRequestDeadlines);
  const requestEndsAt = deadlines.get(deadlineKey) ?? null;
  useEffect(() => {
    if (holder.takeoverAllowed === true) flyScreen.controlRequestDeadlines.remove(deadlineKey);
  }, [holder.takeoverAllowed]);
  const takeover = takeoverLine(holder);
  const waiting = requestEndsAt === null ? controlWaitLine(holder) : null;
  const label = acquireLabel(holder);
  return (
    <>
      <Text typography="bodyMedium" className="block px-sheet py-2">{line}</Text>
      {takeover !== null ? <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-sheet">{takeover}</Text> : null}
      <ControlSectionTitle holder={holder} />
      {requestEndsAt !== null ? <RequestCountdown endsAt={requestEndsAt} onDone={() => flyScreen.controlRequestDeadlines.remove(deadlineKey)} /> : null}
      <AllowTakeoverBox holder={holder} onRefusal={onRefusal} />
      {waiting !== null ? <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-sheet py-2">{waiting}</Text> : null}
      {label !== null ? (
        <TextButton
          enabled={acquireEnabled(holder, requestEndsAt !== null)}
          onClick={() =>
            void (async () => {
              const ask = await askForControl(holder);
              onRefusal(ask.refusal);
              if (ask.refusal === null && ask.timeoutSeconds > 0) flyScreen.controlRequestDeadlines.set(deadlineKey, Date.now() + ask.timeoutSeconds * 1000);
            })()
          }
          className="mx-4"
        >
          {label}
        </TextButton>
      ) : null}
      {requestTimeoutEditable(holder) ? <SettingFactRow path={REQUEST_TIMEOUT_PATH} title="Request timeout (sec)" /> : null}
      <SettingFactRow path={GCS_SYSTEM_ID_PATH} title="This GCS MAVLink system ID" />
    </>
  );
};

const ControlSectionTitle = ({ holder }: { holder: ControlStation }) => {
  const title = controlSectionTitle(holder);
  return title === null ? null : <Text typography="titleSmall" className="block px-sheet py-2">{title}</Text>;
};

const SettingFactRow = ({ path, title }: { path: string; title: string }) => {
  const control = useQgcPath(settingControl(path));
  const fact = useMemo(() => (control !== null && optText(control, 'kind') === 'object' ? factFromControl(control) : null), [control]);
  return fact === null ? null : <FactRow fact={fact} title={title} subtitle={factSubtitle(fact)} fieldModifier="w-full px-sheet py-2" />;
};

const RequestCountdown = ({ endsAt, onDone }: { endsAt: number; onDone: () => void }) => {
  const [now, setNow] = useKeyedState([endsAt], () => Date.now());
  useLaunchedEffect([endsAt], async (scope) => {
    const tick = async (at: number): Promise<boolean> => {
      if (at >= endsAt) return true;
      await delay(COUNTDOWN_TICK_MS);
      if (!scope.isActive()) return false;
      const next = Date.now();
      setNow(next);
      return tick(next);
    };
    if (await tick(now)) onDone();
  });
  return <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-sheet py-2">{requestSentLabel(endsAt - now)}</Text>;
};

const COUNTDOWN_TICK_MS = 100;

const AllowTakeoverBox = ({ holder, onRefusal, onRead = () => undefined }: { holder: ControlStation; onRefusal: (message: string | null) => void; onRead?: (allow: boolean | null) => void }) => {
  const served = useQgcValue(ALLOW_TAKEOVER_SETTING);
  const [typed, setTyped] = useState<boolean | null>(null);
  const stored = served === null ? null : truthy(served);
  const allow = typed ?? stored;
  useEffect(() => {
    setTyped(null);
    onRead(stored);
  }, [stored]);
  return (
    <div className="flex items-center px-sheet py-1">
      <Text typography="bodyMedium" className="flex-1">Allow takeover</Text>
      <Checkbox
        checked={allow === true}
        enabled={allow !== null && allowTakeoverEditable(holder)}
        onCheckedChange={(wanted) => {
          setTyped(wanted);
          onRead(wanted);
          void (async () => {
            const refused = await saveAllowTakeover(wanted);
            onRefusal(refused);
            if (refused !== null) setTyped(null);
          })();
        }}
      />
    </div>
  );
};

const InControlNote = ({ holder, onRefusal }: { holder: ControlStation; onRefusal: (message: string | null) => void }) => {
  const [allow, setAllow] = useState<boolean | null>(null);
  const line = inControlLine(holder);
  const takeover = takeoverLine(holder);
  return (
    <>
      {line !== null ? <Text typography="bodyMedium" color={md('primary')} className="block px-sheet py-2">{line}</Text> : null}
      {takeover !== null ? <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-sheet">{takeover}</Text> : null}
      <ControlSectionTitle holder={holder} />
      <AllowTakeoverBox holder={holder} onRefusal={onRefusal} onRead={setAllow} />
      <TextButton
        enabled={takeoverChangeable(holder, allow)}
        onClick={() => {
          const wanted = allow;
          if (wanted === null) return;
          void (async () => onRefusal(await changeTakeover(wanted)))();
        }}
        className="mx-4"
      >
        Change
      </TextButton>
      <SettingFactRow path={GCS_SYSTEM_ID_PATH} title="This GCS MAVLink system ID" />
    </>
  );
};

export const MULTI_VEHICLE_PANEL_SETTING = 'settings.appSettings.enableMultiVehiclePanel';
export const MULTI_VEHICLE_PANEL = `${MULTI_VEHICLE_PANEL_SETTING}.rawValue`;

export const panelToggleShown = (fact: Json | null): boolean => fact === null || optBoolean(fact, 'visible', true);

export const multiVehiclePanelEnabled = (setting: Json | null): boolean =>
  setting !== null && has(setting, 'value') && !isNull(setting, 'value') ? optBoolean(setting, 'value', true) : true;

export const activeVehicleId = (view: Json | null): number | null => {
  if (view === null || isNull(view, 'activeId')) return null;
  const id = optInt(view, 'activeId', -1);
  return id > 0 ? id : null;
};

export const useRememberActiveVehicleId = (): number | null => useQgcDerived(VEHICLES_VIEW, activeVehicleId);

const ROW_COMPASS_SIZE = 28;
const ROW_HEADING_COLOUR = '#EE3424';
export const DISARMED_ALPHA = 0.5;

export const rowCompassAlpha = (armed: boolean): number => (armed ? 1 : DISARMED_ALPHA);

const ROW_HEADING_SHADE = '#C72B27';

const VehicleRowCompass = ({ heading, armed }: { heading: number; armed: boolean }) => {
  const radius = ROW_COMPASS_SIZE / 2;
  const centre = radius;
  const half = radius / 3;
  const top = centre - half;
  const bottom = centre + half;
  const notch = centre + half * 0.5;
  const right = `M${centre},${top}L${centre + half},${bottom}L${centre},${notch}Z`;
  const left = `M${centre},${top}L${centre - half},${bottom}L${centre},${notch}Z`;
  return (
    <svg width={ROW_COMPASS_SIZE} height={ROW_COMPASS_SIZE} viewBox={`0 0 ${ROW_COMPASS_SIZE} ${ROW_COMPASS_SIZE}`} style={{ opacity: rowCompassAlpha(armed) }} aria-hidden>
      <circle cx={centre} cy={centre} r={radius} fill={md('surface')} />
      <circle cx={centre} cy={centre} r={radius - 0.5} fill="none" stroke={md('onSurface')} strokeWidth={1} />
      {Number.isFinite(heading) ? (
        <g transform={`rotate(${heading} ${centre} ${centre})`}>
          <path d={right} fill={ROW_HEADING_COLOUR} />
          <path d={left} fill={ROW_HEADING_SHADE} />
          <path d={right} fill="none" stroke={ROW_HEADING_COLOUR} strokeWidth={1} />
          <path d={left} fill="none" stroke={ROW_HEADING_COLOUR} strokeWidth={1} />
        </g>
      ) : null}
    </svg>
  );
};
