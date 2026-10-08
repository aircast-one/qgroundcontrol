import os
import SwiftUI

private let RAIL_SHUTTER_ROOM: CGFloat = 56
private let VIRTUAL_JOYSTICK_BOTTOM_MARGIN: CGFloat = 96
private let EDIT_BAR_MAX_WIDTH: CGFloat = 560
private let NAVIGATION_RAIL_WIDTH: CGFloat = 80
private let NAVIGATION_BAR_HEIGHT: CGFloat = 64
private let noticeLog = Logger(subsystem: "one.aircast.app", category: "HostNotices")

func reselectClearsAnalyze(_ current: Tab, _ tapped: Tab) -> Bool { current == tapped && tapped == .Analyze }

func visibleTabs(_ advanced: Bool) -> [Tab] { Tab.allCases.filter { advanced || $0 != .Analyze } }

enum Tab: String, CaseIterable {
    case Fly, Plan, Analyze

    var label: String { rawValue }

    var icon: Icon {
        switch self {
        case .Fly: .flight
        case .Plan: .map
        case .Analyze: .analytics
        }
    }

    static func from(_ destination: String) -> Tab? {
        switch destination.lowercased() {
        case "fly": .Fly
        case "plan": .Plan
        case "analyze": .Analyze
        default: nil
        }
    }
}

@MainActor
private enum ShellStart {
    static var started = false
}

struct MainActivity: View {
    @State private var flyScreen = FlyScreenState(layout: OverlayLayoutState(overlayLayoutStore()))
    @State private var navigation = AppNavigationState()
    @State private var openPlan = OpenPlanDocument()
    @State private var rootSize = CGSize.zero

    var body: some View {
        ZStack {
            ConnectionLocks()
            VoiceAlerts()
            AppFontScale {
                AircastShell()
            }
        }
        .background {
            Color.clear
                .ignoresSafeArea()
                .onGeometryChange(for: CGSize.self) { $0.size } action: { rootSize = $0 }
        }
        .environment(\.LocalRootSize, rootSize)
        .environment(navigation)
        .environment(openPlan)
        .environment(flyScreen)
        .environment(flyScreen.mapEdits)
        .task {
            guard !ShellStart.started else { return }
            ShellStart.started = true
            GamepadInput.start()
            VirtualStickSender.start()
        }
    }
}

private struct FlyArming: Equatable {
    let armed: Bool?
    let state: String?
}

private func paletteFollowsSystem(_ control: JSON?) -> Bool {
    guard let value = control?["value"].int else { return false }
    return value != PALETTE_INDOOR && value != PALETTE_OUTDOOR
}

private struct HostNoticeWatch: View {
    let onNotices: (JSON?) -> Void
    @QgcPath private var notices: JSON?

    init(acknowledgedThrough: Int64, onNotices: @escaping (JSON?) -> Void) {
        self.onNotices = onNotices
        _notices = QgcPath(hostNoticesPath(acknowledgedThrough))
    }

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: notices, initial: true) { _, now in onNotices(now) }
    }
}

struct AircastShell: View {
    @Environment(AppNavigationState.self) private var navigation
    @Environment(FlyScreenState.self) private var flyScreen
    @FlyIsPortrait private var portrait
    @HasVehicle private var vehicleNow
    @AdvancedUiShown private var advanced
    @AppDarkTheme private var darkBars
    @QgcPath(settingControl(PALETTE_SETTING)) private var paletteControl
    @QgcPath(FLY_STATE) private var flyStateJson
    @QgcPath(VIDEO_VIEW) private var videoJson
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @State private var tab = Tab.Fly
    @State private var hadVehicle: Bool?
    @State private var analyzePage: AnalyzePage?
    @State private var popEpoch = 0
    @State private var flyView = loadFlyView()
    @State private var videoFullScreen = false
    @State private var mapClickAt: MapPoint?
    @State private var waypointTapped: Int?
    @State private var roiTapped: TrackPoint?
    @State private var snackbars = SnackbarHostState()
    @State private var alerts = SnackbarHostState()
    @State private var acknowledgedThrough: Int64 = -1
    @State private var shownAt: [String: Int64] = [:]
    @State private var appMessages: [AppMessage] = []
    @State private var wasArmed = false
    @State private var flewWhileArmed = false
    @State private var armedBattery: Int?
    @State private var lastVehicles: VehicleChoices?
    @State private var settingsRequested: String?
    @State private var resumeDismissed: Int?

