import SwiftUI

private let SENSOR_FAULT_STATE = "unhealthy"

let ARM_REQUEST = "arm"
let FORCE_ARM_REQUEST = "forceArm"

let ARM_UNAVAILABLE = "Arming is not available right now."

func deckRequestRefusal(_ offer: GuidedOffer?) -> String {
    offer?.reason.nonBlank ?? ARM_UNAVAILABLE
}

struct ArmControls: Equatable {
    let holdText: String
    let holdEnabled: Bool
    let mayBeRefused: Bool
    let forceLink: Bool
    let forceHold: Bool
}

func armControls(_ state: FlyState, _ forceOpen: Bool) -> ArmControls {
    ArmControls(
        holdText: state.armed ? "Hold to disarm" : "Hold to arm",
        holdEnabled: state.canArm,
        mayBeRefused: !state.armed && !state.nominal && state.canArm && !forceOpen,
        forceLink: !state.armed && !forceOpen && (!state.canArm || state.fault),
        forceHold: !state.armed && forceOpen
    )
}

let SENSOR_HEALTHY_STATE = "healthy"

func messagesToggleText(_ shown: Bool) -> String { shown ? "Hide messages" : "Show messages" }

func shownSensors(_ sensors: [SensorHealth], _ showAll: Bool) -> [SensorHealth] {
    showAll ? sensors : sensors.filter { $0.state != SENSOR_HEALTHY_STATE }
}

struct VehicleStatusSheet: View {
    @Environment(\.theme) private var theme
    @Environment(AppNavigationState.self) private var navigation
    @AdvancedUiShown private var advanced
    @QgcPath(SENSOR_HEALTH) private var healthJson
    @State private var showAll = false
    let onDismiss: () -> Void

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s1) {
                    StatusSummary()
                    ArmSection(onDismiss: onDismiss)
                    sensors
                    StatusMessages()
                    OverallStatus()
                    Button(AIRCRAFT_SETUP) { open(SETUP_OVERVIEW_PAGE) }
                        .buttonStyle(.text)
                        .padding(.horizontal, Space.s3)
                    ParameterForm(page: STATUS_SETTINGS_PAGE)
                        .frame(maxHeight: 360)
                    if advanced {
                        ForEach([("Vehicle parameters", SETUP_PARAMETERS_PAGE)], id: \.0) { label, page in
                            HStack {
                                Text(label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                                Button("Configure") { open(page) }.buttonStyle(.bordered)
                            }
                            .padding(.horizontal, Space.s5)
                            .padding(.vertical, 2)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.bottom, Space.s4)
            }
        }
    }

    private func open(_ page: String) {
        navigation.setupPage = page
        onDismiss()
    }

    @ViewBuilder
    private var sensors: some View {
        if let reading = sensorHealth(healthJson), healthJson?["healthChecksSupported"].bool != true, reading.available, !reading.sensors.isEmpty {
            Text("Sensors").font(.titleSmall).padding(.horizontal, Space.s5).padding(.vertical, Space.s1)
            ForEach(Array(shownSensors(reading.sensors, showAll).enumerated()), id: \.offset) { _, sensor in
                let fault = sensor.state == SENSOR_FAULT_STATE
                Button { open(SETUP_OVERVIEW_PAGE) } label: {
                    HStack(spacing: Space.s2) {
                        Text(sensor.name)
                            .font(.bodyMedium)
                            .foregroundStyle(fault ? theme.colors.onSurface : theme.colors.onSurfaceVariant)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        Text(sensor.label).font(.bodySmall).foregroundStyle(fault ? theme.colors.error : theme.colors.onSurfaceVariant)
                        Image(.chevronRight).font(.system(size: 13)).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    .padding(.horizontal, Space.s5)
                    .padding(.vertical, 6)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
            let normal = reading.sensors.filter { $0.state == SENSOR_HEALTHY_STATE }.count
            if normal > 0 {
                Button(showAll ? "Show less" : "Show \(normal) more") { showAll.toggle() }
                    .buttonStyle(.text)
                    .padding(.horizontal, Space.s3)
            }
        }
    }
}

private struct StatusMessages: View {
    @QgcPath(MESSAGES) private var messagesJson
    @State private var shown = false

    var body: some View {
        let lines = Array(vehicleMessages(messagesJson).reversed())
        VStack(alignment: .leading, spacing: Space.s1) {
            if !lines.isEmpty {
                HStack {
                    Text("Messages").font(.titleSmall).frame(maxWidth: .infinity, alignment: .leading)
                    if shown {
                        Button("Clear") { offMain { VehicleCommands.clearMessages() } }.buttonStyle(.text)
                    }
                }
                .padding(.horizontal, Space.s5)
                Button(messagesToggleText(shown)) { shown.toggle() }
                    .buttonStyle(.text)
                    .padding(.horizontal, Space.s3)
                if shown {
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            ForEach(Array(lines.enumerated()), id: \.offset) { _, message in
                                MessageLine(level: message.level, time: message.time) {
                                    Text(message.text).font(.bodyMedium)
                                }
                            }
                        }
                    }
                    .frame(maxHeight: 320)
                    .padding(.horizontal, Space.s5)
                }
            }
        }
        .onAppear { offMain { VehicleCommands.resetAllMessages() } }
    }
}

