import SwiftUI

enum AnalyzeSection: CaseIterable {
    case FlightData, Live, Vehicle

    var title: String {
        switch self {
        case .FlightData: "Flight data"
        case .Live: "Live"
        case .Vehicle: "Vehicle"
        }
    }
}

enum AnalyzePage: CaseIterable, Hashable {
    case LogDownload, GeoTag, Vibration, Inspector, Console, Messages, Firmware

    var label: String {
        switch self {
        case .LogDownload: "Flight logs"
        case .GeoTag: "Geotag images"
        case .Vibration: "Vibration"
        case .Inspector: "MAVLink inspector"
        case .Console: "Console"
        case .Messages: "Messages"
        case .Firmware: "Firmware"
        }
    }

    var description: String {
        switch self {
        case .LogDownload: "Download flight logs from the vehicle"
        case .GeoTag: "Match photos to the flight log"
        case .Vibration: "Accelerometer vibration levels and clipping"
        case .Inspector: "Live message rates and field values"
        case .Console: "Vehicle shell over MAVLink"
        case .Messages: "What the vehicle has said since it connected"
        case .Firmware: "Flash a board through its bootloader over USB"
        }
    }

    var section: AnalyzeSection {
        switch self {
        case .LogDownload, .GeoTag: .FlightData
        case .Vibration, .Inspector, .Console, .Messages: .Live
        case .Firmware: .Vehicle
        }
    }

    var icon: Icon {
        switch self {
        case .LogDownload: .download
        case .GeoTag: .photoCamera
        case .Vibration: .vibration
        case .Inspector: .analytics
        case .Console: .terminal
        case .Messages: .description
        case .Firmware: .developerBoard
        }
    }

    var requiresVehicle: Bool {
        switch self {
        case .LogDownload, .Vibration, .Inspector, .Console: true
        case .GeoTag, .Messages, .Firmware: false
        }
    }
}

func analyzeStatus(_ page: AnalyzePage, _ unread: Int, _ vibration: String?) -> (String, SetupState) {
    if page == .Messages && unread > 0 { return ("\(unread)", .NeedsAttention) }
    if page == .Vibration && ["danger", "warning"].contains(vibration) { return (severityLabel(vibration), .NeedsAttention) }
    if page == .Vibration && vibration == "normal" { return (severityLabel(vibration), .Done) }
    return ("", .Neutral)
}

func analyzeSubtitle(_ page: AnalyzePage, _ messages: [VehicleMessage], vibration: VibrationReading? = nil, rate: String? = nil) -> String {
    switch page {
    case .Inspector: rate ?? page.description
    case .Messages where !messages.isEmpty: severitySummary(messages)
    case .Vibration: vibration.flatMap(vibrationGlance) ?? page.description
    default: page.description
    }
}

let REQUIRES_VEHICLE = "Requires a connected vehicle"

func analyzeGate(_ page: AnalyzePage, _ connected: Bool) -> String? {
    page.requiresVehicle && !connected ? REQUIRES_VEHICLE : nil
}

func analyzeNote(_ page: AnalyzePage, _ connected: Bool, _ vibration: String?) -> String? {
    connected && page == .Vibration ? vibration : nil
}

private struct AnalyzePageList: View {
    let onSelect: (AnalyzePage) -> Void
    var selected: AnalyzePage? = nil
    @Environment(\.theme) private var theme
    @HasVehicle private var connected
    @QgcPath(VIBRATION_VIEW) private var vibrationJson
    @QgcPath(MESSAGES) private var messagesJson
    @QgcPath(INSPECTOR_VIEW) private var inspectorJson