    private var flyNow: FlyState? { flyState(flyStateJson) }

    private var fullScreen: Bool {
        videoFullScreen && tab == .Fly && flyView != .Map && videoReading(videoJson)?.decoding == true
    }

    private var shownFlyView: FlyView {
        let noVideoSource = videoReading(videoJson).map { !$0.available && !$0.sourceChosen } == true
        return flyViewShown(flyView, flyNow?.armed == true, noVideoSource)
    }

    private func refuseNavigation() -> Bool {
        guard let said = navigationRefusal(navigation.blockedReason, true) else { return false }
        Task { await snackbars.showSnackbar(said) }
        return true
    }

    private func selectTab(_ entry: Tab) {
        if refuseNavigation() { return }
        if tab == entry {
            if reselectClearsAnalyze(tab, entry) { analyzePage = nil }
            popEpoch += 1
        } else {
            tab = entry
        }
    }

    var body: some View {
        let onFly = tab == .Fly
        let landscape = !portrait
        let flyLandscape = onFly && landscape
        let tabs = visibleTabs(advanced)
        let systemPalette = paletteFollowsSystem(paletteControl)
        HStack(spacing: 0) {
            if landscape && !flyLandscape {
                NavigationRail(tabs: tabs, tab: tab, settingsOpen: navigation.settingsOpen, onSelect: selectTab, onSettings: { navigation.settingsOpen = true })
            }
            VStack(spacing: 0) {
                if !onFly {
                    SnackbarHost(hostState: alerts) { AppSnackbar(data: $0) }
                }
                content(onFly: onFly, tabs: tabs)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                LogReplayBar()
                if !landscape && !onFly {
                    NavigationBar(tabs: tabs, tab: tab, settingsOpen: navigation.settingsOpen, onSelect: selectTab, onSettings: { navigation.settingsOpen = true })
                }
            }
        }
        .background(ShellBackground().ignoresSafeArea())
        .environment(\.immersive, flyLandscape)
        .statusBarHidden(flyLandscape || fullScreen)
        .persistentSystemOverlays(flyLandscape || fullScreen ? .hidden : .automatic)
        .background { HostNoticeWatch(acknowledgedThrough: acknowledgedThrough, onNotices: noticesArrived) }
        .environment(\.theme, darkBars ? .darkTheme : .lightTheme)
        .tint((darkBars ? Theme.darkTheme : Theme.lightTheme).colors.primary)
        .preferredColorScheme(systemPalette ? nil : darkBars ? .dark : .light)
        .onChange(of: tab) { _, now in if now != .Fly { flyScreen.layout.editing = false } }
        .onChange(of: navigation.setupPage) { _, page in if page != nil { navigation.openAircraft() } }
        .onChange(of: navigation.settingsPage) { _, page in if page != nil { navigation.settingsOpen = true } }
        .onChange(of: navigation.settingsOpen, initial: true) { _, open in
            guard open else { return }
            settingsRequested = navigation.settingsPage
            navigation.settingsPage = nil
        }
        .onChange(of: vehicleNow, initial: true) { _, now in
            if now && hadVehicle == false && navigation.settingsShowing == .Transmission { navigation.settingsOpen = false }
            hadVehicle = now
        }
        .onChange(of: flyView) { _, now in saveFlyView(now) }
        .onChange(of: FlyArming(armed: flyNow?.armed, state: flyNow?.state), initial: true) { armingChanged() }
        .onChange(of: vehiclesJson, initial: true) { _, json in vehiclesChanged(json) }
        .onChange(of: fullScreen) { _, now in if !now { videoFullScreen = false } }
    }

