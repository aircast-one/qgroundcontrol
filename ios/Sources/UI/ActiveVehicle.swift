import SwiftUI

private let STATUS_BAR_SCRIM_ALPHA = 0.55

func osdModeText(_ title: String) -> String { title.components(separatedBy: " \u{00b7} ").first ?? title }

func osdStatusNote(_ title: String) -> String? {
    guard let range = title.range(of: " \u{00b7} ") else { return nil }
    let note = String(title[range.upperBound...])
    return note.isBlank ? nil : note
}

struct MvAction: Equatable {
    let id: String
    let title: String
    let confirmTitle: String
    let prompt: String
    let offer: String
    let reason: String

    var ready: Bool { offer == "ready" }
}

func mvActions(_ view: JSON?) -> [MvAction] {
    (view?["actions"].arrayOrNil ?? []).filter { $0.object != nil }.compactMap { entry in
        let id = entry["id"].string
        guard !id.isBlank else { return nil }
        let title = entry["title"].string
        return MvAction(
            id: id,
            title: title,
            confirmTitle: entry["confirmTitle"].string.ifBlank(title),
            prompt: entry["prompt"].string,
            offer: entry["offer"].string,
            reason: entry["reason"].string
        )
    }
}

func mvReasonFor(_ action: MvAction) -> String? {
    action.reason.isBlank || action.ready ? nil : action.reason
}

let STATUS_SETTINGS_PAGE = "Status Settings"
let CLOSE_VEHICLE = "vehicle.closeVehicle"

func readinessSubtitle(_ state: FlyState?, _ blocker: String?, _ failing: Int) -> String? {
    blocker.map { blocker in
        [state.flatMap { $0.mode.isBlank ? nil : $0.mode }, chipBlocker(blocker) + (failing > 1 ? " +\(failing - 1)" : "")]
            .compactMap { $0 }
            .joined(separator: " \u{00b7} ")
    }
}

let VEHICLE_CONNECT_COMPLETE = "vehicle.initialConnectComplete"
let VEHICLE_LOAD_PROGRESS = "vehicle.loadProgress"

func loadingProgress(_ complete: JSON?, _ progress: JSON?) -> Float? {
    guard case .bool(false)? = complete else { return nil }
    guard case .number(let fraction)? = progress else { return 0 }
    return Float(min(max(fraction, 0), 1))
}

@propertyWrapper
struct VehicleLoading: DynamicProperty {
    @QgcValue(VEHICLE_CONNECT_COMPLETE) private var complete
    @QgcValue(VEHICLE_LOAD_PROGRESS) private var progress

    var wrappedValue: Float? { loadingProgress(complete, progress) }
}

struct VehicleStateChip: View {
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd
    @Environment(AppNavigationState.self) private var navigation
    @Environment(FlyScreenState.self) private var flyScreen
    @QgcPath(FLY_STATE) private var flyJson
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @QgcPath(OPERATOR_CONTROL_VIEW) private var controlJson
    @QgcPath(VEHICLE_LINKS) private var linksJson
    @QgcPath(OFFLINE_STATUS_VIEW) private var offlineJson
    @QgcPath(WARNINGS) private var warningsJson
    @QgcPath(MULTI_VEHICLE_PANEL) private var panelJson
    @QgcPath(MULTI_VEHICLE_PANEL_SETTING) private var panelFact
    @FlyIsPortrait private var portrait
    @VehicleLoading private var loading
    @State private var lostMenu = false
    @State private var why = false
    @State private var picking = false
    @State private var offline = false
    @State private var statusSettings = false
    @State private var modeMenu = false
    @State private var refusal: String?

    var body: some View {
        let fly = flyState(flyJson)
        let lost = fly?.contactLost == true
        SilentSeconds(lost: lost) { silentFor in
            chip(fly, lost, silentFor)
        }
        .background {
            OpenOnRequest(name: "status", open: { statusSettings = true })
            OpenOnRequest(name: "modes", open: { modeMenu = true })
            if offline {
                OfflineStatusSheet { offline = false }
            }
            if statusSettings && fly?.connected == true {
                VehicleStatusSheet { statusSettings = false }
            }
            if why {
                VehicleMessagesSheet { why = false }
            }
            if picking {
                pickingSheet(fly)
            }
        }
        .onChange(of: fly?.connected == true, initial: true) { _, connected in if connected { offline = false } }
    }

