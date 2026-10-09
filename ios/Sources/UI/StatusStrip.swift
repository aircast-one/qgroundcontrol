import SwiftUI

private let BATTERY = "view.battery"
private let VEHICLE_FLIGHT_TIME = "vehicle.flightTime"
private let GPS_VIEW = "view.gps"

enum BatteryLevel { case Normal, Caution, Warning, Critical }

struct BatteryReading: Equatable {
    let text: String
    let level: BatteryLevel
    var packs: Int = 1
}

func packCountText(_ packs: Int) -> String { packs > 1 ? "\u{00d7}\(packs)" : "" }

func batteryLevelOf(_ name: String?) -> BatteryLevel {
    switch name {
    case "critical": .Critical
    case "warning": .Warning
    case "caution": .Caution
    default: .Normal
    }
}

func batteryReadings(_ view: JSON?) -> [BatteryReading] {
    guard let view, view["available"].bool, let packs = view["indicatorPacks"].arrayOrNil else { return [] }
    return packs.filter { $0.object != nil }.compactMap { pack in
        let lines = pack["indicatorLines"].array.map(\.string).filter { !$0.isBlank }
        guard !lines.isEmpty else { return nil }
        let label = pack["indicatorLabel"].string
        return BatteryReading(
            text: [label.isEmpty ? nil : label, lines.joined(separator: " · ")].compactMap { $0 }.joined(separator: " "),
            level: batteryLevelOf(pack["level"].string),
            packs: pack["packCount"].int(1)
        )
    }
}

struct RcCell: Equatable {
    let text: String
    let lost: Bool
}

func rcCell(_ state: FlyState?) -> RcCell? {
    guard let state, state.rcSupported, !state.rcSignalText.isBlank else { return nil }
    return RcCell(text: "\(state.rcSignalText) RC", lost: state.rcSignal == 0)
}

enum FixLevel { case None, TwoD, Good }

func fixLevel(_ lock: Double) -> FixLevel? {
    if lock.isNaN { return nil }
    if lock < 2 { return FixLevel.None }
    return lock < 3 ? .TwoD : .Good
}

let NO_COUNT = "--"

func satsText(_ fix: FixLevel, _ count: String) -> String {
    switch fix {
    case .None: "No fix"
    case .TwoD: count.isBlank ? "2D only" : "\(count) sats · 2D only"
    case .Good: count.isBlank ? NO_COUNT : "\(count) sats"
    }
}

private func batteryLevelColour(_ level: BatteryLevel, _ theme: Theme) -> Color? {
    switch level {
    case .Normal: nil
    case .Caution: theme.aircast.warning
    case .Warning: theme.aircast.alert
    case .Critical: theme.colors.error
    }
}

private func gpsColour(_ fix: FixLevel, _ theme: Theme) -> Color? {
    switch fix {
    case .None: theme.colors.error
    case .TwoD: theme.aircast.warning
    case .Good: nil
    }
}

extension EnvironmentValues {
    @Entry var LocalCompactStatus = false
    @Entry var LocalNarrowStatus = false
}

extension View {
    @ViewBuilder
    func foregroundOrInherited(_ color: Color?) -> some View {
        if let color { foregroundStyle(color) } else { self }
    }
}

struct StatusListItem<Trailing: View>: View {
    @Environment(\.theme) private var theme
    let headline: String
    var headlineColor: Color? = nil
    var supporting: String? = nil
    var supportingColor: Color? = nil
    @ViewBuilder var trailing: () -> Trailing