    @ViewBuilder
    private func content(onFly: Bool, tabs: [Tab]) -> some View {
        ZStack {
            if onFly {
                flyContent(tabs)
                ConnectingCard()
            }
            if tab == .Fly {
                VirtualJoystick()
                    .padding(.horizontal, 12)
                    .padding(.bottom, VIRTUAL_JOYSTICK_BOTTOM_MARGIN)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                if let point = mapClickAt {
                    MapClickMenu(point: point) { mapClickAt = nil }
                }
                if let sequence = waypointTapped {
                    SetWaypointSheet(sequence: sequence) { waypointTapped = nil }
                }
                if let at = roiTapped {
                    RoiSheet(at: at) { roiTapped = nil }
                }
            }
            MissionCompleteDialog()
            if onFly {
                ResumeFailedPrompt(dismissed: resumeDismissed) { resumeDismissed = $0 }
            }
            FirstRunDialog()
            GimbalTakeControlDialog()
            if let shown = appMessages.first {
                AppMessageDialog(message: shown, onOpenSetup: { navigation.openAircraft() }) { appMessages = Array(appMessages.dropFirst()) }
            }
            Group {
                switch tab {
                case .Plan:
                    PlanTab().background(ShellBackground().ignoresSafeArea())
                case .Analyze:
                    AnalyzeScreen(page: analyzePage, onSelect: { analyzePage = $0 })
                case .Fly:
                    EmptyView()
                }
            }
            .id(popEpoch)
            if tab == .Fly {
                OverlayEditBar()
                    .padding(8)
                    .frame(maxWidth: EDIT_BAR_MAX_WIDTH)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                    .zIndex(2)
            }
            if navigation.settingsOpen {
                SettingsSheet(requested: settingsRequested) { navigation.settingsOpen = false }
                    .zIndex(10)
            }
        }
        .overlay(alignment: .bottom) {
            SnackbarHost(hostState: snackbars) { AppSnackbar(data: $0) }
                .padding(.bottom, onFly && portrait ? flyScreen.mapInsets.bottom : 0)
        }
    }

    @ViewBuilder
    private func flyContent(_ tabs: [Tab]) -> some View {
        let status: () -> AnyView = {
            AnyView(HStack(spacing: Space.s2) {
                FlyTabMenu(destinations: tabs.map { entry in MenuDestination(label: entry.label, icon: entry.icon, current: entry == .Fly) { selectTab(entry) } }
                    + [MenuDestination(label: "Settings", icon: .settings, current: false) { navigation.settingsOpen = true }])
                LayoutWidget(key: "vehicleState", movable: false, hideable: false) { VehicleStateChip() }
                    .frame(maxWidth: .infinity, alignment: .leading)
                LayoutWidget(key: "vtolState", movable: false, hideable: false) { VtolStateCell() }
                ControlRequestPrompt()
                LayoutWidget(key: "statusPill", movable: false, hideable: false) { StatusPill() }
                FlySettingsButton()
            })
        }
        let keyRow: () -> AnyView = { AnyView(LayoutWidget(key: "obstacleArc") { ObstacleArc() }) }
        let overlays: () -> AnyView = {
            AnyView(VStack(alignment: .leading, spacing: Space.s2) {
                LayoutWidget(key: "messageBanner", hideable: false) { VehicleMessageBanner() }
                LayoutWidget(key: "fleet") { FleetCard() }
                LayoutWidget(key: "missionProgress") { MissionProgressCard() }
                LayoutWidget(key: "obstacleReadout") { ObstacleReadout() }
                LayoutWidget(key: "terrainProgress") { TerrainProgress() }
                LayoutWidget(key: "orbit") { OrbitReadout() }
                LayoutWidget(key: "followMe") { FollowMeReadout() }
                LayoutWidget(key: "rcControls") { RcControlsLayer() }
            })
        }
        let video: (Bool) -> AnyView = { expanded in
            AnyView(VideoSurface(
                expanded: expanded,
                fullScreen: videoFullScreen,
                onClick: { flyView = flyViewSwapped(flyView) },
                onDoubleTap: { videoFullScreen.toggle() }
            ))
        }
        let map: () -> AnyView = { AnyView(flyMap) }
        let actions: (FlyDeckLayout) -> AnyView = { layout in AnyView(FlightActions(layout: layout) { CameraShutters() }) }
        let onView: (FlyView) -> Void = { view in flyView = view }
        let onFullScreen: () -> Void = {
            flyView = .Video
            videoFullScreen = true
        }
        let onExitFullScreen: () -> Void = { videoFullScreen = false }
        if portrait {
            FlyPortrait(
                view: shownFlyView,
                onView: onView,
                status: status,
                video: video,
                map: map,
                keyRow: keyRow,
                rail: { thumbnailRoom in flyRail(true, thumbnailRoom) },
                overlays: overlays,
                actions: actions,
                fullScreen: fullScreen,
                onFullScreen: onFullScreen,
                onExitFullScreen: onExitFullScreen
            )
        } else {
            FlyScreen(
                view: shownFlyView,
                onView: onView,
                status: status,
                video: video,
                map: map,
                keyRow: keyRow,
                rail: { thumbnailRoom in flyRail(false, thumbnailRoom) },
                overlays: overlays,
                actions: actions,
                fullScreen: fullScreen,
                onFullScreen: onFullScreen,
                onExitFullScreen: onExitFullScreen
            )
        }
    }

