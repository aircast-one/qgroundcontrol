import SwiftUI

struct FirmwareLine: Equatable {
    var summary: String
    var vehicleType: String
}

func firmwareLine(_ view: JSON?) -> FirmwareLine {
    FirmwareLine(summary: view?["summary"].string ?? "", vehicleType: view?["vehicleType"].string ?? "")
}

struct SetupComponent: Equatable, Hashable {
    var index: Int
    var name: String
    var known: String? = nil
    var className: String = ""
    var needsAttention: Bool
    var blockedReason: String? = nil
    var prerequisite: String? = nil
}

func setupIcon(_ known: String?, className: String = "") -> Icon {
    switch known {
    case "radio", "joystick": .gamepad
    case "flightModes": .toggleOn
    case "sensors": .sensors
    case "safety": .shield
    case "power": .bolt
    case "esc": .tune
    default: COMPONENT_ICONS.first { className.contains($0.token) }?.icon ?? .build
    }
}

func setupSubtitle(_ vehicle: String, _ firmware: String) -> String {
    [firmware, vehicle].filter { !$0.isBlank }.joined(separator: " · ")
}

func readinessNote(_ readiness: SetupReadiness?, _ listed: Bool) -> String? {
    guard let readiness, readiness.setupComplete != true, !listed else { return nil }
    let note = [readiness.headline, readiness.detail].filter { !$0.isBlank }.joined(separator: ". ")
    return note.isBlank ? nil : note
}

func parameterCountText(_ count: Int) -> String? {
    count > 0 ? "\(count.formatted(.number.locale(Locale(identifier: "en_US")))) parameters" : nil
}

func attentionAction(_ className: String) -> String {
    ["Sensors", "Radio"].contains { className.contains($0) } ? "Calibrate" : "Set up"
}

func setupNote(_ className: String) -> String {
    COMPONENT_NOTES.first { className.contains($0.token) }?.note ?? ""
}

private let COMPONENT_NOTES: [(token: String, note: String)] = [
    ("Failsafe", "What it does when the battery, radio or link fails"),
    ("FlightSafety", "Return altitude, landing speed and geofence"),
    ("Safety", "Return altitude, landing speed and geofence"),
    ("FlightModes", "Which mode each switch position selects"),
    ("Airframe", "The frame type the autopilot flies"),
    ("SubFrame", "The frame type the autopilot flies"),
    ("Gimbal", "Camera mount axes and their limits"),
    ("Joystick", "A gamepad as the stick instead of a radio"),
    ("Logging", "What the autopilot records and when"),
    ("Motor", "Spin each motor to check order and direction"),
    ("Power", "Battery monitor, capacity and voltage"),
    ("Radio", "Calibrate the sticks and switches"),
    ("RemoteSupport", "Share telemetry with a support engineer"),
    ("Scripting", "Lua scripts running on the autopilot"),
    ("Sensors", "Compass, accelerometer and level"),
    ("AdvancedTuning", "Every rate and filter, per axis"),
    ("Tuning", "How it responds to the sticks"),
    ("Actuator", "Outputs and what drives them"),
    ("Servo", "Outputs and what drives them"),
    ("Follow", "Follow a target or this phone"),
    ("Camera", "Camera trigger and gimbal"),
]

private let COMPONENT_ICONS: [(token: String, icon: Icon)] = [
    ("Failsafe", .warning),
    ("Airframe", .flight),
    ("SubFrame", .flight),
    ("Gimbal", .photoCamera),
    ("Logging", .description),
    ("Motor", .speed),
    ("RemoteSupport", .link),
    ("Scripting", .terminal),
    ("Tuning", .tune),
    ("Actuator", .speed),
    ("Servo", .tune),
    ("Follow", .myLocation),
    ("Airspeed", .speed),
    ("Lights", .bolt),
    ("ESP8266", .wifi),
    ("Syslink", .wifi),
]

func prerequisiteText(_ first: String, _ wanted: String) -> String {
    "\(sentenceCase(first)) has to be set up before \(sentenceCase(wanted))."
}

struct ParameterWait: Equatable {
    var title: String
    var body: String
    var downloadOffered: Bool = false
}

let PARAMETER_REFRESH = "parameterTools.refresh"