    var body: some View {
        HStack(spacing: Space.s4) {
            VStack(alignment: .leading, spacing: 2) {
                Text(headline).font(.bodyLarge).foregroundStyle(headlineColor ?? theme.colors.onSurface)
                if let supporting {
                    Text(supporting).font(.bodyMedium).foregroundStyle(supportingColor ?? theme.colors.onSurfaceVariant)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            trailing()
                .font(.labelMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
        }
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
        .frame(minHeight: 56)
    }
}

struct StatusPill: View {
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd
    @RtkStatusWatch private var rtk
    @GcsBattery private var gcsBattery
    @HasVehicle private var hasVehicle
    @FlyIsPortrait private var portrait

    var body: some View {
        if statusPillShown(hasVehicle, rtk != nil, gcsBattery != nil) {
            Group {
                if portrait {
                    readings(narrow: true)
                } else {
                    ViewThatFits(in: .horizontal) {
                        readings(narrow: false).frame(idealWidth: NARROW_PILL_WIDTH)
                        readings(narrow: true)
                    }
                }
            }
            .foregroundStyle(theme.aircast.outdoorForeground)
            .background(osdBackdrop(Color.black.opacity(0.45), flyOsd), in: Capsule())
            .layoutPriority(1)
        }
    }

    private func readings(narrow: Bool) -> some View {
        StatusReadingsInline(rtk: rtk, gcsBattery: gcsBattery)
            .padding(.trailing, stripGap(narrow))
            .padding(.vertical, 6)
            .environment(\.LocalCompactStatus, true)
            .environment(\.LocalNarrowStatus, narrow)
    }
}

private let STRIP_GAP: CGFloat = 14
private let NARROW_STRIP_GAP: CGFloat = 6
private let NARROW_PILL_WIDTH: CGFloat = 120
private let NARROW_STATUS_CELLS = 1

private func stripGap(_ narrow: Bool) -> CGFloat { narrow ? NARROW_STRIP_GAP : STRIP_GAP }

private let STATUS_CELLS = ["battery", "flightTime", "gps", "rc", "rcOverride", "telemetry", "links", "aircastLink", "esc", "joystick", "remoteId", "gpsResilience", "rtk", "gcsBattery", "gimbal", "supportForwarding"]

func shownStatusCells(_ narrow: Bool) -> Int { narrow ? NARROW_STATUS_CELLS : COMPACT_STATUS_CELLS }

func statusPillShown(_ vehicle: Bool, _ rtk: Bool, _ gcsBattery: Bool) -> Bool { vehicle || rtk || gcsBattery }

struct StatusReadingsInline: View {
    let rtk: RtkStatus?
    let gcsBattery: GcsBatteryReading?
    @HasVehicle private var available

    var body: some View {
        if available {
            VehicleStatusReadings(rtk: rtk, gcsBattery: gcsBattery)
        } else {
            HStack(spacing: 0) {
                RtkIndicatorCell(status: rtk).padding(.leading, STRIP_GAP)
                GcsBatteryCell(reading: gcsBattery).padding(.leading, STRIP_GAP)
            }
        }
    }
}

private struct VehicleStatusReadings: View {
    let rtk: RtkStatus?
    let gcsBattery: GcsBatteryReading?
    @Environment(\.theme) private var theme
    @Environment(AppNavigationState.self) private var navigation
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.LocalCompactStatus) private var compactStatus
    @Environment(\.LocalNarrowStatus) private var narrow
    @AdvancedUiShown private var advanced
    @QgcPath(FLY_STATE) private var stateJson
    @QgcPath(BATTERY) private var batteryJson
    @QgcPath(VEHICLE_LINKS) private var linksJson
    @QgcPath(GPS_VIEW) private var gpsJson
    @QgcPath(SETUP) private var setupJson
    @State private var detail: StripDetail?
    @State private var batterySettings = false
    @State private var allStatus = false
    @State private var batteryDisplay = false
    @State private var rtkSettings = false
    @State private var contentWidth: CGFloat?

    var body: some View {
        let state = flyState(stateJson)
        let live = state?.staleNotice.isBlank ?? true
        let layout = flyScreen.layout
        let ordered = orderedKeys(STATUS_CELLS, layout.indicatorOrder)
        let compact = compactStatus && !layout.editing
        let keys = compact ? Array(ordered.prefix(shownStatusCells(narrow))) : ordered
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(keys, id: \.self) { key in
                    LayoutWidget(key: "indicator-\(key)", movable: false) {
                        if layout.editing {
                            HStack(spacing: 0) {
                                Button("\u{2039}") { movedKey(keys, key, -1).map { layout.saveIndicatorOrder($0) } }
                                cell(key, state, live)
                                Button("\u{203A}") { movedKey(keys, key, 1).map { layout.saveIndicatorOrder($0) } }
                            }
                            .buttonStyle(.borderless)
                            .padding(.leading, stripGap(narrow))
                        } else {
                            cell(key, state, live).padding(.leading, stripGap(narrow))
                        }
                    }
                }
                if compact && !narrow {
                    Image(.chevronRight)
                        .padding(.leading, STRIP_GAP)
                        .contentShape(Rectangle())
                        .onTapGesture { allStatus = true }
                        .accessibilityLabel("All status")
                        .accessibilityAddTraits(.isButton)
                }
            }
            .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { contentWidth = $0 }
        }
        .scrollBounceBehavior(.basedOnSize, axes: .horizontal)
        .frame(maxWidth: contentWidth)
        .opacity(live ? 1 : 0.45)
        .background {
            OpenOnRequest(name: "status-all", open: { allStatus = true })
            if allStatus {
                AircastSheet(onDismissRequest: { allStatus = false }) {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 14) {
                            Text("Status").font(.titleLarge)
                            ForEach(ordered, id: \.self) { key in cell(key, state, live) }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, Space.s6)
                        .padding(.vertical, Space.s2)
                    }
                    .environment(\.LocalCompactStatus, false)
                    .background { stripSheets(state) }
                }
            } else {
                stripSheets(state)
            }
        }
    }

    @ViewBuilder
    private func stripSheets(_ state: FlyState?) -> some View {
        SilentSeconds(lost: vehicleLinks(linksJson)?.contactLost == true) { seconds in
            if let shown = detail {
                detailSheet(shown, state, seconds.map(silenceText))
            }
        }
        if rtkSettings {
            AircastSheet(onDismissRequest: { rtkSettings = false }) {
                RtkSettingsSheetContent(status: rtk)
            }
        }
        if batterySettings {
            AircastSheet(onDismissRequest: { batterySettings = false }) {
                if let wait = indicatorParameterWait(setupJson) {
                    Text(wait).padding(.horizontal, Space.s6).padding(.vertical, Space.s4)
                } else {
                    ParameterForm(page: BATTERY_SETTINGS_PAGE)
                }
            }
        }
        if batteryDisplay {
            AircastSheet(onDismissRequest: { batteryDisplay = false }) {
                BatteryDisplaySettings()
            }
        }
    }

    @ViewBuilder
    private func cell(_ key: String, _ state: FlyState?, _ live: Bool) -> some View {
        switch key {
        case "battery":
            BatteryCells(batteries: batteryReadings(batteryJson), live: live) { detail = .Battery }
        case "flightTime":
            CompactFlightTime()
        case "gps":
            if let gps = gpsStatus(gpsJson) {
                let fix = fixLevel(gps.lock)
                InlineCell(text: fix.map { satsText($0, gps.satellites.map(String.init) ?? "") } ?? NO_COUNT, colour: fix.flatMap { gpsColour($0, theme) }, icon: .satelliteAlt) { detail = .Gps }
            }
        case "rc":
            if let cell = rcCell(state) {
                InlineCell(text: cell.text, colour: cell.lost ? theme.colors.error : nil, icon: .gamepad) { detail = .Rc }
            }
        case "rcOverride":
            if let cell = overrideCell(state) {
                InlineCell(text: cell.text, colour: theme.aircast.warning) { offMainInOrder { Qgc.invoke(CLEAR_RC_OVERRIDES) } }
            }
        case "telemetry":
            if let text = telemetryCell(state) {
                InlineCell(text: text, colour: nil, icon: .sensors) { detail = .Telemetry }
            }
        case "links":
            if let cell = linkCell(vehicleLinks(linksJson)) {
                InlineCell(text: cell.text, colour: cell.degraded ? theme.aircast.warning : nil, icon: .signalCellularAlt) { detail = .Links }
            }
        case "aircastLink": AircastLinkCell()
        case "esc": EscIndicatorCell()
        case "joystick": JoystickIndicatorCell()
        case "remoteId": RemoteIdIndicatorCell()
        case "gpsResilience": GpsResilienceCell()
        case "rtk": RtkIndicatorCell(status: rtk)
        case "gcsBattery": GcsBatteryCell(reading: gcsBattery)
        case "gimbal": GimbalIndicatorCell()
        case "supportForwarding": SupportForwardingCell()
        default: EmptyView()
        }
    }

    private func detailSheet(_ shown: StripDetail, _ state: FlyState?, _ silence: String?) -> some View {
        let hasPowerSetup = setupComponents(setupJson).contains { $0.name == POWER_SETUP_PAGE }
        let battery = shown == .Battery
        let rows: [DetailRow] = switch shown {
        case .Battery: [totalDraw(batteryJson).map { DetailRow(label: "Total draw", value: $0) }].compactMap { $0 } + batteryDetail(batteryJson)
        case .Gps: gpsDetail(gpsStatus(gpsJson))
        case .Telemetry: telemetryDetail(state?.telemetry)
        case .Rc: rcDetail(state)
        case .Links: linkDetail(vehicleLinks(linksJson), linkNames(linksJson), linksJson?["primary"].string)
        }
        let footer: AnyView? = battery ? AnyView(VStack(alignment: .leading, spacing: 0) {
            Button("Battery failsafes") { detail = nil; batterySettings = true }.padding(.horizontal, Space.s3)
            Button("Battery display") { detail = nil; batteryDisplay = true }.padding(.horizontal, Space.s3)
            if hasPowerSetup && advanced {
                Button("Power setup") { detail = nil; navigation.setupPage = POWER_SETUP_PAGE }.padding(.horizontal, Space.s3)
            }
        }.buttonStyle(.borderless)) : nil
        let action: AnyView? = switch shown {
        case .Battery: batteryReturnOffered(batteryJson) ? AnyView(BatteryReturnButton { detail = nil }) : nil
        case .Gps: AnyView(Button("RTK GPS settings") { detail = nil; rtkSettings = true }.buttonStyle(.borderless).padding(.horizontal, Space.s3))
        default: nil
        }
        return InstrumentSheet(
            title: instrumentTitle(shown),
            rows: rows,
            headline: battery ? batteryHeadline(batteryJson) : nil,
            silence: silence,
            action: action,
            footer: footer,
            onDismiss: { detail = nil }
        )
    }
}