func expandedAfterTap(_ expanded: Set<Int>, _ index: Int, _ check: ArmingCheck) -> Set<Int> {
    if check.description.isBlank { return expanded }
    return expanded.contains(index) ? expanded.subtracting([index]) : expanded.union([index])
}

private struct OverallStatus: View {
    @Environment(\.theme) private var theme
    @QgcPath(WARNINGS) private var warnings
    @State private var expanded: Set<Int> = []
    @State private var editing: String?

    var body: some View {
        let checks = armingChecks(warnings) ?? []
        VStack(alignment: .leading, spacing: Space.s1) {
            if !checks.isEmpty {
                Text("Overall status").font(.titleSmall).padding(.horizontal, Space.s5).padding(.vertical, Space.s1)
                ForEach(Array(checks.enumerated()), id: \.offset) { index, check in
                    VStack(alignment: .leading, spacing: 2) {
                        HStack {
                            LinkedText(html: check.message, style: .bodyMedium, color: checkColour(check.severity), onParameter: { editing = $0 })
                                .frame(maxWidth: .infinity, alignment: .leading)
                            if !check.description.isBlank {
                                Image(.arrowDropDown)
                                    .frame(width: 24, height: 24)
                                    .accessibilityLabel(expanded.contains(index) ? "Hide details" : "Show details")
                            }
                        }
                        if expanded.contains(index) {
                            LinkedText(html: check.description, style: .bodySmall, color: theme.colors.onSurfaceVariant, onParameter: { editing = $0 })
                        }
                    }
                    .padding(.horizontal, Space.s5)
                    .padding(.vertical, 6)
                    .contentShape(Rectangle())
                    .onTapGesture { expanded = expandedAfterTap(expanded, index, check) }
                    .accessibilityAddTraits(check.description.isBlank ? [] : .isButton)
                }
            }
            if let name = editing {
                ParameterEditDialog(name: name, title: EDIT_PARAMETER_TITLE, onDismiss: { editing = nil })
            }
        }
        .onChange(of: checks) { expanded = [] }
    }

    private func checkColour(_ severity: String) -> Color {
        switch severity {
        case "error": theme.colors.error
        case "warning": theme.aircast.warning
        default: theme.colors.onSurface
        }
    }
}

private struct StatusSummary: View {
    @Environment(\.theme) private var theme
    @QgcPath(FLY_STATE) private var stateJson

    var body: some View {
        if let state = flyState(stateJson), !state.summaryDetail.isBlank {
            VStack(alignment: .leading) {
                Text(state.stateText).font(.titleMedium)
                Text(state.summaryDetail).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .padding(.horizontal, Space.s5)
            .padding(.vertical, Space.s1)
        }
    }
}

private struct ArmSection: View {
    @Environment(\.theme) private var theme
    @Environment(FlyScreenState.self) private var flyScreen
    @QgcPath(FLY_STATE) private var stateJson
    @State private var forceOpen = false
    let onDismiss: () -> Void

    var body: some View {
        if let state = flyState(stateJson), state.connected {
            let controls = armControls(state, forceOpen)
            VStack(alignment: .leading, spacing: Space.s1) {
                HoldToConfirm(label: controls.holdText, destructive: state.armed, enabled: controls.holdEnabled) { request(ARM_REQUEST) }
                if controls.mayBeRefused {
                    Text("Arming may be refused.").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                if controls.forceLink {
                    Button("Force arm…") { forceOpen = true }.buttonStyle(.text)
                }
                if controls.forceHold {
                    HoldToConfirm(label: "Hold to force arm", destructive: true) { request(FORCE_ARM_REQUEST) }
                }
            }
            .padding(.horizontal, Space.s5)
            .padding(.vertical, Space.s1)
        }
    }

    private func request(_ action: String) {
        flyScreen.deckRequest = action
        onDismiss()
    }
}