    private var flyMap: some View {
        let inset = flyView == .Map || portrait
        return FlyMap(
            cameraBottomPx: 0,
            topInsetPx: inset ? flyScreen.mapInsets.top : 0,
            bottomInsetPx: inset ? flyScreen.mapInsets.bottom : 0,
            logoEndInsetPx: inset ? MAP_LAYERS_CLEARANCE : nil,
            pip: flyView == .Video && !portrait,
            onMapClick: { lat, lon in mapClickAt = MapPoint(latitude: lat, longitude: lon) },
            onMissionItemClick: { waypointTapped = $0 },
            onRoiClick: { roiTapped = $0 },
            onTrafficClick: { flyScreen.requestedSheet = TRAFFIC_SHEET },
            clickMarker: mapClickAt.map { TrackPoint(latitude: $0.latitude, longitude: $0.longitude) }
        )
    }

    private func flyRail(_ stacked: Bool, _ thumbnailRoom: Bool) -> AnyView {
        let cameraSwitch = LayoutWidget(key: "cameraSwitch") { CameraSwitch(thumbnailRoom: thumbnailRoom) }
        let cameraControl = LayoutWidget(key: "cameraControl") { CameraControlLayer(shutters: !portrait) }
        if stacked {
            return AnyView(VStack(alignment: .trailing, spacing: 0) {
                cameraSwitch.padding(.bottom, Space.s2)
                cameraControl
            })
        }
        return AnyView(HStack(spacing: 0) {
            cameraSwitch.padding(.trailing, Space.s2)
            cameraControl.frame(minWidth: RAIL_SHUTTER_ROOM)
        })
    }

    private func noticesArrived(_ notices: JSON?) {
        guard let batch = noticeBatch(notices), batch.through > acknowledgedThrough else { return }
        acknowledgedThrough = batch.through
        batch.unknownKinds.forEach { kind in
            noticeLog.warning("unrecognised notice kind '\(kind, privacy: .public)' - showing it rather than guessing")
        }
        if let destination = batch.destination {
            if let target = Tab.from(destination) {
                tab = target
            } else if destination.lowercased() == "setup" {
                navigation.openAircraft()
            } else {
                navigation.settingsOpen = true
            }
        }
        let now = Int64(Date().timeIntervalSince1970 * 1000)
        let banners = quietBanners(batch.banners, shownAt, now)
        shownAt = shownAt.merging(banners.map { ($0, now) }) { _, latest in latest }
        let queued = appMessages + batch.dialogs
        appMessages = queued.enumerated().filter { at, message in queued.firstIndex(of: message) == at }.map(\.element)
        let through = batch.through
        let critical = criticalBanner(banners.filter { batch.errorBanners.contains($0) }).flatMap { tab != .Fly ? $0 : nil }
        let ordinary = banners.filter { !batch.errorBanners.contains($0) }
        let alertHost = alerts
        let bannerHost = snackbars
        Task {
            _ = await offMain { AppCommands.acknowledgeNoticesThrough(through) }
            if let critical {
                Task {
                    await alertHost.showSnackbar(critical, withDismissAction: true, duration: .Indefinite)
                    _ = await offMain { Qgc.invoke(RESET_ERROR_LEVEL_MESSAGES) }
                }
            }
            for banner in ordinary { await bannerHost.showSnackbar(banner) }
        }
    }