private struct BatteryCells: View {
    @Environment(\.theme) private var theme
    @Environment(\.LocalCompactStatus) private var compactStatus
    @Environment(\.LocalNarrowStatus) private var narrow
    let batteries: [BatteryReading]
    let live: Bool
    let onClick: () -> Void

    var body: some View {
        if !batteries.isEmpty {
            HStack(spacing: BATTERY_GAP) {
                ForEach(Array(batteries.enumerated()), id: \.offset) { _, reading in
                    let colour = live ? batteryLevelColour(reading.level, theme) : theme.colors.outline
                    if compactStatus {
                        BatteryRing(text: reading.text, colour: colour, count: narrow ? "" : packCountText(reading.packs), onClick: onClick)
                    } else {
                        InlineCell(text: [reading.text, packCountText(reading.packs)].filter { !$0.isEmpty }.joined(separator: " "), colour: colour, icon: .battery5Bar, onClick: onClick)
                    }
                }
            }
        }
    }
}

private struct CompactFlightTime: View {
    @Environment(\.LocalCompactStatus) private var compactStatus

    var body: some View {
        if compactStatus { FlightTimeCell() }
    }
}

let BATTERY_SETTINGS_PAGE = "Battery Settings"

func indicatorParameterWait(_ setup: JSON?) -> String? {
    if parametersReady(setup) { return nil }
    return setup?["parametersReason"].string == "skipped" ? "Parameters not available" : "Waiting for parameters…"
}