let PARAMETERS_STOPPED = "Setup needs them. Disconnect and connect the link to ask again."

func parameterWait(_ view: JSON?) -> ParameterWait? {
    guard let view, !view["parametersReady"].bool else { return nil }
    let reason = view["parametersReason"].string
    switch reason {
    case "", "noVehicle": return nil
    case "loading": return ParameterWait(title: "Loading parameters from the vehicle.", body: "")
    case "skipped": return ParameterWait(title: view["parametersText"].string, body: "", downloadOffered: true)
    default:
        return ParameterWait(
            title: view["parametersText"].string.ifBlank("This vehicle has not sent its parameters (\(reason))."),
            body: PARAMETERS_STOPPED
        )
    }
}

func parametersIncomplete(_ view: JSON?) -> String? {
    guard let view, view["parametersReason"].string == "incomplete" else { return nil }
    return "Parameters Incomplete. \(view["parametersText"].string)"
}

let SETUP_PARAMETERS_PAGE = "Parameters"
let SETUP_OVERVIEW_PAGE = ""
private let NAVIGATION_SETTLE_MS = 500
let SETUP_SUMMARY = "view.setupSummary"
let SETUP_SUMMARY_PAGE = "Summary"
private let SETUP_SUMMARY_POLL_MS = 2000

func setupSummaries(_ view: JSON?) -> [String: [SummaryLine]] {
    let listed = (view?["components"].arrayOrNil ?? []).filter { $0.object != nil }
    return Dictionary(
        listed.map { component in
            (component["name"].string, component["rows"].array.filter { $0.object != nil }.map {
                SummaryLine(label: $0["label"].string, value: $0["value"].string, warn: $0["warn"].bool)
            })
        },
        uniquingKeysWith: { _, last in last }
    )
}

func setupMatches(_ name: String, _ search: String) -> Bool {
    search.isBlank || name.lowercased().contains(search.trimmed.lowercased())
}

func splitDesktopOnly(_ components: [SetupComponent], _ opensHere: (SetupComponent) -> Bool) -> (first: [SetupComponent], second: [SetupComponent]) {
    (components.filter(opensHere), components.filter { !opensHere($0) })
}

func remainingSetup(_ components: [SetupComponent]) -> [SetupComponent] {
    components.filter { !$0.needsAttention }
}

private func presentText(_ element: JSON, _ key: String) -> String? {
    element[key].isNull || element[key].string.isBlank ? nil : element[key].string
}

func setupComponents(_ view: JSON?) -> [SetupComponent] {
    (view?["components"].arrayOrNil ?? []).enumerated().compactMap { index, element in
        guard element.object != nil, !element["name"].string.isBlank else { return nil }
        return SetupComponent(
            index: index,
            name: element["name"].string,
            known: presentText(element, "known"),
            className: element["className"].string,
            needsAttention: element["needsAttention"].bool,
            blockedReason: presentText(element, "blockedReason"),
            prerequisite: presentText(element, "prerequisite")
        )
    }
}

private let OWN_SCREEN_HEADS: Set<String> = [SENSORS, RADIO, REMOTE_SUPPORT, MOTORS, FLIGHT_MODES_PAGE]

private struct SectionHits: View {
    let titles: [String]
    let onOpen: (String) -> Void

    var body: some View {
        ForEach(titles, id: \.self) { title in
            Button(sentenceCase(title)) { onOpen(title) }
                .buttonStyle(.borderless)
                .padding(.leading, 56)
                .padding(.vertical, Space.s2)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

func disabledWhile(_ reason: String) -> String { "Disabled while the vehicle is \(reason)" }

extension View {
    func swallowTouches() -> some View {
        contentShape(Rectangle()).onTapGesture {}.gesture(DragGesture(minimumDistance: 0))
    }
}

private struct SetupNotice: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.bodyLarge)
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            .padding(Space.s6)
    }
}

func reportsOpening(_ prerequisite: String?, _ page: SetupPage?) -> Bool {
    prerequisite != nil || (page?.parameterSections != true && page?.screen != NOT_SUPPORTED_SCREEN)
}

