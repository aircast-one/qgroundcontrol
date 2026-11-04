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
        holdText: state.armed ? "Slide to disarm" : "Slide to arm",
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
    @QgcPath(WARNINGS) private var warnings
    @State private var showAll = false
    @State private var expanded: Set<Int> = []
    @State private var editing: String?
    let onDismiss: () -> Void

    var body: some View {
        let checks = armingChecks(warnings) ?? []
        NativeDialogSheet(title: "Vehicle status", onDismissRequest: onDismiss) {
            NativeForm {
                StatusSummary()
                ArmSection(onDismiss: onDismiss)
                sensors
                StatusMessages()
                overallStatus(checks)
                Section {
                    Button { open(SETUP_OVERVIEW_PAGE) } label: { NativeRowLabel(title: AIRCRAFT_SETUP) }
                        .listRowButton()
                }
                ParameterForm(page: STATUS_SETTINGS_PAGE)
                if advanced {
                    Section {
                        ForEach([("Vehicle parameters", SETUP_PARAMETERS_PAGE)], id: \.0) { label, page in
                            Button { open(page) } label: { NativeRowLabel(title: label) }
                                .listRowButton()
                        }
                    }
                }
            }
            .onAppear { offMain { VehicleCommands.resetAllMessages() } }
            .onChange(of: checks) { expanded = [] }
            .dialogButtons(primary: DialogButton(title: "Done", action: onDismiss))
            .background {
                if let name = editing {
                    ParameterEditDialog(name: name, title: EDIT_PARAMETER_TITLE, onDismiss: { editing = nil })
                }
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
            let normal = reading.sensors.filter { $0.state == SENSOR_HEALTHY_STATE }.count
            Section {
                ForEach(Array(shownSensors(reading.sensors, showAll).enumerated()), id: \.offset) { _, sensor in
                    let fault = sensor.state == SENSOR_FAULT_STATE
                    Button { open(SETUP_OVERVIEW_PAGE) } label: {
                        NativeRowLabel(title: sensor.name, status: sensor.label, statusColor: fault ? theme.colors.error : nil, dimmed: !fault)
                    }
                    .listRowButton()
                }
                if normal > 0 {
                    Button(showAll ? "Show less" : "Show \(normal) more") { showAll.toggle() }
                }
            } header: {
                Text("Sensors")
            }
        }
    }

    @ViewBuilder
    private func overallStatus(_ checks: [ArmingCheck]) -> some View {
        if !checks.isEmpty {
            Section {
                ForEach(Array(checks.enumerated()), id: \.offset) { index, check in
                    VStack(alignment: .leading, spacing: 2) {
                        HStack {
                            LinkedText(html: check.message, style: .bodyMedium, color: checkColour(check.severity), onParameter: { editing = $0 })
                                .frame(maxWidth: .infinity, alignment: .leading)
                            if !check.description.isBlank {
                                Image(systemName: expanded.contains(index) ? "chevron.up" : "chevron.down")
                                    .font(.footnote.weight(.semibold))
                                    .foregroundStyle(Color.secondary)
                                    .accessibilityLabel(expanded.contains(index) ? "Hide details" : "Show details")
                            }
                        }
                        if expanded.contains(index) {
                            LinkedText(html: check.description, style: .bodySmall, color: theme.colors.onSurfaceVariant, onParameter: { editing = $0 })
                        }
                    }
                    .contentShape(Rectangle())
                    .onTapGesture { expanded = expandedAfterTap(expanded, index, check) }
                    .accessibilityAddTraits(check.description.isBlank ? [] : .isButton)
                }
            } header: {
                Text("Overall status")
            }
        }
    }

    private func checkColour(_ severity: String) -> Color {
        switch severity {
        case "error": theme.colors.error
        case "warning": theme.aircast.warning
        default: theme.colors.onSurface
        }
    }
}

private struct StatusMessages: View {
    @QgcPath(MESSAGES) private var messagesJson
    @State private var shown = false

    var body: some View {
        let lines = Array(vehicleMessages(messagesJson).reversed())
        if !lines.isEmpty {
            Section {
                Button(messagesToggleText(shown)) { shown.toggle() }
                if shown {
                    ForEach(Array(lines.enumerated()), id: \.offset) { _, message in
                        MessageLine(level: message.level, time: message.time) {
                            Text(message.text).font(.bodyMedium)
                        }
                    }
                    Button("Clear messages") { offMain { VehicleCommands.clearMessages() } }
                }
            } header: {
                Text("Messages")
            }
        }
    }
}

func expandedAfterTap(_ expanded: Set<Int>, _ index: Int, _ check: ArmingCheck) -> Set<Int> {
    if check.description.isBlank { return expanded }
    return expanded.contains(index) ? expanded.subtracting([index]) : expanded.union([index])
}

struct StatusSummaryLines: Equatable {
    let title: String
    let detail: String
    let problem: PayloadProblem?
}

func statusSummary(_ state: FlyState?) -> StatusSummaryLines? {
    guard let state, !state.summaryDetail.isBlank else { return nil }
    return StatusSummaryLines(title: state.stateText, detail: state.summaryDetail, problem: state.summaryProblem)
}

private struct StatusSummary: View {
    @Environment(\.theme) private var theme
    @QgcPath(FLY_STATE) private var stateJson
    @State private var refusal: String?

    var body: some View {
        if let summary = statusSummary(flyState(stateJson)) {
            Section {
                VStack(alignment: .leading, spacing: 2) {
                    Text(summary.title).font(.headline)
                    HStack(spacing: Space.s2) {
                        Text(summary.detail).font(.subheadline).foregroundStyle(Color.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        if let problem = summary.problem {
                            PayloadFixButton(problem: problem) { refusal = $0 }
                        }
                    }
                    if let refusal {
                        Text(refusal).font(.footnote).foregroundStyle(theme.colors.error)
                    }
                }
            }
        }
    }
}

private struct ArmSection: View {
    @Environment(FlyScreenState.self) private var flyScreen
    @QgcPath(FLY_STATE) private var stateJson
    @State private var forceOpen = false
    let onDismiss: () -> Void

    var body: some View {
        if let state = flyState(stateJson), state.connected {
            let controls = armControls(state, forceOpen)
            Section {
                VStack(alignment: .leading, spacing: Space.s1) {
                    HoldToConfirm(label: controls.holdText, destructive: state.armed, enabled: controls.holdEnabled) { request(ARM_REQUEST) }
                    if controls.mayBeRefused {
                        Text("Arming may be refused.").font(.footnote).foregroundStyle(Color.secondary)
                    }
                    if controls.forceLink {
                        Button("Force arm…") { forceOpen = true }
                    }
                    if controls.forceHold {
                        HoldToConfirm(label: "Slide to force arm", destructive: true) { request(FORCE_ARM_REQUEST) }
                    }
                }
            }
        }
    }

    private func request(_ action: String) {
        flyScreen.deckRequest = action
        onDismiss()
    }
}