private let BATTERY_INDICATOR_SETTINGS = "settings.batteryIndicatorSettings"
let BATTERY_DISPLAY_FACTS = ["valueDisplay", "threshold1", "threshold2"]

func batteryDisplayFacts(_ facts: [Fact]) -> [Fact] {
    BATTERY_DISPLAY_FACTS.compactMap { name in facts.first { $0.name == name } }
}

private struct BatteryDisplaySettings: View {
    @QgcFacts(BATTERY_INDICATOR_SETTINGS) private var facts

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Battery display").font(.titleMedium).padding(.horizontal, Space.s6).padding(.vertical, Space.s2)
            ForEach(batteryDisplayFacts(facts)) { fact in FactRow(fact: fact) }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.bottom, Space.s6)
    }
}

let CLEAR_RC_OVERRIDES = "vehicle.clearRcChannelOverrides"
let POWER_SETUP_PAGE = "Power"

enum StripDetail { case Battery, Gps, Links, Telemetry, Rc }

func rcDetail(_ state: FlyState?) -> [DetailRow] {
    [state.flatMap { $0.rcSignalText.isBlank ? nil : DetailRow(label: "RSSI", value: $0.rcSignalText) }].compactMap { $0 }
}

func instrumentTitle(_ instrument: StripDetail) -> String {
    switch instrument {
    case .Battery: "Battery"
    case .Gps: "Vehicle GPS status"
    case .Links: "Links to this aircraft"
    case .Telemetry: "Telemetry RSSI status"
    case .Rc: "RC RSSI status"
    }
}

private struct InstrumentSheet: View {
    @Environment(\.theme) private var theme
    let title: String
    let rows: [DetailRow]
    var headline: BatteryHeadline? = nil
    var silence: String? = nil
    var action: AnyView? = nil
    var footer: AnyView? = nil
    let onDismiss: () -> Void