struct SetupDialog<Content: View, Buttons: View>: View {
    let title: String
    @ViewBuilder let content: () -> Content
    @ViewBuilder let buttons: () -> Buttons
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s4) {
            Text(title).font(.headlineSmall)
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s3) { content() }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s6)
            }
            .padding(.horizontal, -Space.s6)
            HStack(spacing: Space.s2) {
                Spacer(minLength: 0)
                buttons()
            }
        }
        .padding(Space.s6)
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
        .presentationBackground(theme.colors.surfaceContainerLow)
    }
}

func presented<Value>(_ value: Binding<Value?>) -> Binding<Bool> {
    Binding(get: { value.wrappedValue != nil }, set: { if !$0 { value.wrappedValue = nil } })
}

private struct NavigationRequest: Equatable {
    let page: String?
    let components: [SetupComponent]
    let hasVehicle: Bool
}

private struct AutoOpen: Equatable {
    let components: [SetupComponent]
    let page: String?
    let twoPane: Bool
}

struct SetupScreen: View {
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme
    @QgcPath(SETUP) private var setupJson
    @QgcPath(FIRMWARE) private var firmwareJson
    @AdvancedUiShown private var advanced
    @State private var openComponent: SetupComponent?
    @State private var openSection: String?
    @State private var parametersOpen = false
    @State private var setupSearch = ""
    @State private var parametersSearch = ""
    @State private var formSections: [String: [ParameterRows]] = [:]
    @State private var summaries: [String: [SummaryLine]] = [:]

    private var hasVehicle: Bool { setupReadiness(setupJson)?.connected == true }

    var body: some View {
        let components = setupComponents(setupJson)
        content(components)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .onChange(of: hasVehicle) { _, connected in
                guard !connected else { return }
                summaries = [:]
                openComponent = nil
                openSection = nil
            }
            .task(id: setupSearch.isBlank ? [] : components) {
                let searching = !setupSearch.isBlank
                let json = setupJson
                let sections = searching ? await offMain {
                    Dictionary(
                        components
                            .filter { setupPage(json, $0.name)?.parameterSections == true && !OWN_SCREEN_HEADS.contains(headPage($0)) }
                            .map { ($0.name, readPage($0.name)) },
                        uniquingKeysWith: { _, last in last }
                    )
                } : [:]
                guard !Task.isCancelled else { return }
                formSections = sections
            }
            .task(id: NavigationRequest(page: navigation.setupPage, components: components, hasVehicle: hasVehicle)) {
                await follow(navigation.setupPage, components)
            }
    }

    private func follow(_ requested: String?, _ components: [SetupComponent]) async {
        guard let requested, hasVehicle, !components.isEmpty else { return }
        if requested == SETUP_PARAMETERS_PAGE {
            parametersSearch = ""
            parametersOpen = true
        } else if let found = components.first(where: { $0.name == requested }) {
            openComponent = found
            openSection = nil
        }
        try? await Task.sleep(for: .milliseconds(NAVIGATION_SETTLE_MS))
        guard !Task.isCancelled else { return }
        navigation.setupPage = nil
    }

    private func openFromList(_ component: SetupComponent, _ section: String?) {
        parametersOpen = false
        openComponent = component
        openSection = section
    }