    private func chip(_ fly: FlyState?, _ lost: Bool, _ silentFor: Int64?) -> some View {
        let choices = vehicleChoices(vehiclesJson)
        let station = controlStation(controlJson)
        let taken = controlIsElsewhere(station)
        let failsafe = vehicleLinks(linksJson)?.failsafe
        let blocker = armingBlocker(warningsJson).flatMap { fly?.connected == true && fly?.armed != true ? $0 : nil }
        let failing = (armingChecks(warningsJson) ?? []).count
        let subtitle = readinessSubtitle(fly, blocker, failing) ?? vehicleSubtitle(fly, offlineMainStatus(offlineJson))
        let disconnected = fly?.connected != true
        let tone = blocker != nil ? ChipTone.Error : chipTone(fly, lost)
        let statusBar = !portrait
        let title = lost ? signalLostTitle(silentFor, failsafe: failsafe) : activeVehicleTitle(choices, subtitle)
        return HStack(spacing: 6) {
            Image(.flight).font(.system(size: 18)).frame(width: 24, height: 24)
            Text(statusBar ? osdModeText(title) : title)
                .font(statusBar ? .titleMedium : .labelLarge)
                .lineLimit(1)
                .truncationMode(.tail)
                .layoutPriority(-1)
            if statusBar, let note = osdStatusNote(title) {
                Text(note)
                    .font(.labelLarge)
                    .lineLimit(1)
                    .foregroundStyle(noteColour(tone))
                    .padding(.horizontal, Space.s3)
                    .padding(.vertical, 6)
                    .background(Color.black.opacity(STATUS_BAR_SCRIM_ALPHA), in: RoundedRectangle(cornerRadius: Corner.small))
            }
            if flyScreen.pendingMode != nil {
                ProgressView().controlSize(.small).frame(width: 16, height: 16)
            } else if choices.ambiguous || taken {
                let alarm = lostVehiclesText(lostVehicles(choices)) ?? (taken ? controlLine(station) : nil)
                Image(alarm == nil ? .arrowDropDown : .warning)
                    .accessibilityLabel(alarm ?? "Choose which vehicle to fly")
            } else if !disconnected {
                Image(.arrowDropDown)
                    .accessibilityLabel(lost ? SIGNAL_LOST : "Change flight mode")
                    .accessibilityAddTraits(.isButton)
                    .onTapGesture { if lost { lostMenu = true } else { modeMenu = true } }
            }
            if let progress = loading, !lost {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Loading vehicle").font(.labelSmall)
                    ProgressView(value: Double(progress)).frame(width: 72)
                }
                .padding(.leading, Space.s2)
            }
        }
        .padding(.horizontal, Space.s2)
        .frame(minHeight: 32)
        .foregroundStyle(statusBar ? theme.aircast.outdoorForeground : contentColour(tone))
        .background(statusBar ? Color.clear : osdBackdrop(containerColour(tone), flyOsd), in: RoundedRectangle(cornerRadius: Corner.small))
        .contentShape(Rectangle())
        .accessibilityAddTraits(.isButton)
        .onTapGesture {
            if lost {
                lostMenu = true
            } else if choices.ambiguous || taken {
                picking = true
            } else if disconnected {
                offline = true
            } else if blocker != nil {
                why = true
            } else {
                modeMenu = true
            }
        }
        .background {
            FlightModeMenu(expanded: modeMenu && !disconnected, onDismiss: { modeMenu = false }, onStatus: { statusSettings = true }, onMessages: { why = true })
        }
        .confirmationDialog(
            silentFor.map(silenceText) ?? SIGNAL_LOST,
            isPresented: Binding(get: { lostMenu && lost }, set: { if !$0 { lostMenu = false } }),
            titleVisibility: .visible
        ) {
            Button("Disconnect", role: .destructive) {
                lostMenu = false
                Task { flyScreen.refusal = await offMain { Qgc.refusalOf(CLOSE_VEHICLE) } }
            }
        }
    }

    private func containerColour(_ tone: ChipTone) -> Color {
        switch tone {
        case .Error: theme.colors.errorContainer
        case .Neutral: theme.colors.surfaceContainerHigh
        case .Warning: theme.aircast.warningContainer
        case .Success: theme.aircast.successContainer
        }
    }

    private func contentColour(_ tone: ChipTone) -> Color {
        switch tone {
        case .Error: osdTint(theme.colors.onErrorContainer, theme.colors.error, flyOsd)
        case .Neutral: theme.colors.onSurface
        case .Warning: theme.aircast.warning
        case .Success: theme.aircast.success
        }
    }

    private func noteColour(_ tone: ChipTone) -> Color {
        switch tone {
        case .Error: theme.colors.error
        case .Neutral: theme.aircast.outdoorForeground
        case .Warning: theme.aircast.warning
        case .Success: theme.aircast.success
        }
    }

    private func pickingSheet(_ fly: FlyState?) -> some View {
        let choices = vehicleChoices(vehiclesJson)
        let station = controlStation(controlJson)
        let taken = controlIsElsewhere(station)
        let disconnected = fly?.connected != true
        let panelEnabled = multiVehiclePanelEnabled(panelJson)
        return AircastSheet(onDismissRequest: { picking = false }, skipPartiallyExpanded: true) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    Text(lostVehiclesText(lostVehicles(choices)) ?? CHOOSER_TITLE)
                        .font(.titleMedium)
                        .padding(.horizontal, Space.s6)
                        .padding(.vertical, Space.s2)
                    VehicleRows(choices: choices, selectable: panelEnabled, onRefusal: { refusal = $0 }, onSwitched: { picking = false })
                    HStack(spacing: Space.s4) {
                        Image(.add)
                        Text("Connect another vehicle").font(.bodyLarge)
                        Spacer()
                    }
                    .foregroundStyle(theme.colors.primary)
                    .padding(.horizontal, Space.s4)
                    .frame(minHeight: 56)
                    .contentShape(Rectangle())
                    .onTapGesture {
                        picking = false
                        navigation.settingsPage = CONNECTIONS_PAGE
                    }
                    if let refusal {
                        Text(refusal)
                            .font(.bodySmall)
                            .foregroundStyle(theme.colors.error)
                            .padding(.horizontal, Space.s6)
                            .padding(.vertical, Space.s3)
                    }
                    if panelToggleShown(panelFact) {
                        Toggle(isOn: Binding(get: { panelEnabled }, set: { wanted in
                            Task { refusal = await offMain { Qgc.writeRefusal(MULTI_VEHICLE_PANEL_SETTING, wanted) } }
                        })) {
                            Text("Enable multi-vehicle panel").font(.bodyLarge)
                        }
                        .padding(.horizontal, Space.s4)
                        .frame(minHeight: 56)
                    }
                    if !disconnected && !taken {
                        HStack {
                            Button("Flight mode") {
                                picking = false
                                modeMenu = true
                            }
                            Button("Vehicle status") {
                                picking = false
                                statusSettings = true
                            }
                        }
                        .buttonStyle(.borderless)
                        .padding(.horizontal, Space.s4)
                    }
                    ControlHolderNote(station: station, vehicleId: activeVehicleId(vehiclesJson), onRefusal: { refusal = $0 })
                    FootNote(text: "Tap a name to fly that aircraft. Arm, Takeoff and every action on the flight screen go to the one you pick.")
                    if panelEnabled {
                        FleetControls(view: vehiclesJson, choices: choices, onRefusal: { refusal = $0 })
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }
}