    var body: some View {
        let stale = silence != nil
        let muted = theme.colors.onSurfaceVariant
        AircastSheet(onDismissRequest: onDismiss) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    Text(title).font(.titleMedium).padding(.horizontal, Space.s6).padding(.vertical, Space.s2)
                    if let silence {
                        Text(silence).font(.bodyMedium).bold().foregroundStyle(theme.colors.error).padding(.horizontal, Space.s6).padding(.vertical, Space.s1)
                    }
                    if let worst = headline {
                        VStack(alignment: .leading) {
                            Text(worst.text).font(.headlineMedium).bold().foregroundStyle(stale ? muted : headlineColour(worst))
                            if !worst.detail.isBlank { Text(worst.detail).font(.bodyMedium).foregroundStyle(muted) }
                            if !worst.margin.isBlank { Text(worst.margin).font(.bodyMedium).foregroundStyle(stale ? muted : theme.colors.onSurface) }
                        }
                        .padding(.horizontal, Space.s6)
                        .padding(.vertical, Space.s1)
                    }
                    action
                    if rows.isEmpty {
                        Text("The vehicle has not reported anything else about this yet.")
                            .font(.bodyMedium)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                            .padding(.horizontal, Space.s6)
                            .padding(.vertical, Space.s3)
                    } else {
                        ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                            StatusListItem(headline: row.label, headlineColor: row.severity == SEVERITY_SECONDARY ? theme.colors.onSurfaceVariant : nil) {
                                Text(row.value).foregroundStyle(stale ? muted : severityColour(row.severity))
                            }
                        }
                    }
                    footer
                    FootNote(text: "Readings come from the aircraft and stop updating when it stops answering.")
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private func headlineColour(_ headline: BatteryHeadline) -> Color {
        headline.severity > 0 ? severityColour(headline.severity) : batteryLevelColour(headline.level, theme) ?? theme.colors.onSurface
    }

    private func severityColour(_ severity: Int) -> Color {
        if severity >= 2 { return theme.colors.error }
        if severity == 1 { return theme.aircast.warning }
        return severity == SEVERITY_SECONDARY ? theme.colors.onSurfaceVariant : theme.colors.onSurface
    }
}

private struct InlineCell: View {
    @Environment(\.theme) private var theme
    @Environment(\.LocalCompactStatus) private var compactStatus
    let text: String
    let colour: Color?
    var icon: Icon? = nil
    var onClick: (() -> Void)? = nil

    var body: some View {
        let tint = colour ?? theme.colors.onSurfaceVariant
        let shown = compactStatus && icon != nil ? compactStatusText(text) : text
        HStack(spacing: Space.s1) {
            if let icon {
                Image(icon).font(.system(size: 14)).frame(width: 18, height: 18)
            }
            if !shown.isEmpty {
                Text(shown).font(.labelMedium).lineLimit(1)
            }
        }
        .foregroundStyle(tint)
        .contentShape(Rectangle())
        .onTapGesture { onClick?() }
        .accessibilityAddTraits(onClick == nil ? [] : .isButton)
    }
}

private let BATTERY_RING_SIZE: CGFloat = 30
private let BATTERY_RING_STROKE: CGFloat = 3
private let BATTERY_RING_TRACK_ALPHA = 0.25

func batteryPercent(_ text: String) -> Int? {
    Int(compactStatusText(text).removingSuffix("%")).map { min(max($0, 0), 100) }
}

func batteryPercentNow() -> Int? { batteryReadings(Qgc.get(BATTERY)).first.flatMap { batteryPercent($0.text) } }

func flightTimeNow() -> Double? { flightTimeSeconds(Qgc.get(VEHICLE_FLIGHT_TIME)) }

func flightTimeText(_ seconds: Double?) -> String {
    let whole = seconds.flatMap { $0.isFinite && $0 > 0 ? Int($0) : nil } ?? 0
    return String(format: "%02d'%02d\"", whole / 60, whole % 60)
}

private let FLIGHT_TIME_POLL_MS = 1000

func flightTimeSeconds(_ view: JSON?) -> Double? {
    if case .number(let seconds)? = view?["value"] { return seconds }
    return nil
}

private struct FlightTimeCell: View {
    @Environment(\.theme) private var theme
    @State private var seconds: Double?

    var body: some View {
        Text(flightTimeText(seconds))
            .font(.titleSmall)
            .monospacedDigit()
            .foregroundStyle(theme.aircast.outdoorForeground)
            .accessibilityLabel("Flight time")
            .task {
                while !Task.isCancelled {
                    seconds = await offMain { flightTimeSeconds(Qgc.get(VEHICLE_FLIGHT_TIME)) }
                    try? await Task.sleep(for: .milliseconds(FLIGHT_TIME_POLL_MS))
                }
            }
    }
}

