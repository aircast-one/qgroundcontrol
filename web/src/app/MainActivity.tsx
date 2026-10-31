import { Fragment, useContext, useEffect, useRef, useState } from 'react';
import { AppCommands, VideoCommands } from '@/bridge/BridgeCommands';
import type { Json } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import { call } from '@/core/client';
import { useBackHandler, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { same } from '@/kotlin/Std';
import { NavigationBar, NavigationBarItem, NavigationRail, NavigationRailItem, type SnackbarHostState, SnackbarHost, Surface, useSnackbarHostState, type SheetAnchor } from '@/m3';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { md } from '@/theme/Colors';
import { DEBUG_UI_ACTION, DebugUi, DebugUiReceiver } from '@/app/DebugUi';
import { GamepadInput } from '@/app/GamepadInput';
import { SpeechOut } from '@/app/SpeechOut';
import { VehicleLinkKeeper } from '@/app/VehicleLinkService';
import { LocalImmersive } from '@/map/AircastSheet';
import { FlyMap, plainTaps } from '@/map/FlyMap';
import { LocalFlyMapEdits } from '@/map/FlyMapEdits';
import { AircastTheme } from '@/map/Theme';
import type { TrackPoint } from '@/map/VehicleMap';
import { VEHICLES_VIEW, type VehicleChoices, VehicleBridge, activeChanged, handoverNotice, rememberedChoices, vehicleChoices } from '@/map/VehiclePicker';
import { VehicleStateChip } from '@/ui/ActiveVehicle';
import { DeviceDeepLink, DeviceStatusChip } from '@/ui/DeviceScreen';
import { deviceLinkOf } from '@/ui/DeviceScreens';
import { deviceRequested } from '@/ui/DeviceStatus';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { type AnalyzePage, AnalyzeScreen } from '@/ui/AnalyzeScreen';
import { AppFontScale } from '@/ui/AppFontScale';
import { AppNavigationState, LocalAppNavigation, LocalOpenPlan, OpenPlanDocument, navigationRefusal } from '@/ui/AppNavigation';
import { CameraControlLayer, CameraShutters } from '@/ui/CameraControlLayer';
import { CameraSwitch } from '@/ui/CameraSwitch';
import { CloseGuard } from '@/ui/CloseGuard';
import { ConnectingCard } from '@/ui/ConnectingCard';
import { ConnectionLocks } from '@/ui/ConnectionLocks';
import { ControlRequestPrompt } from '@/ui/ControlRequestPrompt';
import { DeepLinkSetupPrompt } from '@/ui/DeepLinkSetupPrompt';
import { DeviceCameraHost } from '@/ui/DeviceCameraHost';
import { FirstRunDialog } from '@/ui/FirstRunDialog';
import type { FlyDeckLayout } from '@/ui/FlyDeck';
import { FlyPortrait } from '@/ui/FlyPortrait';
import { FlyScreenState, LocalFlyScreenState, REQUESTABLE_SHEETS, TRAFFIC_SHEET } from '@/ui/FlyScreenState';
import { FleetCard, FlyScreen, FlyTabMenu, FlyView, MAP_LAYERS_CLEARANCE, type ModifierProps, flyViewShown, flyViewSwapped, loadFlyView, saveFlyView, useFlyIsPortrait } from '@/ui/FlyShell';
import { FLY_STATE, VEHICLE_FLIGHT_DISTANCE, disarmNotice, flownDistanceText, flyState, landedSummary, useHasVehicle } from '@/ui/FlyState';
import { FollowMeReadout } from '@/ui/FollowMeReadout';
import { GimbalTakeControlDialog } from '@/ui/GimbalIndicator';
import { type AppMessage, AppMessageDialog, type NoticeBatch, RESET_ERROR_LEVEL_MESSAGES, criticalBanner, hostNoticesPath, noticeBatch, quietBanners, showLatest } from '@/ui/HostNotices';
import { LogReplayBar } from '@/ui/LogReplayBar';
import { MapClickMenu, MapHoldHint, type MapPoint, RoiSheet, SetWaypointSheet, mapHoldHintText } from '@/ui/MapClickMenu';
import { useDesktopLayout } from '@/ui/DesktopLayout';
import { MissionCompleteDialog } from '@/ui/MissionCompleteDialog';
import { MissionProgressCard } from '@/ui/MissionProgressCard';
import { ObstacleArc } from '@/ui/ObstacleArc';
import { ObstacleReadout } from '@/ui/ObstacleReadout';
import { OrbitReadout } from '@/ui/OrbitReadout';
import { useAppDarkTheme } from '@/ui/Palette';
import { LayoutWidget, OverlayEditBar, OverlayLayoutState, overlayLayoutStore } from '@/ui/OverlayLayout';
import { PlanTab } from '@/ui/PlanTab';
import { RcControlsLayer } from '@/ui/RcControlsLayer';
import { ResumeFailedPrompt } from '@/ui/ResumeFailedPrompt';
import { SettingsGroup } from '@/ui/SettingsScreen';
import { FlySettingsButton, SettingsSheet } from '@/ui/SettingsSheet';
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

const FLY_NOTICE_TOP = 76;

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

type ShellFly = { connected: boolean; armed: boolean; state: string | null };

const shellFly = (json: Json | null): ShellFly | null => {
  const it = flyState(json);
  return it === null ? null : { connected: it.connected, armed: it.armed, state: it.state };
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

const RequestedSettingsSheet = ({ onClose }: { onClose: () => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const [requested] = useState(() => navigation.settingsPage.value);
  useEffect(() => {
    navigation.settingsPage.value = null;
  }, []);
  return <SettingsSheet requested={requested} onClose={onClose} />;
};

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
    if (vehicleNow && !hadVehicle.current && navigation.settingsShowing.value === SettingsGroup.Transmission) navigation.settingsOpen.value = false;
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
  const layoutLocked = useValue(overlayLayout.locked);
  const guidedPanelOpen = useValue(flyScreen.guidedPanels) > 0;
  const requestedSheet = useValue(flyScreen.requestedSheet);
  const deckRequest = useValue(flyScreen.deckRequest);
  useEffect(() => {
    DebugUi.state = JSON.stringify({ tab: tab.name, flyView: flyView.name, landscape, layoutEditing, layoutLocked, guidedPanelOpen, pendingOpen: requestedSheet ?? deckRequest ?? null });
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
  const mapOwnsInsets = flyView === FlyView.Map || portrait;

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
    <div {...modifier}>
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
  const flyMap = (modifier: ModifierProps) => (
    <div {...modifier}>
      <FlyMap
        className="size-full"
        cameraBottomPx={0}
        topInsetPx={mapOwnsInsets ? mapInsets.top : 0}
        bottomInsetPx={mapOwnsInsets ? mapInsets.bottom : 0}
        logoEndInsetPx={mapOwnsInsets ? Math.round(mapOrnamentEnd ?? MAP_LAYERS_CLEARANCE) : null}
        pip={flyView === FlyView.Video && !portrait}
        onMapClick={(latitude: number, longitude: number) => {
          setMapHoldHint(MapHoldHint.Learned);
          setMapClickAnchor(lastPointer.current);
          setMapClickAt({ latitude, longitude });
          setPicturePressed(null);
        }}
        onPlainTap={() => setMapHoldHint((hint: MapHoldHint) => (hint === MapHoldHint.Unseen && vehicleNow ? MapHoldHint.Due : hint))}
        onMissionItemClick={(sequence: number) => setWaypointTapped(sequence)}
        onRoiClick={(at: TrackPoint) => setRoiTapped(at)}
        onTrafficClick={() => {
          flyScreen.requestedSheet.value = TRAFFIC_SHEET;
        }}
        clickMarker={mapClickAt === null ? null : { latitude: mapClickAt.latitude, longitude: mapClickAt.longitude }}
      />
    </div>
  );
  const flyCameraControlLayer = (
    <LayoutWidget widgetKey="cameraControl">
      <CameraControlLayer shutters={!portrait} />
    </LayoutWidget>
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
    <LayoutWidget widgetKey="rcControls">
      <RcControlsLayer />
    </LayoutWidget>
  );

  const desktop = useDesktopLayout();
  const snackbars = useSnackbarHostState();
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
  const wasArmed = useRef(false);
  const flewWhileArmed = useRef(false);
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
    const armedNow = flyNow !== null && flyNow.connected && flyNow.armed;
    if (armedNow && !wasArmed.current) {
      void batteryPercentNow().then((percent) => {
        armedBattery.current = percent;
      });
    }
    const said = disarmNotice(wasArmed.current, flewWhileArmed.current, armedNow);
    if (said !== null) void announceDisarm(said, flewWhileArmed.current, armedBattery.current);
    flewWhileArmed.current = armedNow && (flewWhileArmed.current || flyNow?.state === 'flying' || flyNow?.state === 'landing');
    wasArmed.current = armedNow;
  }, [flyNow?.armed, flyNow?.state]);

  const noVideoSource = useQgcDerived(VIDEO_VIEW, noVideoSourceOf);
  const pictureNow = useQgcDerived(VIDEO_VIEW, videoDecodingOf);
  const syntheticOnScreen = useRememberSyntheticAvailable();
  const shownFlyView = flyViewShown(flyView, flyNow?.armed === true, noVideoSource && !syntheticOnScreen, !vehicleNow && noVideoSource && !pictureNow);

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
  useBackHandler(fullScreen, () => setVideoFullScreen(false));

  const settingsOpen = useValue(navigation.settingsOpen);
  const tabs = visibleTabs(useAdvancedUiShown());
  const [resumeDismissed, setResumeDismissed] = useState<number | null>(null);
  const onFly = tab === Tab.Fly;
  const flyLandscape = onFly && landscape;
  const selectTab = (entry: Tab) => {
    if (refuseNavigation()) return;
    if (tab === entry) {
      if (reselectClearsAnalyze(tab, entry)) setAnalyzePage(null);
      setPopEpoch((epoch) => epoch + 1);
    } else {
      setTab(entry);
    }
  };
  const openSettings = () => {
    navigation.settingsOpen.value = true;
  };

  const flyStatus = (
    <>
      <FlyTabMenu destinations={[...tabs.map((it) => ({ label: it.label, icon: it.icon, current: it === Tab.Fly, onSelect: () => selectTab(it) })), { label: 'Settings', icon: 'ic_settings', current: false, onSelect: openSettings }]} />
      <div className="flex min-w-0 flex-1 items-center justify-start">
        <LayoutWidget widgetKey="vehicleState" movable={false} hideable={false} className="min-w-0">
          <VehicleStateChip />
        </LayoutWidget>
      </div>
      <LayoutWidget widgetKey="vtolState" movable={false} hideable={false}>
        <VtolStateCell />
      </LayoutWidget>
      <ControlRequestPrompt />
      <DeviceStatusChip />
      <LayoutWidget widgetKey="statusPill" movable={false} hideable={false}>
        <StatusPill />
      </LayoutWidget>
      <FlySettingsButton />
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
      <GimbalTakeControlDialog />
      {appMessages[0] !== undefined ? <AppMessageDialog message={appMessages[0]} onOpenSetup={() => navigation.openAircraft()} onDismiss={() => setAppMessages((messages) => messages.slice(1))} /> : null}
      <LocalImmersive.Provider value={flyLandscape}>
        <div className="flex h-full w-full bg-surface text-on-surface">
          {landscape && !flyLandscape ? (
            <NavigationRail className="h-full pt-[max(12px,env(safe-area-inset-top))] pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)]">
              {tabs.map((entry) => (
                <NavigationRailItem key={entry.name} selected={tab === entry && !settingsOpen} onClick={() => selectTab(entry)} icon={<Drawable name={entry.icon} label={entry.label} />} label={entry.label} />
              ))}
              <NavigationRailItem selected={settingsOpen} onClick={openSettings} icon={<Drawable name="ic_settings" label="Settings" />} label="Settings" />
            </NavigationRail>
          ) : null}
          <div className="flex min-w-0 flex-1 flex-col">
            {!onFly ? <AlertHost hostState={alerts} className="p-3 pt-[max(12px,env(safe-area-inset-top))]" /> : null}
            <div className={`relative min-h-0 flex-1 overflow-hidden ${contentInsets}`}>
              {onFly ? <OverlayEditBar className="absolute bottom-0 left-1/2 z-[2] w-full max-w-[560px] -translate-x-1/2 p-2 pb-[calc(8px+env(safe-area-inset-bottom))]" /> : null}
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
              {onFly ? (
                <div className="pointer-events-none absolute inset-x-0 z-40 flex justify-center px-4" style={{ top: `calc(env(safe-area-inset-top) + ${FLY_NOTICE_TOP}px)` }}>
                  <div className="w-full" style={{ maxWidth: FLY_NOTICE_MAX_WIDTH }}>
                    <SnackbarHost hostState={snackbars} />
                  </div>
                </div>
              ) : null}

              {mapClickAt !== null && onFly ? <MapClickMenu point={mapClickAt} anchor={mapClickAnchor} only={picturePressed?.actions ?? null} onDismiss={closeMapClick} /> : null}
              {waypointTapped !== null && onFly ? <SetWaypointSheet sequence={waypointTapped} onDismiss={() => setWaypointTapped(null)} /> : null}
              {roiTapped !== null && onFly ? <RoiSheet at={roiTapped} onDismiss={() => setRoiTapped(null)} /> : null}
              {settingsOpen ? (
                <RequestedSettingsSheet
                  onClose={() => {
                    navigation.settingsOpen.value = false;
                  }}
                />
              ) : null}
              <MissionCompleteDialog />
              {onFly ? <ResumeFailedPrompt dismissed={resumeDismissed} onDismissed={setResumeDismissed} /> : null}
              <FirstRunDialog />
              <DeepLinkSetupPrompt />

              <Fragment key={popEpoch}>
                {tab === Tab.Plan ? (
                  <Surface className="absolute inset-0" contentColor={md('onSurface')}>
                    <PlanTab onFly={() => setTab(Tab.Fly)} />
                  </Surface>
                ) : null}
                {tab === Tab.Analyze ? <AnalyzeScreen page={analyzePage} onSelect={setAnalyzePage} className="absolute inset-0" /> : null}
              </Fragment>
              {!onFly ? (
                <div className="pointer-events-none absolute inset-x-0 bottom-4 z-40 flex justify-center px-4">
                  <SnackbarHost hostState={snackbars} />
                </div>
              ) : null}
            </div>
            <div className="flex flex-col">
              <LogReplayBar />
              {!landscape && !onFly ? (
                <NavigationBar>
                  {tabs.map((entry) => (
                    <NavigationBarItem key={entry.name} selected={tab === entry && !settingsOpen} onClick={() => selectTab(entry)} icon={<Drawable name={entry.icon} label={entry.label} />} label={entry.label} />
                  ))}
                  <NavigationBarItem selected={settingsOpen} onClick={openSettings} icon={<Drawable name="ic_settings" label="Settings" />} label="Settings" />
                </NavigationBar>
              ) : null}
            </div>
          </div>
        </div>
      </LocalImmersive.Provider>
    </AircastTheme>
  );
};