    @ViewBuilder
    private func content(_ components: [SetupComponent]) -> some View {
        if !hasVehicle {
            VStack(spacing: 0) {
                EmptyState(icon: .build, title: LOOKING_TITLE, text: LOOKING_HINT)
                Button("Set up connection") { navigation.settingsPage = "Connections" }.buttonStyle(.bordered)
            }
            .frame(maxWidth: .infinity)
        } else if let waiting = parameterWait(setupJson) {
            VStack(spacing: Space.s2) {
                SetupNotice(text: waiting.title)
                if !waiting.body.isBlank {
                    Text(waiting.body)
                        .font(.bodyMedium)
                        .multilineTextAlignment(.center)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .frame(maxWidth: .infinity)
                        .padding(.horizontal, Space.s6)
                }
                if waiting.downloadOffered {
                    Button("Download parameters") { offMain { _ = Qgc.invoke(PARAMETER_REFRESH) } }
                        .buttonStyle(.borderedProminent)
                }
            }
        } else {
            GeometryReader { geometry in
                let twoPane = geometry.size.width >= LIST_DETAIL_MIN_WIDTH
                let detailShown = hasDetail
                HStack(spacing: 0) {
                    if twoPane || !detailShown {
                        overview(components)
                            .frame(width: twoPane ? LIST_PANE_WIDTH : nil)
                            .background(twoPane ? theme.colors.surfaceContainerLow : .clear)
                    }
                    if twoPane || detailShown {
                        ZStack(alignment: .topLeading) {
                            if detailShown {
                                detail.frame(maxWidth: twoPane ? DETAIL_PANE_MAX_WIDTH : .infinity)
                            } else {
                                EmptyState(icon: .build, title: AIRCRAFT_SETUP, text: "Choose a component on the left.")
                            }
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                        .environment(\.LocalTwoPane, twoPane)
                    }
                }
                .task(id: AutoOpen(components: components, page: navigation.setupPage, twoPane: twoPane)) {
                    guard twoPane, openComponent == nil, !parametersOpen, navigation.setupPage == nil else { return }
                    openComponent = components.first { headCanOpen(setupPage(setupJson, $0.name), $0.name) }
                    openSection = nil
                }
            }
        }
    }

    private var hasDetail: Bool {
        parametersOpen || openComponent.map { headCanOpen(setupPage(setupJson, $0.name), $0.name) } == true
    }

    @ViewBuilder
    private var detail: some View {
        if parametersOpen {
            VStack(spacing: 0) {
                SetupPageBar(title: "Parameters") { parametersOpen = false }
                ParametersScreen(initialSearch: parametersSearch)
            }
            .id("parameters")
        } else if let open = openComponent {
            ComponentPage(
                open: open,
                section: openSection,
                setupJson: setupJson,
                onBack: { openComponent = nil },
                onOpen: { first in
                    openComponent = setupComponents(setupJson).first { $0.name == first }
                    openSection = nil
                }
            )
            .id(open.name)
        }
    }

    private func overview(_ components: [SetupComponent]) -> some View {
        SetupOverview(
            components: components,
            setupJson: setupJson,
            firmwareJson: firmwareJson,
            advanced: advanced,
            setupSearch: $setupSearch,
            sectionHits: formSections.mapValues { rows in rows.filter { sectionMatches($0, setupSearch) }.map(\.title) },
            summaries: $summaries,
            openComponent: openComponent,
            parametersOpen: parametersOpen,
            onOpen: openFromList,
            onParameters: { search in
                parametersSearch = search
                openComponent = nil
                parametersOpen = true
            }
        )
    }
}

private struct SetupOverview: View {
    let components: [SetupComponent]
    let setupJson: JSON?
    let firmwareJson: JSON?
    let advanced: Bool
    @Binding var setupSearch: String
    let sectionHits: [String: [String]]
    @Binding var summaries: [String: [SummaryLine]]
    let openComponent: SetupComponent?
    let parametersOpen: Bool
    let onOpen: (SetupComponent, String?) -> Void
    let onParameters: (String) -> Void
    @State private var parameterCount = 0

    private func searchHit(_ component: SetupComponent) -> Bool {
        setupMatches(component.name, setupSearch) || !(sectionHits[component.name] ?? []).isEmpty
    }

