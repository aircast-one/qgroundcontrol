import { Fragment, type ReactNode, useContext, useEffect, useRef, useState } from 'react';
import { AppCommands, VideoCommands } from '@/bridge/BridgeCommands';
import type { Json } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcBool, useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import { call } from '@/core/client';
import { useBackHandler, useEscapeHandler, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { same } from '@/kotlin/Std';
import { NavigationBar, NavigationBarItem, NavigationRail, NavigationRailItem, type SnackbarHostState, SnackbarHost, Surface, useSnackbarHostState, type SheetAnchor } from '@/m3';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { md } from '@/theme/Colors';
import { browserFullScreenSupported, toggleBrowserFullScreen, useBrowserFullScreen } from '@/app/BrowserFullScreen';
import { DEBUG_UI_ACTION, DebugUi, DebugUiReceiver } from '@/app/DebugUi';
import { GamepadInput } from '@/app/GamepadInput';
import { type MenuCommand, useMenuCommands } from '@/app/MenuCommands';
import { SpeechOut } from '@/app/SpeechOut';
import { VehicleLinkKeeper } from '@/app/VehicleLinkService';
import { LocalImmersive } from '@/map/AircastSheet';
import { FlyMap, plainTaps } from '@/map/FlyMap';
import { LocalFlyMapEdits } from '@/map/FlyMapEdits';
import { AircastTheme } from '@/map/Theme';
import type { TrackPoint } from '@/map/VehicleMap';
import { lostVehicle } from '@/map/LostVehicle';
import { VEHICLES_VIEW, type VehicleChoices, VehicleBridge, activeChanged, handoverNotice, rememberedChoices, vehicleChoices } from '@/map/VehiclePicker';
import { VehicleStateChip, useRememberActiveVehicleId } from '@/ui/ActiveVehicle';
import { DeviceDeepLink, DeviceStatusChip } from '@/ui/DeviceScreen';
import { deviceLinkOf } from '@/ui/DeviceScreens';
import { deviceRequested } from '@/ui/DeviceStatus';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { type AnalyzePage, AnalyzeScreen } from '@/ui/AnalyzeScreen';
import { AppFontScale } from '@/ui/AppFontScale';
import { AppNavigationState, LocalAppNavigation, LocalOpenPlan, OpenPlanDocument, type PlanRequest, navigationRefusal } from '@/ui/AppNavigation';
import { CAMERA_CONTROL_KEY, CameraControlLayer, CameraShutters } from '@/ui/CameraControlLayer';
import { CameraSwitch } from '@/ui/CameraSwitch';
import { ArmedFlightBar } from '@/ui/ArmedFlightBar';
import { CloseGuard } from '@/ui/CloseGuard';
import { PlanDraftKeeper } from '@/ui/PlanDraft';
import { ConnectingCard } from '@/ui/ConnectingCard';
import { ConnectionLocks } from '@/ui/ConnectionLocks';
import { ControlRequestPrompt } from '@/ui/ControlRequestPrompt';
import { DeepLinkSetupPrompt } from '@/ui/DeepLinkSetupPrompt';
import { DeviceCameraHost } from '@/ui/DeviceCameraHost';
import { BetaDiagnosticsDialog } from '@/ui/BetaDiagnosticsDialog';
import { HandoffDialog } from '@/ui/HandoffDialog';
import { FirstRunDialog } from '@/ui/FirstRunDialog';
import { trackFocus } from '@/ui/FocusReturn';
import { type FlyDeckLayout, LocalFlyDestinations, MenuTile } from '@/ui/FlyDeck';
import { FlyPortrait } from '@/ui/FlyPortrait';
import { FlyScreenState, LocalFlyScreenState, REQUESTABLE_SHEETS, TRAFFIC_SHEET } from '@/ui/FlyScreenState';
import { FleetCard, FlyMenuButton, FlyScreen, FlyView, MAP_LAYERS_CLEARANCE, type ModifierProps, flyViewShown, flyViewSwapped, loadFlyView, saveFlyView, useFlyIsPortrait } from '@/ui/FlyShell';
import { killedRecently, motorsKilledAt } from '@/ui/EmergencyStop';
import { FLY_STATE, NO_ARMED_SESSION, VEHICLE_FLIGHT_DISTANCE, armedSessionStep, flownDistanceText, flyState, landedSummary, useHasVehicle } from '@/ui/FlyState';
import { FollowMeReadout } from '@/ui/FollowMeReadout';
import { GimbalTakeControlDialog, gimbalRefused } from '@/ui/GimbalIndicator';
import { type AppMessage, AppMessageDialog, type NoticeBatch, RESET_ERROR_LEVEL_MESSAGES, criticalBanner, hostNoticesPath, noticeBatch, quietBanners, showLatest } from '@/ui/HostNotices';
import { LinkLostCard, LinkLostWatch } from '@/ui/LinkLost';
import { LogReplayBar } from '@/ui/LogReplayBar';
import { Map3dView, useMap3dDrawn } from '@/ui/Map3dView';
import { MapClickMenu, MapHoldHint, type MapPoint, RoiSheet, SetWaypointSheet, mapHoldHintText } from '@/ui/MapClickMenu';
import { useDesktopLayout } from '@/ui/DesktopLayout';
import { MissionCompleteDialog } from '@/ui/MissionCompleteDialog';
import { MissionProgressCard } from '@/ui/MissionProgressCard';
import { MotorLoadWidget } from '@/ui/MotorLoadWidget';
import { ObstacleArc } from '@/ui/ObstacleArc';
import { ObstacleReadout } from '@/ui/ObstacleReadout';
import { OrbitReadout } from '@/ui/OrbitReadout';
import { useAppDarkTheme } from '@/ui/Palette';
import { LAYOUT_EDITING_IN_FLIGHT, LayoutDoneButton, LayoutGroup, LayoutOptionsMenu, LayoutWidget, OverlayLayoutState, overlayLayoutStore, useHoldToEditLayout } from '@/ui/OverlayLayout';
import { PlanTab } from '@/ui/PlanTab';
import { RcControlsLayer } from '@/ui/RcControlsLayer';
import { ResumeFailedPrompt } from '@/ui/ResumeFailedPrompt';
import { AIRCRAFT_SETUP, SettingsGroup } from '@/ui/SettingsScreen';
import { SettingsSheet } from '@/ui/SettingsSheet';
import { StatusPill, batteryPercentNow, flightTimeNow } from '@/ui/StatusStrip';
import { useRememberSyntheticAvailable } from '@/ui/SyntheticView';
import { TerrainDataWarning } from '@/ui/TerrainDataWarning';
import { AppSnackbar, VehicleMessageBanner } from '@/ui/VehicleMessages';
import { FlightActions } from '@/ui/VehicleUi';
import { VIDEO_VIEW, videoReading } from '@/ui/VideoView';
import { type PicturePress, pictureHoldHintText, usePictureGoTo } from '@/ui/PictureHold';
import { VideoSurface } from '@/ui/VideoSurface';
import { VirtualStickSender } from '@/ui/VirtualJoystick';
import { VoiceAlerts } from '@/ui/VoiceAlerts';
import { VtolStateCell } from '@/ui/VtolStateCell';
import { LocalChangeNotice, useChangeNotice } from '@/ui/ValueRow';

const FLY_NOTICE_TOP = 76;
const FLY_NOTICE_BELOW_CHROME = 30;

const FLY_NOTICE_MAX_WIDTH = 560;

const DEEP_LINK_SCHEME = 'aircast-qgc';

export const reselectClearsAnalyze = (current: Tab, tapped: Tab): boolean => current === tapped && tapped === Tab.Analyze;

export type Tab = { readonly name: 'Fly' | 'Plan' | 'Analyze'; readonly ordinal: number; readonly label: string; readonly icon: DrawableName };

const TAB_VALUES = {
  Fly: { name: 'Fly', ordinal: 0, label: 'Fly', icon: 'ic_flight' },
  Plan: { name: 'Plan', ordinal: 1, label: 'Plan', icon: 'ic_map' },
  Analyze: { name: 'Analyze', ordinal: 2, label: 'Analyze', icon: 'ic_analytics' },
} as const satisfies Record<string, Tab>;

export const Tab = {
  ...TAB_VALUES,
  from: (destination: string): Tab | null => {
    switch (destination.toLowerCase()) {
      case 'fly': return TAB_VALUES.Fly;
      case 'plan': return TAB_VALUES.Plan;
      case 'analyze': return TAB_VALUES.Analyze;
      default: return null;
    }
  },
};

export const TabEntries: readonly Tab[] = [Tab.Fly, Tab.Plan, Tab.Analyze];

export const visibleTabs = (advanced: boolean): Tab[] => TabEntries.filter((it) => advanced || it !== Tab.Analyze);

const launchDeepLink = (): string | null => (window.location.search.length > 1 ? `${DEEP_LINK_SCHEME}://open${window.location.search}` : null);

export const MainActivity = () => {
  const [flyScreen] = useState(() => new FlyScreenState(new OverlayLayoutState(overlayLayoutStore())));
  const [navigation] = useState(() => new AppNavigationState());
  const [openPlan] = useState(() => new OpenPlanDocument());

  useLaunchedEffect([], (scope) => {
    Qgc.start();
    SpeechOut.start();
    const link = launchDeepLink();
    if (link !== null) void call('notifyDeepLink', link);
    deviceRequested.value = deviceLinkOf(window.location.hash);
    GamepadInput.start(scope);
    VirtualStickSender.start(scope);
    VehicleLinkKeeper.start();
    trackFocus();
  });

  useEffect(() => {
    if (!import.meta.env.DEV) return undefined;
    const receiver = new DebugUiReceiver();
    Object.assign(window, { [DEBUG_UI_ACTION]: (cmd: string | null, value: string | null = null) => receiver.onReceive(cmd, value) });
    return () => void Reflect.deleteProperty(window, DEBUG_UI_ACTION);
  }, []);

  return (
    <>
      <ConnectionLocks />
      <LinkLostWatch />
      <VoiceAlerts />
      <DeviceCameraHost />
      <AppFontScale>
        <LocalAppNavigation.Provider value={navigation}>
          <DeviceDeepLink />
          <LocalOpenPlan.Provider value={openPlan}>
            <LocalFlyScreenState.Provider value={flyScreen}>
              <LocalFlyMapEdits.Provider value={flyScreen.mapEdits}>
                <AircastShell />
              </LocalFlyMapEdits.Provider>
            </LocalFlyScreenState.Provider>
          </LocalOpenPlan.Provider>
        </LocalAppNavigation.Provider>
      </AppFontScale>
    </>
  );
};

type ShellFly = { connected: boolean; instance: number | null; armed: boolean; state: string | null };

const shellFly = (json: Json | null): ShellFly | null => {
  const it = flyState(json);
  return it === null ? null : { connected: it.connected, instance: it.instance, armed: it.armed, state: it.state };
};

const noVideoSourceOf = (json: Json | null): boolean => {
  const reading = videoReading(json);
  return reading !== null && !reading.available && !reading.sourceChosen;
};

const videoDecodingOf = (json: Json | null): boolean => videoReading(json)?.decoding === true;

const distinctMessages = (messages: readonly AppMessage[]): AppMessage[] => messages.filter((message, index) => messages.findIndex((it) => same(it, message)) === index);

const notBlank = (text: string): boolean => text.trim() !== '';

const orient = (orientation: OrientationLockType) => {
  if (orientation === 'any') return screen.orientation.unlock();
  return void screen.orientation.lock(orientation).catch(() => undefined);
};

const AlertHost = ({ hostState, className }: { hostState: SnackbarHostState; className?: string }) => {
  const shown = hostState.current;
  return shown === null ? null : (
    <div className={className}>
      <AppSnackbar data={{ visuals: { message: shown.message, duration: 'Indefinite' }, dismiss: () => shown.settle('Dismissed') }} />
    </div>
  );
};

const RequestedSettingsSheet = ({ onClose, overFly, status }: { onClose: () => void; overFly: boolean; status: ReactNode }) => {
  const navigation = useContext(LocalAppNavigation);
  const [requested] = useState(() => navigation.settingsPage.value);
  useEffect(() => {
    navigation.settingsPage.value = null;
  }, []);
  return <SettingsSheet requested={requested} onClose={onClose} overFly={overFly} status={status} />;
};

type RailDestination = { key: string; label: string; icon: DrawableName; selected: boolean; onClick: () => void };

export const AircastShell = () => {
  const navigation = useContext(LocalAppNavigation);
  const [tab, setTab] = useState<Tab>(Tab.Fly);
  const tabNow = useUpdatedRef(tab);
  const flyScreen = useContext(LocalFlyScreenState);
  const overlayLayout = flyScreen.layout;
  useEffect(() => {
    if (tab !== Tab.Fly) overlayLayout.editing.value = false;
  }, [tab]);
  const setupPage = useValue(navigation.setupPage);
  useEffect(() => {
    if (setupPage !== null) navigation.openAircraft();
  }, [setupPage]);
  const settingsPage = useValue(navigation.settingsPage);
  useEffect(() => {
    if (settingsPage !== null) navigation.settingsOpen.value = true;
  }, [settingsPage]);
  const vehicleNow = useHasVehicle();
  const hadVehicle = useRef(vehicleNow);
  useEffect(() => {
    if (vehicleNow && !hadVehicle.current && navigation.settingsShowing.value === SettingsGroup.Transmission) {
      navigation.settingsOpen.value = false;
      setTab(Tab.Fly);
    }
    hadVehicle.current = vehicleNow;
  }, [vehicleNow]);
  const [analyzePage, setAnalyzePage] = useState<AnalyzePage | null>(null);
  const [popEpoch, setPopEpoch] = useState(0);
  const [flyView, setFlyView] = useState<FlyView>(loadFlyView);
  useEffect(() => saveFlyView(flyView), [flyView]);

  const openRequested = (target: string) => {
    if (target === 'readings') flyScreen.choosingReadings.value = true;
    else if (target === 'settings') navigation.settingsOpen.value = true;
    else if (REQUESTABLE_SHEETS.has(target)) flyScreen.requestedSheet.value = target;
    else flyScreen.deckRequest.value = target;
  };

  useEffect(
    () =>
      DebugUi.commands.collect((command) => {
        switch (command.kind) {
          case 'ShowTab': return setTab(command.tab);
          case 'ShowFlyView':
            setTab(Tab.Fly);
            return setFlyView(command.view);
          case 'Orient': return orient(command.orientation);
          case 'EditLayout':
            if (command.on) return overlayLayout.startEditing();
            overlayLayout.editing.value = false;
            return undefined;
          case 'ResetLayout': return overlayLayout.reset();
          case 'Open': return openRequested(command.target);
        }
      }),
    [],
  );

  const portrait = useFlyIsPortrait();
  const landscape = !portrait;
  const layoutEditing = useValue(overlayLayout.editing);
  const guidedPanelOpen = useValue(flyScreen.guidedPanels) > 0;
  const requestedSheet = useValue(flyScreen.requestedSheet);
  const deckRequest = useValue(flyScreen.deckRequest);
  useEffect(() => {
    DebugUi.state = JSON.stringify({ tab: tab.name, flyView: flyView.name, landscape, layoutEditing, guidedPanelOpen, pendingOpen: requestedSheet ?? deckRequest ?? null });
  });
  const videoExpanded = flyView !== FlyView.Map;
  const [videoFullScreen, setVideoFullScreen] = useState(false);

  const [mapClickAt, setMapClickAt] = useState<MapPoint | null>(null);
  const [mapClickAnchor, setMapClickAnchor] = useState<SheetAnchor | undefined>(undefined);
  const [picturePressed, setPicturePressed] = useState<PicturePress | null>(null);
  const lastPointer = useRef<SheetAnchor>({ x: 0, y: 0 });
  const closeMapClick = () => {
    setMapClickAt(null);
    setPicturePressed(null);
  };
  useEffect(() => {
    if (!vehicleNow) closeMapClick();
  }, [vehicleNow]);
  useEffect(() => {
    const remember = (event: PointerEvent) => {
      lastPointer.current = { x: event.clientX, y: event.clientY };
    };
    window.addEventListener('pointerdown', remember, true);
    return () => window.removeEventListener('pointerdown', remember, true);
  }, []);
  const [mapHoldHint, setMapHoldHint] = useState<MapHoldHint>(MapHoldHint.Unseen);
  const [pictureHoldHint, setPictureHoldHint] = useState<MapHoldHint>(MapHoldHint.Unseen);
  const pictureGoTo = usePictureGoTo();
  const [waypointTapped, setWaypointTapped] = useState<number | null>(null);
  const [roiTapped, setRoiTapped] = useState<TrackPoint | null>(null);
  const mapInsets = useValue(flyScreen.mapInsets);
  const mapOrnamentEnd = useValue(flyScreen.mapOrnamentEnd);
  const map3dShown = useMap3dDrawn();

  const flightActions = (layout: FlyDeckLayout) => <FlightActions layout={layout} center={<CameraShutters />} />;
  const pressPicture = (press: PicturePress) => {
    setPictureHoldHint(MapHoldHint.Learned);
    if (press.point === null) {
      void snackbars.showSnackbar(press.reason);
      return;
    }
    setMapClickAnchor(lastPointer.current);
    setMapClickAt(press.point);
    setPicturePressed(press);
  };
  const flyVideo = (modifier: ModifierProps, expanded: boolean) => (
    <div {...modifier} inert={layoutEditing}>
      <div className="size-full" {...plainTaps(expanded && pictureGoTo && pictureHoldHint === MapHoldHint.Unseen ? () => setPictureHoldHint(MapHoldHint.Due) : null)}>
        <VideoSurface
          className="size-full"
          expanded={expanded}
          fullScreen={videoFullScreen}
          onClick={() => setFlyView(flyViewSwapped)}
          onDoubleTap={() => setVideoFullScreen((shown) => !shown)}
          onGround={pictureGoTo ? pressPicture : null}
          groundSpot={mapClickAt !== null ? (picturePressed?.spot ?? null) : null}
        />
      </div>
    </div>
  );
  const openMapClick = (latitude: number, longitude: number) => {
    setMapHoldHint(MapHoldHint.Learned);
    setMapClickAnchor(lastPointer.current);
    setMapClickAt({ latitude, longitude });
    setPicturePressed(null);
  };
  const flyMap = (modifier: ModifierProps) => (
    <div {...modifier} inert={layoutEditing}>
      {map3dShown && !mapPip ? (
        <Map3dView
          className="size-full"
          bottomInsetPx={mapBottomInset}
          logoEndInsetPx={mapLogoEndInset}
          onMapClick={openMapClick}
          onRefused={(reason: string) => void snackbars.showSnackbar(reason)}
        />
      ) : (
        <FlyMap
          className="size-full"
          cameraBottomPx={0}
          topInsetPx={mapOwnsInsets ? mapInsets.top : 0}
          bottomInsetPx={mapBottomInset}
          ornamentBottomPx={mapOrnamentBottom}
          logoEndInsetPx={mapLogoEndInset}
          pip={mapPip}
          onMapClick={openMapClick}
          onPlainTap={() => setMapHoldHint((hint: MapHoldHint) => (hint === MapHoldHint.Unseen && vehicleNow ? MapHoldHint.Due : hint))}
          onMissionItemClick={(sequence: number) => setWaypointTapped(sequence)}
          onRoiClick={(at: TrackPoint) => setRoiTapped(at)}
          onTrafficClick={() => {
            flyScreen.requestedSheet.value = TRAFFIC_SHEET;
          }}
          clickMarker={mapClickAt === null ? null : { latitude: mapClickAt.latitude, longitude: mapClickAt.longitude }}
        />
      )}
    </div>
  );
  const statusPillHold = useHoldToEditLayout();
  const flyCameraControlLayer = (
    <LayoutGroup widgetKey={CAMERA_CONTROL_KEY}>
      <CameraControlLayer shutters={!portrait} />
    </LayoutGroup>
  );
  const flyObstacleArc = (
    <LayoutWidget widgetKey="obstacleArc">
      <ObstacleArc />
    </LayoutWidget>
  );
  const flyOrbitReadout = (
    <LayoutWidget widgetKey="orbit">
      <OrbitReadout />
    </LayoutWidget>
  );
  const flyFollowMeReadout = (
    <LayoutWidget widgetKey="followMe">
      <FollowMeReadout />
    </LayoutWidget>
  );
  const flyRcControlsLayer = (
    <LayoutWidget widgetKey="rcControls" className="self-stretch">
      <RcControlsLayer />
    </LayoutWidget>
  );

  const desktop = useDesktopLayout();
  const snackbars = useSnackbarHostState();
  const changeNotice = useChangeNotice(snackbars);
  const refusedGimbal = useValue(gimbalRefused);
  useLaunchedEffect([refusedGimbal], async () => {
    if (refusedGimbal === null) return;
    await snackbars.showSnackbar(refusedGimbal);
    gimbalRefused.value = null;
  });
  useLaunchedEffect([mapHoldHint], async (scope) => {
    if (mapHoldHint !== MapHoldHint.Due) return;
    await snackbars.showSnackbar(mapHoldHintText(desktop));
    if (scope.isActive()) setMapHoldHint(MapHoldHint.Learned);
  });
  useLaunchedEffect([pictureHoldHint], async (scope) => {
    if (pictureHoldHint !== MapHoldHint.Due) return;
    await snackbars.showSnackbar(pictureHoldHintText(desktop));
    if (scope.isActive()) setPictureHoldHint(MapHoldHint.Learned);
  });
  const alerts = useSnackbarHostState();
  const [acknowledgedThrough, setAcknowledgedThrough] = useState(-1);
  const acknowledged = useRef(-1);
  const notices = useQgcPath(hostNoticesPath(acknowledgedThrough));
  const shownAt = useRef<ReadonlyMap<string, number>>(new Map());
  const payloadShowing = useRef<AbortController | null>(null);
  const [appMessages, setAppMessages] = useState<readonly AppMessage[]>([]);

  const refuseNavigation = (): boolean => {
    const said = navigationRefusal(navigation.blockedReason.value, true);
    if (said !== null) void snackbars.showSnackbar(said);
    return said !== null;
  };
  useBackHandler(tab !== Tab.Fly, () => {
    if (!refuseNavigation()) setTab(Tab.Fly);
  });

  const openDestination = (destination: string) => {
    const tabbed = Tab.from(destination);
    if (tabbed !== null) setTab(tabbed);
    else if (destination.toLowerCase() === 'setup') navigation.openAircraft();
    else navigation.settingsOpen.value = true;
  };

  const announce = async (batch: NoticeBatch, banners: readonly string[]) => {
    await AppCommands.acknowledgeNoticesThrough(batch.through);
    const critical = criticalBanner(banners.filter((it) => batch.errorBanners.includes(it)));
    if (critical !== null && tabNow.current !== Tab.Fly) {
      void alerts.showSnackbar(critical, null, 'Indefinite').then(() => Qgc.invoke(RESET_ERROR_LEVEL_MESSAGES));
    }
    await banners.filter((it) => !batch.errorBanners.includes(it)).reduce<Promise<unknown>>((shown, banner) => shown.then(() => snackbars.showSnackbar(banner)), Promise.resolve());
  };

  useEffect(() => {
    const batch = noticeBatch(notices);
    if (batch === null || batch.through <= acknowledged.current) return;
    acknowledged.current = batch.through;
    setAcknowledgedThrough(batch.through);
    batch.unknownKinds.map((it) => console.warn('HostNotices', `unrecognised notice kind '${it}' - showing it rather than guessing`));
    if (batch.destination !== null) openDestination(batch.destination);
    const now = Date.now();
    const banners = quietBanners(batch.banners, shownAt.current, now);
    shownAt.current = new Map([...shownAt.current, ...banners.map((it): [string, number] => [it, now])]);
    setAppMessages((messages) => distinctMessages([...messages, ...batch.dialogs]));
    if (batch.payload !== null) payloadShowing.current = showLatest(snackbars, payloadShowing.current, batch.payload);
    void announce(batch, banners);
  }, [notices]);

  const flyNow = useQgcDerived(FLY_STATE, shellFly);
  const armedNow = flyNow !== null && flyNow.connected && flyNow.armed;
  const editingInFlight = layoutEditing && armedNow;
  const flyChromeBottom = useValue(flyScreen.widgetInsets).top;
  useEffect(() => {
    if (!editingInFlight) return undefined;
    const shown = new AbortController();
    void snackbars.showSnackbar(LAYOUT_EDITING_IN_FLIGHT, 'Done', 'Indefinite', shown.signal).then((result) => {
      if (result === 'ActionPerformed') overlayLayout.editing.value = false;
    });
    return () => shown.abort();
  }, [editingInFlight]);
  const activeId = useRememberActiveVehicleId();
  const sub = useQgcBool('vehicle.sub');
  const rover = useQgcBool('vehicle.rover');
  const lands = !sub && !rover;
  const armedSession = useRef(NO_ARMED_SESSION);
  const armedBattery = useRef<number | null>(null);

  const announceDisarm = async (said: string, landed: boolean, startBattery: number | null) => {
    const used = async (start: number) => {
      const now = await batteryPercentNow();
      return now === null ? null : start - now;
    };
    const summary = !landed ? '' : landedSummary(await flightTimeNow(), flownDistanceText(await Qgc.get(VEHICLE_FLIGHT_DISTANCE)), startBattery === null ? null : await used(startBattery));
    await snackbars.showSnackbar([said, summary].filter(notBlank).join(' · '));
  };

  useEffect(() => {
    const step = armedSessionStep(armedSession.current, flyNow?.connected === true, flyNow?.instance ?? activeId, armedNow, flyNow?.state ?? null, { killed: killedRecently(motorsKilledAt.value, performance.now()), lands });
    if (step.started) {
      void batteryPercentNow().then((percent) => {
        armedBattery.current = percent;
      });
    }
    if (step.notice !== null) void announceDisarm(step.notice, step.landed, armedBattery.current);
    armedSession.current = step.session;
  }, [flyNow?.armed, flyNow?.state, flyNow?.connected, flyNow?.instance, activeId]);

  const noVideoSource = useQgcDerived(VIDEO_VIEW, noVideoSourceOf);
  const pictureNow = useQgcDerived(VIDEO_VIEW, videoDecodingOf);
  const syntheticOnScreen = useRememberSyntheticAvailable();
  const shownFlyView = flyViewShown(flyView, flyNow?.armed === true, noVideoSource && !syntheticOnScreen, !vehicleNow && noVideoSource && !pictureNow);
  const mapPip = shownFlyView === FlyView.Video && !portrait;
  const mapOwnsInsets = shownFlyView === FlyView.Map || portrait;
  const mapBottomInset = mapOwnsInsets ? mapInsets.bottom : 0;
  const mapOrnamentBottom = mapOwnsInsets ? mapInsets.ornaments : 0;
  const mapLogoEndInset = mapOwnsInsets ? Math.round(mapOrnamentEnd ?? MAP_LAYERS_CLEARANCE) : null;

  const vehicles = useQgcPath(VEHICLES_VIEW);
  const lastVehicles = useRef<VehicleChoices | null>(null);
  const lastChoices = useRef<VehicleChoices | null>(null);
  useEffect(() => {
    const now = vehicleChoices(vehicles);
    if (lastChoices.current !== null && same(lastChoices.current, now)) return;
    lastChoices.current = now;
    const notice = handoverNotice(lastVehicles.current, now, VehicleBridge.lastAsked);
    if (activeChanged(lastVehicles.current, now)) VehicleBridge.forget();
    lastVehicles.current = rememberedChoices(lastVehicles.current, now);
    if (notice !== null) void snackbars.showSnackbar(notice);
  }, [vehicles]);

  useLaunchedEffect([], async () => {
    await VideoCommands.setNativeRendering(true);
    await VideoCommands.initNative();
  });

  const darkBars = useAppDarkTheme();

  const videoDecoding = useQgcDerived(VIDEO_VIEW, videoDecodingOf);
  const fullScreen = videoFullScreen && tab === Tab.Fly && videoExpanded && (videoDecoding || syntheticOnScreen);
  useEffect(() => {
    if (!fullScreen) setVideoFullScreen(false);
  }, [fullScreen, videoFullScreen]);
  useEscapeHandler(fullScreen, () => setVideoFullScreen(false));

  const settingsOpen = useValue(navigation.settingsOpen);
  const settingsShowing = useValue(navigation.settingsShowing);
  const setupShowing = settingsOpen && settingsShowing === null;
  const [settingsEpoch, setSettingsEpoch] = useState(0);
  const browserFullScreen = useBrowserFullScreen();
  const tabs = visibleTabs(useAdvancedUiShown());
  const [resumeDismissed, setResumeDismissed] = useState<number | null>(null);
  const onFly = tab === Tab.Fly;
  const flyLandscape = onFly && landscape;
  const closeSettings = () => {
    navigation.settingsOpen.value = false;
  };
  const selectTab = (entry: Tab) => {
    if (refuseNavigation()) return;
    if (settingsOpen) {
      closeSettings();
      setTab(entry);
    } else if (tab === entry) {
      if (reselectClearsAnalyze(tab, entry)) setAnalyzePage(null);
      setPopEpoch((epoch) => epoch + 1);
    } else {
      setTab(entry);
    }
  };
  const openSettings = () => {
    if (settingsOpen && refuseNavigation()) return;
    setSettingsEpoch((epoch) => epoch + 1);
    navigation.settingsOpen.value = true;
  };
  const openSetup = () => {
    if (settingsOpen && refuseNavigation()) return;
    setSettingsEpoch((epoch) => epoch + 1);
    navigation.openAircraft();
  };
  const backToFly = () => {
    if (refuseNavigation()) return;
    closeSettings();
    setTab(Tab.Fly);
  };
  const flyRequested = useValue(navigation.flyRequested);
  useEffect(() => {
    if (!flyRequested) return;
    navigation.flyRequested.value = false;
    backToFly();
  }, [flyRequested]);
  const showTab = (entry: Tab): boolean => {
    if (refuseNavigation()) return false;
    closeSettings();
    setTab(entry);
    return true;
  };
  const requestPlan = (request: PlanRequest) => {
    if (showTab(Tab.Plan)) navigation.planRequest.value = request;
  };
  const menuCommands: Record<MenuCommand, () => void> = {
    about: () => {
      navigation.settingsPage.value = 'About';
    },
    settings: openSettings,
    fly: () => void showTab(Tab.Fly),
    plan: () => void showTab(Tab.Plan),
    analyze: () => void showTab(Tab.Analyze),
    setup: openSetup,
    'new-plan': () => requestPlan('new'),
    'open-plan': () => requestPlan('open'),
    'save-plan': () => requestPlan('save'),
    'save-plan-as': () => requestPlan('saveAs'),
  };
  useMenuCommands((command) => menuCommands[command]());
  const railShown = !fullScreen && (desktop || landscape) && (!onFly || settingsOpen);
  const destinations: readonly RailDestination[] = [
    ...tabs.map((entry) => ({ key: entry.name, label: entry.label, icon: entry.icon, selected: tab === entry && !settingsOpen, onClick: () => selectTab(entry) })),
    { key: 'setup', label: AIRCRAFT_SETUP, icon: 'ic_build', selected: setupShowing, onClick: openSetup },
    { key: 'settings', label: 'Settings', icon: 'ic_settings', selected: settingsOpen && !setupShowing, onClick: openSettings },
  ];
  const fullScreenTiles = browserFullScreenSupported() ? [MenuTile({ label: browserFullScreen ? 'Exit full screen' : 'Full screen', icon: 'ic_fullscreen', enabled: true, onClick: () => void toggleBrowserFullScreen().catch(() => undefined) })] : [];
  const flyDestinations: readonly MenuTile[] = [
    ...(railShown ? [] : destinations.filter((it) => it.key !== Tab.Fly.name).map((it) => MenuTile({ label: it.label, icon: it.icon, enabled: true, onClick: it.onClick }))),
    ...fullScreenTiles,
  ];
  const linkLost = useValue(lostVehicle) !== null;
  const flightBar = (armedNow || linkLost) && (!onFly || settingsOpen) ? <ArmedFlightBar onFly={backToFly} /> : null;
  const sheetCovers = settingsOpen && !desktop;
  const settingsSheet = settingsOpen ? <RequestedSettingsSheet key={settingsEpoch} onClose={closeSettings} overFly={onFly} status={sheetCovers ? flightBar : null} /> : null;

  const flyStatus = (
    <>
      {layoutEditing ? <LayoutOptionsMenu /> : null}
      <div className={`flex items-center justify-start ${layoutEditing ? 'grow shrink-0' : 'min-w-0 flex-1'}`}>
        <LayoutWidget widgetKey="vehicleState" movable={false} hideable={false} className="min-w-0">
          <VehicleStateChip />
        </LayoutWidget>
      </div>
      <LayoutWidget widgetKey="vtolState" movable={false} hideable={false}>
        <VtolStateCell />
      </LayoutWidget>
      <ControlRequestPrompt />
      <DeviceStatusChip />
      <div className={layoutEditing && portrait ? 'order-last min-w-0 basis-full pt-2 pb-2' : 'min-w-0'} {...statusPillHold}>
        <StatusPill />
      </div>
      {layoutEditing ? <LayoutDoneButton /> : <FlyMenuButton />}
    </>
  );
  const flyRail = (splitRoom: boolean) => (
    <div className="flex flex-col items-end">
      {!splitRoom ? (
        <LayoutWidget widgetKey="cameraSwitch">
          <div className="pb-2">
            <CameraSwitch splitRoom={false} />
          </div>
        </LayoutWidget>
      ) : null}
      {flyCameraControlLayer}
    </div>
  );
  const flyOverlays = (
    <>
      <LayoutWidget widgetKey="messageBanner" hideable={false}>
        <VehicleMessageBanner />
      </LayoutWidget>
      <LayoutWidget widgetKey="motorLoad">
        <MotorLoadWidget />
      </LayoutWidget>
      <LayoutWidget widgetKey="fleet">
        <FleetCard />
      </LayoutWidget>
      <LayoutWidget widgetKey="missionProgress">
        <MissionProgressCard />
      </LayoutWidget>
      <LayoutWidget widgetKey="obstacleReadout">
        <ObstacleReadout />
      </LayoutWidget>
      <LayoutWidget widgetKey="terrainData">
        <TerrainDataWarning />
      </LayoutWidget>
      {flyOrbitReadout}
      {flyFollowMeReadout}
      {flyRcControlsLayer}
    </>
  );
  const enterFullScreen = () => {
    setFlyView(FlyView.Video);
    setVideoFullScreen(true);
  };
  const exitFullScreen = () => setVideoFullScreen(false);
  const contentInsets = onFly ? '' : `pt-[env(safe-area-inset-top)] pr-[env(safe-area-inset-right)] ${landscape ? 'pb-[env(safe-area-inset-bottom)]' : 'pl-[env(safe-area-inset-left)]'}`;

  return (
    <AircastTheme dark={darkBars}>
      <CloseGuard />
      <PlanDraftKeeper />
      <GimbalTakeControlDialog />
      {appMessages[0] !== undefined ? <AppMessageDialog message={appMessages[0]} onOpenSetup={() => navigation.openAircraft()} onDismiss={() => setAppMessages((messages) => messages.slice(1))} /> : null}
      <LocalImmersive.Provider value={flyLandscape}>
        <LocalFlyDestinations.Provider value={flyDestinations}>
          <LocalChangeNotice.Provider value={changeNotice}>
          <div className="flex h-full w-full bg-surface text-on-surface">
            {railShown ? (
              <div className="contents" inert={sheetCovers}>
                <NavigationRail className="h-full shrink-0 overflow-y-auto pt-[max(12px,env(safe-area-inset-top))] pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)]">
                  {destinations.map((entry) => (
                    <NavigationRailItem key={entry.key} selected={entry.selected} onClick={entry.onClick} icon={<Drawable name={entry.icon} label={entry.label} />} label={entry.label} />
                  ))}
                </NavigationRail>
              </div>
            ) : null}
            <div className="flex min-w-0 flex-1 flex-col">
              {!onFly ? (
                <div className="contents" inert={sheetCovers}>
                  <AlertHost hostState={alerts} className="p-3 pt-[max(12px,env(safe-area-inset-top))]" />
                </div>
              ) : null}
              {sheetCovers ? null : flightBar}
              <div className={`relative min-h-0 flex-1 overflow-hidden ${contentInsets}`}>
                <div className="contents" inert={settingsOpen}>
                  {onFly && flyLandscape ? (
                    <FlyScreen
                      view={shownFlyView}
                      onView={setFlyView}
                      status={flyStatus}
                      video={flyVideo}
                      map={flyMap}
                      keyRow={flyObstacleArc}
                      cameraSwitch={(splitRoom, modifier) => (
                        <LayoutWidget widgetKey="cameraSwitch">
                          <div {...modifier}>
                            <CameraSwitch splitRoom={splitRoom} />
                          </div>
                        </LayoutWidget>
                      )}
                      cameraControls={flyCameraControlLayer}
                      overlays={flyOverlays}
                      actions={flightActions}
                      fullScreen={fullScreen}
                      onFullScreen={enterFullScreen}
                      onExitFullScreen={exitFullScreen}
                    />
                  ) : null}
                  {onFly && !flyLandscape ? (
                    <FlyPortrait
                      view={shownFlyView}
                      onView={setFlyView}
                      status={flyStatus}
                      video={flyVideo}
                      map={flyMap}
                      keyRow={flyObstacleArc}
                      rail={flyRail}
                      overlays={flyOverlays}
                      actions={flightActions}
                      fullScreen={fullScreen}
                      onFullScreen={enterFullScreen}
                      onExitFullScreen={exitFullScreen}
                    />
                  ) : null}
                  {onFly ? <ConnectingCard className="absolute inset-x-0 top-1/2 z-10 -translate-y-1/2" /> : null}
                  {mapClickAt !== null && onFly ? <MapClickMenu point={mapClickAt} anchor={mapClickAnchor} only={picturePressed?.actions ?? null} onDismiss={closeMapClick} /> : null}
                  {waypointTapped !== null && onFly ? <SetWaypointSheet sequence={waypointTapped} onDismiss={() => setWaypointTapped(null)} /> : null}
                  {roiTapped !== null && onFly ? <RoiSheet at={roiTapped} onDismiss={() => setRoiTapped(null)} /> : null}
                  <Fragment key={popEpoch}>
                    {tab === Tab.Plan ? (
                      <Surface className="absolute inset-0" contentColor={md('onSurface')}>
                        <PlanTab onFly={() => setTab(Tab.Fly)} />
                      </Surface>
                    ) : null}
                    {tab === Tab.Analyze ? <AnalyzeScreen page={analyzePage} onSelect={setAnalyzePage} className="absolute inset-0" /> : null}
                  </Fragment>
                </div>
                {onFly ? (
                  <div className="pointer-events-none absolute inset-x-0 z-40 flex justify-center px-4" style={{ top: `max(calc(env(safe-area-inset-top) + ${FLY_NOTICE_TOP}px), ${flyChromeBottom + FLY_NOTICE_BELOW_CHROME}px)` }}>
                    <div className="flex w-full flex-col gap-2" style={{ maxWidth: FLY_NOTICE_MAX_WIDTH }}>
                      {settingsOpen ? null : <LinkLostCard className="pointer-events-auto" />}
                      <SnackbarHost hostState={snackbars} />
                    </div>
                  </div>
                ) : null}
                {settingsSheet}
                <MissionCompleteDialog />
                {onFly ? <ResumeFailedPrompt dismissed={resumeDismissed} onDismissed={setResumeDismissed} /> : null}
                <FirstRunDialog />
                {appMessages[0] === undefined ? <BetaDiagnosticsDialog /> : null}
                <HandoffDialog />
                <DeepLinkSetupPrompt />
                {!onFly ? (
                  <div className="pointer-events-none absolute inset-x-0 bottom-4 z-40 flex justify-center px-4">
                    <SnackbarHost hostState={snackbars} />
                  </div>
                ) : null}
              </div>
              <div className="flex flex-col" inert={sheetCovers}>
                <LogReplayBar />
                {!landscape && !onFly && !railShown ? (
                  <NavigationBar>
                    {destinations.map((entry) => (
                      <NavigationBarItem key={entry.key} selected={entry.selected} onClick={entry.onClick} icon={<Drawable name={entry.icon} label={entry.label} />} label={entry.label} />
                    ))}
                  </NavigationBar>
                ) : null}
              </div>
            </div>
          </div>
          </LocalChangeNotice.Provider>
        </LocalFlyDestinations.Provider>
      </LocalImmersive.Provider>
    </AircastTheme>
  );
};