private struct FleetControls: View {
    @Environment(\.theme) private var theme
    let view: JSON?
    let choices: VehicleChoices
    let onRefusal: (String?) -> Void
    @State private var confirming: MvAction?

    var body: some View {
        if choices.ambiguous {
            VStack(alignment: .leading, spacing: 0) {
                Text(fleetHeading(selectedIds(choices)))
                    .font(.titleSmall)
                    .padding(.horizontal, Space.s6)
                    .padding(.vertical, Space.s2)
                HStack {
                    Button("Select all") {
                        let all = choices
                        offMain { _ = FleetBridge.selectAll(all) }
                    }
                    .disabled(!choices.canSelectAll)
                    Button("Deselect all") { offMain { _ = FleetBridge.deselectAll() } }
                        .disabled(!choices.canDeselectAll)
                }
                .buttonStyle(.borderless)
                .padding(.horizontal, Space.s4)
                Text("Multi vehicle actions")
                    .font(.titleSmall)
                    .padding(.horizontal, Space.s6)
                    .padding(.vertical, Space.s2)
                ForEach(mvActions(view), id: \.id) { action in
                    StatusListItem(
                        headline: action.title,
                        headlineColor: action.ready ? theme.colors.onSurface : theme.colors.onSurfaceVariant,
                        supporting: confirming?.id == action.id ? nil : fleetActionLine(action)
                    ) { EmptyView() }
                        .contentShape(Rectangle())
                        .onTapGesture { if action.ready { confirming = action } }
                    if confirming?.id == action.id {
                        ConfirmTrack(
                            action: GuidedAction(
                                name: action.confirmTitle,
                                confirm: action.prompt,
                                destructive: fleetIsDestructive(action),
                                run: {
                                    let confirmed = Set(choices.choices.filter(\.selected).map(\.id))
                                    Task {
                                        let sent = await offMain { FleetBridge.command(action.id, confirmed) }
                                        onRefusal(sent ? nil : "\(action.title) did not reach every selected aircraft.")
                                    }
                                }
                            ),
                            optionChecked: .constant(false),
                            onSent: { confirming = nil },
                            onCancel: { confirming = nil }
                        )
                        .padding(.horizontal, Space.s6)
                        .padding(.vertical, Space.s2)
                    }
                }
            }
        }
    }
}