    var body: some View {
        let setup = setupReadiness(setupJson)
        let line = firmwareLine(firmwareJson)
        let needSetup = components.filter { $0.needsAttention && searchHit($0) }
        let remaining = remainingSetup(components).filter(searchHit)
        let split = splitDesktopOnly(remaining) { headCanOpen(setupPage(setupJson, $0.name), headPage($0)) }
        let parametersAreReady = parametersReady(setupJson)
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                ReadinessHeader(
                    vehicle: line.vehicleType.ifBlank("Vehicle"),
                    firmware: line.summary,
                    vehicleId: setup?.vehicleId,
                    note: readinessNote(setup, components.contains { $0.needsAttention })
                )
                SearchPill(
                    value: setupSearch,
                    onValueChange: { setupSearch = $0 },
                    placeholder: "Search",
                    keyboardOptions: .search,
                    keyboardActions: {
                        if advanced && !setupSearch.isBlank { onParameters(setupSearch.trimmed) }
                    }
                )
                if !needSetup.isEmpty {
                    SectionHeader(text: "Needs attention")
                    ForEach(needSetup, id: \.index) { component in
                        attentionRow(component)
                    }
                }
                if components.isEmpty {
                    if let incomplete = parametersIncomplete(setupJson) {
                        SetupNotice(text: incomplete)
                    } else {
                        EmptyState(icon: .build, title: NOTHING_TO_CONFIGURE, text: NOTHING_TO_CONFIGURE_TEXT)
                    }
                } else if !remaining.isEmpty {
                    if !split.first.isEmpty { SectionHeader(text: "Ready") }
                    ForEach(split.first, id: \.index) { readyRow($0) }
                    if !split.second.isEmpty {
                        SectionHeader(text: "Advanced")
                        ForEach(split.second, id: \.index) { readyRow($0) }
                    }
                }
                if advanced && setupMatches("Parameters", setupSearch) {
                    if split.second.isEmpty { SectionHeader(text: "Advanced") }
                    SetupRow(
                        title: "Parameters",
                        state: .Neutral,
                        onClick: { onParameters("") },
                        icon: .tune,
                        subtitle: parameterCountText(parameterCount) ?? "Every setting the vehicle has",
                        selected: parametersOpen
                    )
                }
            }
        }
        .task {
            _ = await offMain { Qgc.invoke(SETUP_PAGE_OPENED, SETUP_SUMMARY_PAGE) }
            while !Task.isCancelled {
                summaries = await offMain { setupSummaries(Qgc.get(SETUP_SUMMARY)) }
                try? await Task.sleep(for: .milliseconds(SETUP_SUMMARY_POLL_MS))
            }
        }
        .task(id: parametersAreReady) {
            let count = parametersAreReady ? await offMain { parameterNames().count } : 0
            guard !Task.isCancelled else { return }
            parameterCount = count
        }
    }

    private func attentionRow(_ component: SetupComponent) -> some View {
        let blocked = component.blockedReason
        let page = setupPage(setupJson, component.name)
        let summary = summaries[component.name] ?? []
        return VStack(spacing: 0) {
            SetupRow(
                title: sentenceCase(component.name),
                status: blocked.map { "Not while \($0)" } ?? attentionAction(component.className),
                state: blocked != nil ? .Unavailable : .NeedsAttention,
                onClick: headCanOpen(page, component.name) ? { onOpen(component, nil) } : nil,
                summary: summary,
                icon: setupIcon(component.known, className: component.className),
                subtitle: summary.isEmpty ? setupNote(component.className) : "",
                selected: component == openComponent
            )
            SectionHits(titles: sectionHits[component.name] ?? []) { onOpen(component, $0) }
        }
    }

    private func readyRow(_ component: SetupComponent) -> some View {
        let page = setupPage(setupJson, component.name)
        let openable = headCanOpen(page, headPage(component))
        let blocked = component.blockedReason
        let summary = summaries[component.name] ?? []
        let status = blocked.map { "Not while \($0)" }
            ?? (component.needsAttention ? attentionAction(component.className) : !openable ? "On desktop" : "")
        let state: SetupState = blocked != nil ? .Unavailable : component.needsAttention ? .NeedsAttention : !openable ? .Unavailable : .Neutral
        return VStack(spacing: 0) {
            SetupRow(
                title: sentenceCase(component.name),
                status: status,
                state: state,
                onClick: openable ? { onOpen(component, nil) } : nil,
                summary: summary,
                icon: setupIcon(component.known, className: component.className),
                subtitle: summary.isEmpty ? setupNote(component.className) : "",
                selected: component == openComponent
            )
            SectionHits(titles: sectionHits[component.name] ?? []) { onOpen(component, $0) }
        }
    }
}