private struct BatteryRing: View {
    @Environment(\.theme) private var theme
    let text: String
    let colour: Color?
    let count: String
    let onClick: () -> Void

    var body: some View {
        if let percent = batteryPercent(text) {
            ZStack {
                Circle()
                    .stroke(theme.aircast.outdoorForeground.opacity(BATTERY_RING_TRACK_ALPHA), style: StrokeStyle(lineWidth: BATTERY_RING_STROKE, lineCap: .round))
                Circle()
                    .trim(from: 0, to: CGFloat(percent) / 100)
                    .stroke(colour ?? theme.aircast.success, style: StrokeStyle(lineWidth: BATTERY_RING_STROKE, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                Text("\(percent)")
                    .font(.labelSmall)
                    .monospacedDigit()
                    .foregroundStyle(theme.aircast.outdoorForeground)
            }
            .padding(BATTERY_RING_STROKE / 2)
            .frame(width: BATTERY_RING_SIZE, height: BATTERY_RING_SIZE)
            .overlay(alignment: .bottomTrailing) {
                if !count.isEmpty {
                    Text(count)
                        .font(.system(size: BATTERY_COUNT_SIZE, weight: .medium))
                        .foregroundStyle(theme.aircast.outdoorForeground)
                        .offset(x: BATTERY_COUNT_ROOM)
                }
            }
            .padding(.trailing, count.isEmpty ? 0 : BATTERY_COUNT_ROOM)
            .contentShape(Rectangle())
            .onTapGesture(perform: onClick)
            .accessibilityLabel(count.isEmpty ? "Battery \(percent)%" : "Lowest of \(count) batteries \(percent)%")
            .accessibilityAddTraits(.isButton)
        } else {
            InlineCell(text: text, colour: colour, icon: .battery5Bar, onClick: onClick)
        }
    }
}

private let BATTERY_COUNT_SIZE: CGFloat = 9
private let BATTERY_COUNT_ROOM: CGFloat = 10
private let BATTERY_GAP: CGFloat = 6

let COMPACT_STATUS_CELLS = 4

func compactStatusText(_ text: String) -> String {
    let lead = text.components(separatedBy: " \u{00b7} ").first ?? text
    return lead.components(separatedBy: " ").last { token in token.contains(where: \.isNumber) } ?? ""
}

struct OverrideCell: Equatable {
    let text: String
}

func overrideCell(_ state: FlyState?) -> OverrideCell? {
    state?.rcOverride == true ? OverrideCell(text: "RC override") : nil
}

func telemetryCell(_ state: FlyState?) -> String? {
    state?.telemetry.map { "\($0.localRssiDbm) dBm" }
}

func telemetryDetail(_ link: TelemetryLink?) -> [DetailRow] {
    guard let link else { return [] }
    return [
        DetailRow(label: "Local RSSI:", value: "\(link.localRssiDbm) dBm"),
        link.remoteRssiDbm.map { DetailRow(label: "Remote RSSI:", value: "\($0) dBm") },
        link.receiveErrors.map { DetailRow(label: "RX Errors:", value: "\($0)") },
        link.errorsFixed.map { DetailRow(label: "Errors Fixed:", value: "\($0)") },
        link.txBuffer.map { DetailRow(label: "TX Buffer:", value: "\($0)") },
        link.localNoise.map { DetailRow(label: "Local Noise:", value: "\($0)") },
        link.remoteNoise.map { DetailRow(label: "Remote Noise:", value: "\($0)") },
    ].compactMap { $0 }
}

private struct BatteryReturnButton: View {
    @Environment(\.theme) private var theme
    @QgcPath(GUIDED_ACTIONS) private var actionsJson
    @State private var confirming = false
    let onClosed: () -> Void

    var body: some View {
        if let rtl = guidedOffers(actionsJson)["rtl"], rtl.shown {
            Button { confirming = true } label: {
                Text("Return").foregroundStyle(theme.colors.error)
            }
            .buttonStyle(.borderless)
            .disabled(!rtl.ready)
            .padding(.horizontal, Space.s3)
            .alert("Return", isPresented: $confirming) {
                Button("Return") {
                    confirming = false
                    onClosed()
                    offMain { VehicleCommands.returnToLaunch(false) }
                }
                Button("Cancel", role: .cancel) { confirming = false }
            } message: {
                Text("Return to the launch position of the vehicle")
            }
        }
    }
}