private struct VehicleModeMenu: View {
    let choice: VehicleChoice
    let onRefusal: (String?) -> Void

    var body: some View {
        Menu("Mode") {
            ForEach(choice.flightModes, id: \.self) { mode in
                Button(mode) {
                    let path = vehicleFlightModePath(choice)
                    let id = choice.id
                    Task { onRefusal(await offMain { Qgc.writeForVehicleRefusal(path, mode, vehicle: id) }) }
                }
            }
        }
    }
}

func fleetPanelShown(_ vehicleCount: Int, _ panelEnabled: Bool) -> Bool { vehicleCount >= 2 && panelEnabled }

struct FleetPanel: View {
    @Environment(\.theme) private var theme
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @QgcPath(MULTI_VEHICLE_PANEL) private var panelJson
    @State private var refusal: String?

    var body: some View {
        let choices = vehicleChoices(vehiclesJson)
        if fleetPanelShown(choices.choices.count, multiVehiclePanelEnabled(panelJson)) {
            VStack(alignment: .leading, spacing: 0) {
                VehicleRows(choices: choices, selectable: true, onRefusal: { refusal = $0 }, onSwitched: {})
                FleetControls(view: vehiclesJson, choices: choices, onRefusal: { refusal = $0 })
                if let refusal {
                    Text(refusal)
                        .font(.bodySmall)
                        .foregroundStyle(theme.colors.error)
                        .padding(.horizontal, Space.s6)
                        .padding(.vertical, Space.s2)
                }
            }
        }
    }
}

private struct VehicleRows: View {
    @Environment(\.theme) private var theme
    let choices: VehicleChoices
    let selectable: Bool
    let onRefusal: (String?) -> Void
    let onSwitched: () -> Void