private struct ComponentPage: View {
    let open: SetupComponent
    let section: String?
    let setupJson: JSON?
    let onBack: () -> Void
    let onOpen: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let nativePage = setupPage(setupJson, open.name)
        let reportsLookups = reportsOpening(open.prerequisite, nativePage)
        VStack(spacing: 0) {
            SetupPageBar(title: sentenceCase(open.name), onBack: onBack)
            if let blocked = open.blockedReason {
                Text(disabledWhile(blocked))
                    .bold()
                    .foregroundStyle(theme.aircast.warning)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s4)
                    .padding(.vertical, Space.s2)
                ZStack {
                    page(nativePage)
                    Color.black.opacity(0.5).swallowTouches()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                page(nativePage).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            }
        }
        .task(id: "\(open.name)|\(reportsLookups)") {
            guard reportsLookups else { return }
            let name = open.name
            _ = await offMain { Qgc.invoke(SETUP_PAGE_OPENED, name) }
        }
    }

    @ViewBuilder
    private func page(_ nativePage: SetupPage?) -> some View {
        let head = headPage(open)
        if let first = open.prerequisite {
            let firstComponent = setupComponents(setupJson).first { $0.name == first }
            VStack(spacing: 0) {
                EmptyState(
                    icon: setupIcon(firstComponent?.known, className: firstComponent?.className ?? ""),
                    title: "\(sentenceCase(first)) first",
                    text: prerequisiteText(first, open.name)
                )
                Button("Set up \(sentenceCase(first))") { onOpen(first) }.buttonStyle(.borderedProminent)
                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity)
        } else if head == SENSORS {
            SensorsScreen()
        } else if head == RADIO {
            RadioScreen()
        } else if head == REMOTE_SUPPORT {
            RemoteSupportScreen()
        } else if nativePage?.screen == APM_SUB_MOTORS_SCREEN {
            ApmSubMotorsScreen()
        } else if head == MOTORS {
            MotorsScreen()
        } else if head == FLIGHT_MODES_PAGE {
            FlightModesSetup()
        } else if let screen = nativePage?.screen, let shown = nativeScreen(screen) {
            shown
        } else if nativePage?.parameterSections == true {
            VStack(spacing: 0) {
                if open.known == "power" { PowerLiveCard() }
                ParameterForm(page: open.name, section: section)
            }
        } else {
            SetupNotice(text: "\(open.name) is set up on the desktop.")
        }
    }

    private func nativeScreen(_ screen: String) -> AnyView? {
        switch screen {
        case PX4_TUNING_SCREEN: AnyView(Px4TuningScreen())
        case PX4_AIRFRAME_SCREEN: AnyView(Px4AirframeScreen())
        case ACTUATORS_SCREEN: AnyView(ActuatorsScreen())
        case APM_SERVOS_SCREEN: AnyView(ApmServosScreen())
        case APM_FOLLOW_SCREEN: AnyView(ApmFollowScreen())
        case SCRIPTING_SCREEN: AnyView(ScriptingScreen())
        case JOYSTICK_SCREEN: AnyView(JoystickScreen())
        case ESP_BRIDGE_SCREEN: AnyView(EspBridgeScreen())
        case SYSLINK_SCREEN: AnyView(SyslinkScreen())
        case APM_SUB_FRAME_SCREEN: AnyView(ApmSubFrameScreen())
        case APM_AIRFRAME_SCREEN: AnyView(ApmAirframeScreen())
        case OPTICAL_FLOW_SCREEN: AnyView(OpticalFlowScreen())
        case NOT_SUPPORTED_SCREEN: AnyView(SetupNotice(text: "Not supported"))
        default: nil
        }
    }
}

private struct SetupPageBar: View {
    let title: String
    let onBack: () -> Void
    @Environment(\.LocalPageHeading) private var pageHeading
    @Environment(\.LocalTwoPane) private var twoPane

    var body: some View {
        if pageHeading != nil && !twoPane {
            OverridePageHeading(title: title, onBack: onBack)
        } else {
            PageTopBar(title: title, backLabel: "Back to \(AIRCRAFT_SETUP)", onBack: onBack)
        }
    }
}

private struct ReadinessHeader: View {
    let vehicle: String
    let firmware: String
    let vehicleId: Int?
    let note: String?
    @Environment(\.LocalPageHeading) private var pageHeading
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if pageHeading == nil { Text(AIRCRAFT_SETUP).font(.headlineMedium) }
            Text(setupSubtitle(vehicle, firmware)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            if let vehicleId {
                Text("Vehicle \(vehicleId)").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if let note {
                Text(note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s4)
        .padding(.top, Space.s5)
        .padding(.bottom, Space.s2)
    }
}