    var body: some View {
        let caveat = vibrationCaveat(vibrationJson)
        let vibration = vibrationReading(vibrationJson)
        let rate = inspectorRateText(inspectorJson)
        let vibrationLevel = vibration.flatMap(worstSeverity)
        let unread = unreadCount(messagesJson)
        let messages = vehicleMessages(messagesJson)
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                VStack(alignment: .leading, spacing: 0) {
                    Text("Analyze").font(.headlineMedium)
                    Text("Logs and tools for the connected vehicle")
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                }
                .padding(EdgeInsets(top: 20, leading: 16, bottom: 8, trailing: 16))
                ForEach(AnalyzeSection.allCases.filter { section in AnalyzePage.allCases.contains { $0.section == section } }, id: \.self) { section in
                    SectionHeader(text: section.title)
                    ForEach(AnalyzePage.allCases.filter { $0.section == section }, id: \.self) { page in
                        let (status, state) = analyzeStatus(page, unread, vibrationLevel)
                        SetupRow(
                            title: page.label,
                            status: status,
                            state: state,
                            onClick: { onSelect(page) },
                            icon: page.icon,
                            subtitle: [analyzeSubtitle(page, messages, vibration: vibration, rate: rate), analyzeNote(page, connected, caveat)]
                                .compactMap { $0 }
                                .joined(separator: "\n"),
                            selected: page == selected
                        )
                    }
                }
            }
        }
    }
}

struct AnalyzeScreen: View {
    let page: AnalyzePage?
    let onSelect: (AnalyzePage?) -> Void
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme
    @State private var toast: String?

    private func switchTo(_ next: AnalyzePage?) {
        if page != nil, let refused = navigationRefusal(navigation.blockedReason, true) {
            toast = refused
        } else {
            onSelect(next)
        }
    }

    private func leave() { switchTo(nil) }

    var body: some View {
        GeometryReader { box in
            let wide = box.size.width >= LIST_DETAIL_MIN_WIDTH
            HStack(spacing: 0) {
                if wide {
                    AnalyzePageList(onSelect: { switchTo($0) }, selected: page)
                        .frame(width: LIST_PANE_WIDTH)
                        .background(theme.colors.surfaceContainerLow)
                }
                Group {
                    if let page {
                        AnalyzePageBody(page: page, leave: leave)
                            .frame(maxWidth: wide ? DETAIL_PANE_MAX_WIDTH : .infinity)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    } else if wide {
                        EmptyState(icon: .analytics, title: "Analyze", text: "Choose a tool on the left.")
                    } else {
                        AnalyzePageList(onSelect: { onSelect($0) })
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                .background(theme.colors.surface)
                .environment(\.LocalTwoPane, wide)
            }
        }
        .overlay(alignment: .bottom) {
            if let toast {
                Text(toast)
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.inverseOnSurface)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                    .background(theme.colors.inverseSurface, in: Capsule())
                    .padding(.bottom, 24)
                    .transition(.opacity)
            }
        }
        .task(id: toast) {
            guard toast != nil, (try? await Task.sleep(for: .seconds(2))) != nil else { return }
            toast = nil
        }
    }
}

private struct AnalyzePageBody: View {
    let page: AnalyzePage
    let leave: () -> Void
    @Environment(\.theme) private var theme
    @HasVehicle private var connected
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson

    var body: some View {
        let gate = analyzeGate(page, connected)
        let reloadKey = page.requiresVehicle ? activeVehicleId(vehiclesJson) : nil
        VStack(spacing: 0) {
            PageTopBar(title: page.label, backLabel: "Back to Analyze", onBack: leave)
            Group {
                if let gate {
                    EmptyState(icon: page.icon, title: gate, text: "")
                } else {
                    shown.id(reloadKey)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .background(theme.colors.surface)
        }
    }

    @ViewBuilder private var shown: some View {
        switch page {
        case .LogDownload: LogDownloadScreen()
        case .Console: ConsoleScreen()
        case .Inspector: InspectorScreen()
        case .Vibration: VibrationScreen()
        case .GeoTag: GeoTagScreen()
        case .Messages: VehicleMessagesPage()
        case .Firmware: FirmwareScreen()
        }
    }
}