    var body: some View {
        let distinguishes = linkDistinguishes(choices.choices)
        ForEach(choices.choices) { choice in
            HStack(spacing: Space.s4) {
                if selectable {
                    Toggle("", isOn: Binding(get: { choice.selected }, set: { wanted in
                        let id = choice.id
                        offMain { _ = FleetBridge.setSelected(id, wanted) }
                    }))
                    .labelsHidden()
                    .toggleStyle(VehicleCheckbox())
                }
                VStack(alignment: .leading, spacing: 0) {
                    Text(choice.name).font(.bodyLarge).lineLimit(1)
                    Text(vehicleChoiceLine(choice, distinguishes)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(1)
                    if let telemetry = vehicleTelemetryLine(choice) {
                        Text(telemetry).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(1)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                HStack(spacing: Space.s2) {
                    if !choice.flightModes.isEmpty && choice.index >= 0 {
                        VehicleModeMenu(choice: choice, onRefusal: onRefusal)
                    }
                    VehicleRowCompass(heading: choice.heading, armed: choice.armed)
                    if choice.active {
                        Image(.check).accessibilityLabel("Flying this one")
                    }
                }
            }
            .padding(.leading, Space.s4)
            .padding(.trailing, Space.s6)
            .padding(.vertical, Space.s2)
            .frame(minHeight: 72)
            .contentShape(Rectangle())
            .onTapGesture {
                guard !choice.active else { return }
                let id = choice.id
                Task {
                    let switched = await offMain { VehicleBridge.askFor(id) }
                    let refused = await offMain { VehicleBridge.lastRefusal }
                    onRefusal(switched ? nil : refused ?? "That vehicle did not take control.")
                    if switched { onSwitched() }
                }
            }
            Divider()
        }
    }
}

private struct VehicleCheckbox: ToggleStyle {
    func makeBody(configuration: Configuration) -> some View {
        Button { configuration.isOn.toggle() } label: {
            Image(systemName: configuration.isOn ? "checkmark.square.fill" : "square")
                .font(.title3)
                .frame(width: 40, height: 40)
        }
        .buttonStyle(.plain)
    }
}

func selectedIds(_ choices: VehicleChoices) -> [Int] { choices.choices.filter(\.selected).map(\.id).sorted() }

func fleetHeading(_ selectedIds: [Int]) -> String {
    "Vehicles Selected: \(selectedIds.map(String.init).joined(separator: ", ").ifEmpty("-"))"
}

func fleetActionLine(_ action: MvAction) -> String {
    mvReasonFor(action) ?? action.prompt.ifBlank(action.title)
}

func fleetIsDestructive(_ action: MvAction) -> Bool { action.id != "mvPause" }

private func nowMillis() -> Int64 { Int64(Date().timeIntervalSince1970 * 1000) }

private struct ControlHolderNote: View {
    @Environment(\.theme) private var theme
    @Environment(FlyScreenState.self) private var flyScreen
    let station: ControlStation?
    let vehicleId: Int?
    let onRefusal: (String?) -> Void

    var body: some View {
        if let holder = station {
            if holder.inControl == true {
                InControlNote(holder: holder, onRefusal: onRefusal)
            } else if let line = holderLine(holder) {
                held(holder, line)
            }
        }
    }

    private func held(_ holder: ControlStation, _ line: String) -> some View {
        let deadlineKey = vehicleId ?? 0
        let requestEndsAt = flyScreen.controlRequestDeadlines[deadlineKey]
        return VStack(alignment: .leading, spacing: 0) {
            Text(line).font(.bodyMedium).padding(.horizontal, Space.s6).padding(.vertical, Space.s2)
            if let takeover = takeoverLine(holder) {
                Text(takeover).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.horizontal, Space.s6)
            }
            ControlSectionTitle(holder: holder)
            if let endsAt = requestEndsAt {
                RequestCountdown(endsAt: endsAt) { flyScreen.controlRequestDeadlines.removeValue(forKey: deadlineKey) }
            }
            AllowTakeoverBox(holder: holder, onRefusal: onRefusal)
            if requestEndsAt == nil, let waiting = controlWaitLine(holder) {
                Text(waiting)
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(.horizontal, Space.s6)
                    .padding(.vertical, Space.s2)
            }
            if let label = acquireLabel(holder) {
                Button(label) {
                    Task {
                        let ask = await offMain { askForControl(holder) }
                        onRefusal(ask.refusal)
                        if ask.refusal == nil && ask.timeoutSeconds > 0 {
                            flyScreen.controlRequestDeadlines[deadlineKey] = nowMillis() + Int64(ask.timeoutSeconds) * 1000
                        }
                    }
                }
                .buttonStyle(.borderless)
                .disabled(!acquireEnabled(holder, requestEndsAt != nil))
                .padding(.horizontal, Space.s4)
            }
            if requestTimeoutEditable(holder) {
                SettingFactRow(path: REQUEST_TIMEOUT_PATH, title: "Request timeout (sec)")
            }
            SettingFactRow(path: GCS_SYSTEM_ID_PATH, title: "This GCS MAVLink system ID")
        }
        .onChange(of: holder.takeoverAllowed, initial: true) { _, allowed in
            if allowed == true { flyScreen.controlRequestDeadlines.removeValue(forKey: deadlineKey) }
        }
    }
}

private struct ControlSectionTitle: View {
    let holder: ControlStation

    var body: some View {
        if let title = controlSectionTitle(holder) {
            Text(title).font(.titleSmall).padding(.horizontal, Space.s6).padding(.vertical, Space.s2)
        }
    }
}

private struct SettingFactRow: View {
    let path: String
    let title: String
    @QgcPath private var control: JSON?

    init(path: String, title: String) {
        self.path = path
        self.title = title
        _control = QgcPath(settingControl(path))
    }

    var body: some View {
        if let control, control["kind"].string == "object", let fact = factFromControl(control) {
            FactRow(fact: fact, title: title, subtitle: factSubtitle(fact), fieldModifier: EdgeInsets(top: 8, leading: 24, bottom: 8, trailing: 24))
        }
    }
}

private let COUNTDOWN_TICK_MS = 100

private struct RequestCountdown: View {
    @Environment(\.theme) private var theme
    let endsAt: Int64
    let onDone: () -> Void
    @State private var now = nowMillis()

    var body: some View {
        Text(requestSentLabel(endsAt - now))
            .font(.bodySmall)
            .foregroundStyle(theme.colors.onSurfaceVariant)
            .padding(.horizontal, Space.s6)
            .padding(.vertical, Space.s2)
            .task(id: endsAt) {
                now = nowMillis()
                while now < endsAt && !Task.isCancelled {
                    try? await Task.sleep(for: .milliseconds(COUNTDOWN_TICK_MS))
                    now = nowMillis()
                }
                if !Task.isCancelled { onDone() }
            }
    }
}

private struct AllowTakeoverBox: View {
    let holder: ControlStation
    let onRefusal: (String?) -> Void
    var onRead: (Bool?) -> Void = { _ in }
    @QgcValue(ALLOW_TAKEOVER_SETTING) private var served
    @State private var typed: Bool?

    var body: some View {
        let stored: Bool? = served.isNull ? nil : truthy(served)
        let allow = typed ?? stored
        Toggle(isOn: Binding(get: { allow == true }, set: { wanted in
            typed = wanted
            onRead(wanted)
            Task {
                let refusal = await offMain { saveAllowTakeover(wanted) }
                onRefusal(refusal)
                if refusal != nil { typed = nil }
            }
        })) {
            Text("Allow takeover").font(.bodyMedium)
        }
        .toggleStyle(VehicleCheckboxTrailing())
        .disabled(allow == nil || !allowTakeoverEditable(holder))
        .padding(.horizontal, Space.s6)
        .padding(.vertical, Space.s1)
        .onChange(of: stored, initial: true) { _, now in
            typed = nil
            onRead(now)
        }
    }
}

private struct VehicleCheckboxTrailing: ToggleStyle {
    func makeBody(configuration: Configuration) -> some View {
        HStack {
            configuration.label.frame(maxWidth: .infinity, alignment: .leading)
            Button { configuration.isOn.toggle() } label: {
                Image(systemName: configuration.isOn ? "checkmark.square.fill" : "square")
                    .font(.title3)
                    .frame(width: 40, height: 40)
            }
            .buttonStyle(.plain)
        }
    }
}

private struct InControlNote: View {
    @Environment(\.theme) private var theme
    let holder: ControlStation
    let onRefusal: (String?) -> Void
    @State private var allow: Bool?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let line = inControlLine(holder) {
                Text(line).font(.bodyMedium).foregroundStyle(theme.colors.primary).padding(.horizontal, Space.s6).padding(.vertical, Space.s2)
            }
            if let takeover = takeoverLine(holder) {
                Text(takeover).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.horizontal, Space.s6)
            }
            ControlSectionTitle(holder: holder)
            AllowTakeoverBox(holder: holder, onRefusal: onRefusal, onRead: { allow = $0 })
            Button("Change") {
                guard let wanted = allow else { return }
                Task { onRefusal(await offMain { changeTakeover(wanted) }) }
            }
            .buttonStyle(.borderless)
            .disabled(!takeoverChangeable(holder, allow))
            .padding(.horizontal, Space.s4)
            SettingFactRow(path: GCS_SYSTEM_ID_PATH, title: "This GCS MAVLink system ID")
        }
    }
}