    private func armingChanged() {
        let armedNow = flyNow?.connected == true && flyNow?.armed == true
        if armedNow && !wasArmed {
            Task { armedBattery = await offMain { batteryPercentNow() } }
        }
        if let said = disarmNotice(wasArmed, flewWhileArmed, armedNow) {
            let landed = flewWhileArmed
            let startBattery = armedBattery
            let host = snackbars
            Task {
                let summary = landed
                    ? await offMain {
                        landedSummary(
                            flightTimeNow(),
                            flownDistanceText(Qgc.get(VEHICLE_FLIGHT_DISTANCE)),
                            startBattery.flatMap { start in batteryPercentNow().map { start - $0 } }
                        )
                    }
                    : ""
                await host.showSnackbar([said, summary].filter { !$0.isBlank }.joined(separator: " \u{00b7} "))
            }
        }
        flewWhileArmed = armedNow && (flewWhileArmed || flyNow?.state == "flying" || flyNow?.state == "landing")
        wasArmed = armedNow
    }

    private func vehiclesChanged(_ json: JSON?) {
        let now = vehicleChoices(json)
        let notice = handoverNotice(lastVehicles, now, VehicleBridge.lastAsked)
        if activeChanged(lastVehicles, now) { VehicleBridge.forget() }
        lastVehicles = rememberedChoices(lastVehicles, now)
        if let notice {
            let host = snackbars
            Task { await host.showSnackbar(notice) }
        }
    }
}

private struct ShellBackground: View {
    @Environment(\.theme) private var theme

    var body: some View { theme.colors.surface }
}

private struct TabItem: View {
    let label: String
    let icon: Icon
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            VStack(spacing: Space.s1) {
                Image(icon)
                    .font(.system(size: 18))
                    .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
                    .frame(width: 56, height: 32)
                    .background(selected ? theme.colors.secondaryContainer : .clear, in: Capsule())
                Text(label)
                    .font(.labelMedium)
                    .foregroundStyle(selected ? theme.colors.onSurface : theme.colors.onSurfaceVariant)
            }
            .frame(maxWidth: .infinity)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }
}

private struct NavigationRail: View {
    let tabs: [Tab]
    let tab: Tab
    let settingsOpen: Bool
    let onSelect: (Tab) -> Void
    let onSettings: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: Space.s3) {
            ForEach(tabs, id: \.self) { entry in
                TabItem(label: entry.label, icon: entry.icon, selected: tab == entry && !settingsOpen) { onSelect(entry) }
            }
            TabItem(label: "Settings", icon: .settings, selected: settingsOpen, onClick: onSettings)
            Spacer(minLength: 0)
        }
        .padding(.top, Space.s3)
        .frame(width: NAVIGATION_RAIL_WIDTH)
        .frame(maxHeight: .infinity)
        .background(theme.colors.surface.ignoresSafeArea())
    }
}

private struct NavigationBar: View {
    let tabs: [Tab]
    let tab: Tab
    let settingsOpen: Bool
    let onSelect: (Tab) -> Void
    let onSettings: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 0) {
            ForEach(tabs, id: \.self) { entry in
                TabItem(label: entry.label, icon: entry.icon, selected: tab == entry && !settingsOpen) { onSelect(entry) }
            }
            TabItem(label: "Settings", icon: .settings, selected: settingsOpen, onClick: onSettings)
        }
        .frame(height: NAVIGATION_BAR_HEIGHT)
        .padding(.top, Space.s2)
        .background(theme.colors.surfaceContainer.ignoresSafeArea())
    }
}