let MULTI_VEHICLE_PANEL_SETTING = "settings.appSettings.enableMultiVehiclePanel"
let MULTI_VEHICLE_PANEL = "\(MULTI_VEHICLE_PANEL_SETTING).rawValue"

func panelToggleShown(_ fact: JSON?) -> Bool { fact?["visible"].bool(true) != false }

func multiVehiclePanelEnabled(_ setting: JSON?) -> Bool {
    guard let setting, setting.has("value") else { return true }
    return setting["value"].bool(true)
}

func activeVehicleId(_ view: JSON?) -> Int? {
    guard let view, !view["activeId"].isNull else { return nil }
    let id = view["activeId"].int(-1)
    return id > 0 ? id : nil
}

private let ROW_COMPASS_SIZE: CGFloat = 28
private let ROW_HEADING_COLOUR = Color(hex: 0xEE3424)
let DISARMED_ALPHA = 0.5

func rowCompassAlpha(_ armed: Bool) -> Double { armed ? 1 : DISARMED_ALPHA }

private let ROW_HEADING_SHADE = Color(hex: 0xC72B27)

private struct VehicleRowCompass: View {
    @Environment(\.theme) private var theme
    let heading: Double
    let armed: Bool

    var body: some View {
        Canvas { context, size in
            let radius = min(size.width, size.height) / 2
            let center = CGPoint(x: size.width / 2, y: size.height / 2)
            context.fill(Path(ellipseIn: CGRect(x: center.x - radius, y: center.y - radius, width: radius * 2, height: radius * 2)), with: .color(theme.colors.surface))
            context.stroke(Path(ellipseIn: CGRect(x: center.x - radius + 0.5, y: center.y - radius + 0.5, width: radius * 2 - 1, height: radius * 2 - 1)), with: .color(theme.colors.onSurface), lineWidth: 1)
            guard heading.isFinite else { return }
            let half = radius / 3
            let top = center.y - half
            let bottom = center.y + half
            let notch = center.y + half * 0.5
            let turn = CGAffineTransform(translationX: center.x, y: center.y)
                .rotated(by: heading * .pi / 180)
                .translatedBy(x: -center.x, y: -center.y)
            let right = Path { path in
                path.addLines([CGPoint(x: center.x, y: top), CGPoint(x: center.x + half, y: bottom), CGPoint(x: center.x, y: notch)])
                path.closeSubpath()
            }.applying(turn)
            let left = Path { path in
                path.addLines([CGPoint(x: center.x, y: top), CGPoint(x: center.x - half, y: bottom), CGPoint(x: center.x, y: notch)])
                path.closeSubpath()
            }.applying(turn)
            context.fill(right, with: .color(ROW_HEADING_COLOUR))
            context.fill(left, with: .color(ROW_HEADING_SHADE))
            context.stroke(right, with: .color(ROW_HEADING_COLOUR), lineWidth: 1)
            context.stroke(left, with: .color(ROW_HEADING_COLOUR), lineWidth: 1)
        }
        .frame(width: ROW_COMPASS_SIZE, height: ROW_COMPASS_SIZE)
        .opacity(rowCompassAlpha(armed))
    }
}
