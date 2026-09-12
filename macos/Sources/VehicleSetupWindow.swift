import AppKit
import SwiftUI

struct ParametersView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        VStack(spacing: 0) {
            filters
            Divider()
            if store.loading {
                EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters"))
                    .frame(maxHeight: .infinity)
            } else if !store.status.isEmpty {
                EmptyStateRow(text: store.status).frame(maxHeight: .infinity)
            } else if store.visible.isEmpty {
                EmptyStateRow(text: VehicleSetupText.filtered(connected: store.connected)).frame(maxHeight: .infinity)
            } else {
                list
            }
        }
        .onAppear(perform: store.load)
        .writeFailureAlert(title: "That parameter was not accepted", $store.writeFailure)
    }

    private var filters: some View {
        HStack(spacing: Overlay.step) {
            Picker("", selection: $store.group) {
                Text("All groups").tag("")
                ForEach(store.groups, id: \.self) { Text($0).tag($0) }
            }
            .labelsHidden()
            .frame(width: 150)

            SearchField(text: $store.search, placeholder: "Search parameters")

            Text("\(store.visible.count) of \(store.parameters.count)")
                .font(.caption).foregroundColor(.secondary)
                .frame(width: 110, alignment: .trailing)
        }
        .padding(Overlay.unit * 0.75)
    }

    private var list: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(Array(store.visible.enumerated()), id: \.element.id) { index, parameter in
                    ParameterRow(parameter: parameter, showSeparator: index > 0) {
                        store.write(parameter, $0)
                    }
                }
            }
            .background(Overlay.card)
            .clipShape(RoundedRectangle(cornerRadius: Overlay.cardRadius))
            .padding(Overlay.unit)
        }
    }
}

struct SensorsView: View {
    @ObservedObject var store: SensorsStore

    var body: some View {
        SetupPageBody(title: "Sensors",
                      note: "What the vehicle reports about its own hardware, live.") {
            if !store.status.isEmpty {
                GroupCard { EmptyStateRow(text: store.status) }
            } else {
                GroupCard {
                    GroupRow(title: store.failing.isEmpty
                                ? "All enabled sensors are healthy"
                                : "\(store.failing.count) sensor\(store.failing.count == 1 ? "" : "s") reporting a fault",
                             description: store.failing.isEmpty
                                ? ""
                                : store.failing.joined(separator: ", "),
                             showSeparator: false,
                             leading: {
                                 Image(systemName: store.failing.isEmpty
                                     ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                                     .foregroundColor(store.failing.isEmpty ? .green : .orange)
                             })
                }

                if store.calibration.connected {
                    calibration
                }

                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: "Reported sensors")
                    GroupCard {
                        ForEach(Array(store.sensors.enumerated()), id: \.element.id) { index, sensor in
                            GroupRow(title: sensor.name,
                                     showSeparator: index > 0,
                                     trailing: {
                                         Text(sensor.label)
                                             .font(.callout)
                                             .foregroundColor(SensorsView.colour(sensor.state))
                                     })
                        }
                    }
                }
            }
        }
        .onAppear(perform: store.start)
        .onDisappear(perform: store.stop)
    }

    @ViewBuilder private var calibration: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
            SectionLabel(text: "Calibration")

            if !store.calibration.needsAttention.isEmpty {
                Label(store.calibration.needsAttention, systemImage: "exclamationmark.triangle.fill")
                    .font(.callout)
                    .foregroundColor(.orange)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.bottom, Overlay.unit * 0.3)
            }

            GroupCard {
                ForEach(Array(store.calibration.routines.enumerated()), id: \.element.id) { row, routine in
                    GroupRow(title: routine.title,
                             description: routine.warning.isEmpty
                                 ? routine.description
                                 : "\(routine.description) \(routine.warning)",
                             showSeparator: row > 0,
                             descriptionLines: routine.descriptionLines,
                             trailing: {
                                 Button("Start") { store.start(routine) }
                                     .disabled(!routine.enabled)
                             })
                        .foregroundColor(routine.blocked ? .secondary : .primary)
                }
            }

            if store.calibration.busy {
                VStack(alignment: .leading, spacing: Overlay.unit * 0.4) {
                    if !store.calibration.helpText.isEmpty {
                        Text(store.calibration.helpText)
                            .font(.callout)
                            .fixedSize(horizontal: false, vertical: true)
                    }

                    HStack(spacing: Overlay.step) {
                        ProgressView(value: store.calibration.progress, total: 100)
                            .frame(maxWidth: 260)
                        Text(store.calibration.progressText)
                            .font(.callout.monospacedDigit())
                            .foregroundColor(.secondary)
                        Spacer()
                        Button("Next", action: store.next)
                            .disabled(!store.calibration.nextEnabled)
                        Button("Cancel", action: store.cancelCalibration)
                            .disabled(!store.calibration.cancelEnabled)
                    }

                    if store.calibration.showsSides {
                        HStack(spacing: Overlay.unit) {
                            ForEach(store.calibration.visibleSides) { side in
                                VStack(spacing: 3) {
                                    Image(systemName: side.symbol)
                                        .font(.system(size: 17))
                                        .foregroundColor(side.stage == .done ? .green
                                            : (side.stage == .inProgress ? .accentColor : .secondary))
                                    Text(side.title)
                                        .font(.caption2)
                                        .foregroundColor(.secondary)
                                }
                            }
                            Spacer()
                        }
                    }
                }
                .padding(.top, Overlay.unit * 0.4)
            }

            if !store.calibration.statusText.isEmpty {
                Text(store.calibration.statusText)
                    .font(.caption.monospaced())
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.top, Overlay.unit * 0.3)
            }
        }
    }

    static func colour(_ state: SensorHealth.State) -> Color {
        switch state {
        case .healthy: return .green
        case .unhealthy: return .orange
        case .disabled, .unknown: return .secondary
        }
    }
}

struct SafetyView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        SetupPageBody(title: "Safety",
                      note: "What the vehicle does when something goes wrong.",
                      connected: store.connected) {
            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                        "reports none of the safety parameters this page knows about."))
                }
            } else {
                SetupSections(sections: sections, store: store)
            }
        }
        .onAppear(perform: store.load)
    }

    private var sections: [SettingsSection] { store.sections(of: "Safety") }
}

struct MotorsView: View {
    @ObservedObject var motors: MotorsStore

    var body: some View {
        SetupPageBody(title: "Motors",
                      note: "Spin one motor at a time to check it turns the right way.",
                      connected: motors.state.connected) {
            VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
                SectionLabel(text: "Safety")
                GroupCard {
                    GroupRow(title: "Motors enabled", showSeparator: false, trailing: {
                        Toggle("", isOn: Binding(get: { motors.safetyOff },
                                                 set: { motors.setSafety($0) }))
                            .labelsHidden()
                            .toggleStyle(.switch)
                            .controlSize(.small)
                            .disabled(!motors.state.connected || motors.state.armed)
                    })
                }
                Text(MotorTest.safetyText(motors.safetyOff))
                    .font(.caption)
                    .foregroundColor(motors.safetyOff ? .orange : .secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Overlay.horizontalPadding)
                if !motors.state.armedRefusal.isEmpty {
                    Text(motors.state.armedRefusal)
                        .font(.caption)
                        .foregroundColor(.orange)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.horizontal, Overlay.horizontalPadding)
                }
            }

            VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
                SectionLabel(text: "Throttle")
                GroupCard {
                    GroupRow(title: String(format: "%.0f%%", motors.throttle), showSeparator: false,
                             trailing: {
                        Slider(value: $motors.throttle,
                               in: MotorTest.minimumThrottle...MotorTest.maximumThrottle)
                            .frame(width: 220)
                            .disabled(!motors.state.canTest(safetyOff: motors.safetyOff))
                    })
                }
                if !motors.state.countWarning.isEmpty {
                    Text(motors.state.countWarning)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.horizontal, Overlay.horizontalPadding)
                }
            }

            VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
                SectionLabel(text: "Motors")
                GroupCard {
                    GroupRow(title: "Spin", showSeparator: false, trailing: {
                        HStack(spacing: 6) {
                            ForEach(Array(motors.state.names.enumerated()), id: \.offset) { index, name in
                                Button(name) { motors.test(motor: index) }
                                    .controlSize(.small)
                            }
                            Button("All") { motors.testAll() }.controlSize(.small)
                            Button("Stop") { motors.stopAll() }.controlSize(.small)
                        }
                        .disabled(!motors.state.canTest(safetyOff: motors.safetyOff))
                    })
                }
            }
        }
    }
}

struct RemoteSupportView: View {
    @ObservedObject var support: RemoteSupportStore

    var body: some View {
        SetupPageBody(title: "Remote Support",
                      note: support.state.note,
                      connected: true) {
            VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
                SectionLabel(text: "Support host")
                GroupCard {
                    GroupRow(title: "Host", showSeparator: false, trailing: {
                        ValueField(value: support.state.host, units: "", width: 260) {
                            support.setHost($0)
                        }
                    })
                    GroupRow(title: support.state.status, trailing: {
                        Button(support.state.actionTitle) {
                            support.state.forwarding ? support.stop() : support.connect()
                        }
                        .controlSize(.small)
                        .disabled(!support.state.canConnect && !support.state.canStop)
                    })
                }
            }
        }
        .onAppear(perform: support.startWatching)
        .onDisappear(perform: support.stopWatching)
        .writeFailureAlert($support.writeFailure)
    }
}

struct PowerView: View {
    @ObservedObject var store: ParametersStore
    @ObservedObject var power: PowerStore

    var body: some View {
        SetupPageBody(title: "Power",
                      note: "What the vehicle measures its pack with, and how that measurement is scaled.",
                      connected: store.connected) {
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: "Measured now")
                GroupCard {
                    if power.battery.available {
                        GroupRow(title: "Voltage", value: power.battery.voltageText, showSeparator: false)
                        GroupRow(title: "Current", value: power.battery.currentText)
                        GroupRow(title: "Remaining", value: power.battery.percentText)
                    } else {
                        EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                            "is not reporting a battery."))
                    }
                }
            }

            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                        "reports none of the battery parameters this page knows about."))
                }
            } else {
                SetupSections(sections: sections, store: store)
            }
        }
        .onAppear {
            store.load()
            power.start()
        }
        .onDisappear(perform: power.stop)
    }

    private var sections: [SettingsSection] { store.sections(of: "Power") }
}

struct LightsView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        SetupPageBody(title: "Lights",
                      note: "The channels the vehicle drives its lights from.",
                      connected: store.connected) {
            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                        "reports no light channels."))
                }
            } else {
                SetupSections(sections: sections, store: store)
            }
        }
        .onAppear(perform: store.load)
    }

    private var sections: [SettingsSection] { store.sections(of: "Lights") }
}

struct CameraView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        SetupPageBody(title: "Camera",
                      note: "The gimbal the vehicle carries and how it triggers a camera.",
                      connected: store.connected) {
            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                        "reports no gimbal or camera parameters."))
                }
            } else {
                SetupSections(sections: sections, store: store)
            }
        }
        .onAppear(perform: store.load)
    }

    private var sections: [SettingsSection] { store.sections(of: "Camera") }
}

struct TuningView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        SetupPageBody(title: "Tuning",
                      note: "The gains that decide how the vehicle answers the sticks. Change one thing at a time and fly it.",
                      connected: store.connected) {
            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                        "reports none of the tuning parameters this page knows about."))
                }
            } else {
                SetupSections(sections: sections, store: store)

                Text("AutoTune and in-flight tuning are flown, not configured, and stay in the Qt view.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Overlay.horizontalPadding)
            }
        }
        .onAppear(perform: store.load)
    }

    private var sections: [SettingsSection] { store.sections(of: "Tuning") }
}

struct RadioView: View {
    @ObservedObject var store: RadioStore

    var body: some View {
        SetupPageBody(title: "Radio",
                      note: "What the transmitter is sending, and the calibration that teaches the vehicle its limits.") {
            GroupCard {
                GroupRow(title: store.state.summary, showSeparator: false,
                         leading: {
                             Image(systemName: store.state.channelCount > 0
                                 ? "antenna.radiowaves.left.and.right" : "antenna.radiowaves.left.and.right.slash")
                                 .foregroundColor(store.state.channelCount > 0 ? .green : .secondary)
                         })
            }

            if !store.state.shortfall.isEmpty {
                Label(store.state.shortfall, systemImage: "exclamationmark.triangle.fill")
                    .font(.callout)
                    .foregroundColor(.orange)
                    .fixedSize(horizontal: false, vertical: true)
            }

            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: "Calibration")
                GroupCard {
                    GroupRow(title: "Transmitter mode",
                             description: "Which stick carries throttle",
                             showSeparator: false,
                             trailing: {
                                 Picker("", selection: Binding(
                                     get: { store.state.transmitterMode },
                                     set: { store.setTransmitterMode($0) })
                                 ) {
                                     Text("Mode 1").tag(1)
                                     Text("Mode 2").tag(2)
                                 }
                                 .labelsHidden()
                                 .frame(width: 120)
                                 .disabled(store.state.calibrating)
                             })
                    GroupRow(title: "Stick calibration",
                             description: store.state.statusText.isEmpty
                                 ? "Move every stick and switch through its full travel when asked."
                                 : store.state.statusText,
                             titleLines: 1,
                             trailing: {
                                 HStack(spacing: Overlay.step) {
                                     if store.state.skipEnabled {
                                         Button("Skip", action: store.skip)
                                     }
                                     if store.state.cancelEnabled {
                                         Button("Cancel", action: store.cancel)
                                     }
                                     Button(store.state.nextText.isEmpty ? "Start" : store.state.nextText,
                                            action: store.next)
                                         .disabled(!store.state.nextEnabled)
                                 }
                             })
                }
            }

            if !store.state.sticks.isEmpty {
                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: "Sticks")
                    GroupCard {
                        ForEach(Array(store.state.sticks.enumerated()), id: \.element.id) { row, stick in
                            GroupRow(title: stick.title,
                                     description: stick.reversed ? "Reversed" : "",
                                     showSeparator: row > 0,
                                     trailing: {
                                         HStack(spacing: Overlay.step) {
                                             RadioBar(fraction: stick.fraction, live: stick.mapped)
                                             Text(stick.valueText)
                                                 .font(.caption.monospacedDigit())
                                                 .foregroundColor(Overlay.value)
                                                 .frame(width: 74, alignment: .trailing)
                                         }
                                     })
                        }
                    }
                }
            }

            if !store.state.liveChannels.isEmpty {
                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: "Channels")
                    GroupCard {
                        ForEach(Array(store.state.liveChannels.enumerated()), id: \.element.id) { row, channel in
                            GroupRow(title: "Channel \(channel.label)",
                                     showSeparator: row > 0,
                                     trailing: {
                                         HStack(spacing: Overlay.step) {
                                             RadioBar(fraction: channel.fraction, live: true)
                                             Text(channel.valueText)
                                                 .font(.caption.monospacedDigit())
                                                 .foregroundColor(Overlay.value)
                                                 .frame(width: 74, alignment: .trailing)
                                         }
                                     })
                        }
                    }
                }
            }
        }
        .onAppear(perform: store.start)
        .onDisappear(perform: store.stop)
        .writeFailureAlert($store.writeFailure)
    }
}

struct RadioBar: View {
    let fraction: Double
    let live: Bool

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .leading) {
                Capsule().fill(Color.primary.opacity(0.12))
                Capsule()
                    .fill(live ? Color.accentColor : Color.secondary.opacity(0.5))
                    .frame(width: max(3, geometry.size.width * fraction))
            }
        }
        .frame(width: 150, height: 7)
    }
}

struct FrameView: View {
    @ObservedObject var store: ParametersStore
    @ObservedObject var frame: FrameStore

    var body: some View {
        SetupPageBody(title: "Frame",
                      note: "Which airframe this is, and what the firmware made of it.",
                      connected: store.connected) {
            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                        "does not report a frame class."))
                }
            } else {
                if needsFrameClass {
                    Label("No airframe is selected. The vehicle will not arm until one is.",
                          systemImage: "exclamationmark.triangle.fill")
                        .font(.callout)
                        .foregroundColor(.orange)
                        .fixedSize(horizontal: false, vertical: true)
                }

                SetupSections(sections: sections, store: store)
            }

            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: "What the vehicle reports")
                GroupCard {
                    if frame.setup.known {
                        GroupRow(title: "Vehicle type", value: frame.setup.vehicleTypeText,
                                 showSeparator: false)
                        GroupRow(title: "Motors", value: frame.setup.motorText)
                    } else {
                        EmptyStateRow(text: "No vehicle is connected.")
                    }
                }
            }
        }
        .onAppear {
            store.load()
            frame.start()
        }
        .onDisappear(perform: frame.stop)
    }

    private var sections: [SettingsSection] { store.sections(of: "Frame") }

    private var needsFrameClass: Bool {
        FrameSetup.needsFrameClass(store.parameter(named: "FRAME_CLASS")?.selectedOption?.raw)
    }
}

struct FlightModesView: View {
    @ObservedObject var store: ParametersStore
    @ObservedObject var modes: ModeSlotsStore

    var body: some View {
        SetupPageBody(title: "Flight Modes",
                      note: "Which mode each position of the transmitter switch selects.",
                      connected: store.connected) {
            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if positions.isEmpty {
                GroupCard { EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                    "does not report a six-position mode switch.")) }
            } else {
                if let naming, let channel = store.parameter(named: naming.channelParameter) {
                    VStack(alignment: .leading, spacing: 0) {
                        SectionLabel(text: "Mode switch")
                        GroupCard {
                            ParameterRow(parameter: channel, showSeparator: false) {
                                store.write(channel, $0)
                            }
                        }
                    }
                }

                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: "Switch positions")
                    if !modes.slots.reason.isEmpty {
                        GroupCard { EmptyStateRow(text: modes.slots.reason) }
                    }
                    GroupCard {
                        ForEach(positions, id: \.index) { position in
                            if let parameter = store.parameter(named: position.parameter) {
                                let active = modes.slots.isLive(position.index)
                                GroupRow(title: "Position \(position.index)",
                                         showSeparator: position.index > 1,
                                         leading: {
                                             Seal(label: "\(position.index)",
                                                  colour: active ? Overlay.launch : Overlay.mission)
                                         },
                                         trailing: {
                                             HStack(spacing: Overlay.step) {
                                                 if active {
                                                     Text("Active")
                                                         .font(.caption.weight(.semibold))
                                                         .foregroundColor(.green)
                                                 }
                                                 ParameterEditor(
                                                     value: parameter.value,
                                                     units: parameter.units,
                                                     options: parameter.options,
                                                     selectedRaw: parameter.selectedOption?.raw ?? "") {
                                                     store.write(parameter, $0)
                                                 }
                                             }
                                         })
                            }
                        }
                    }
                }
            }
        }
        .onAppear {
            store.load()
            modes.startWatching()
        }
        .onDisappear(perform: modes.stopWatching)
    }

    private var naming: FlightModeNaming? {
        FlightModeNaming.chosen(from: Set(store.parameters.map(\.name)))
    }

    private var positions: [FlightModePosition] {
        FlightModePosition.present(in: Set(store.parameters.map(\.name)))
    }
}

struct SetupSummaryView: View {
    @ObservedObject var store: VehicleComponentsStore
    @ObservedObject var sensors: SensorsStore
    @ObservedObject var selection: PageSelection

    private var readiness: VehicleReadiness { store.readiness }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Overlay.unit * 1.25) {
                hero

                if !store.outstanding.isEmpty {
                    VStack(alignment: .leading, spacing: 0) {
                        SectionLabel(text: "Needs setup")
                        GroupCard {
                            ForEach(store.outstanding) { component in
                                GroupRow(title: component.name,
                                         description: "Not configured on this vehicle",
                                         showSeparator: component.id != store.outstanding.first?.id,
                                         leading: { Tile(symbol: "exclamationmark", colour: .red) })
                            }
                        }
                    }
                }

                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: "Components")
                    GroupCard {
                        if store.components.isEmpty {
                            EmptyStateRow(text: store.connected
                                ? "This vehicle reports no setup components."
                                : "Connect a vehicle to see what it needs.")
                        } else {
                            ForEach(store.components) { component in
                                let page = VehicleComponentInfo.page(for: component,
                                                                     among: store.pageNames)
                                let opens = page != nil
                                let glyph = page ?? component.name
                                let faulted = component.isSensors && !sensors.failing.isEmpty
                                let good = !component.needsAttention && !faulted
                                GroupRow(title: component.name,
                                         value: component.needsAttention ? "Needs setup"
                                             : faulted ? "Reporting a fault" : "",
                                         showSeparator: component.id != store.components.first?.id,
                                         leading: {
                                             Tile(symbol: SetupPage.symbol(for: glyph),
                                                  colour: SetupPage.colour(for: glyph))
                                         },
                                         trailing: {
                                             HStack(spacing: 6) {
                                                 Image(systemName: good
                                                     ? "checkmark.circle.fill" : "exclamationmark.circle.fill")
                                                     .foregroundColor(good ? .green : .orange)
                                                 if opens {
                                                     Text("\u{203A}")
                                                         .font(.title3)
                                                         .foregroundColor(Overlay.chevron)
                                                 }
                                             }
                                         })
                                    .contentShape(Rectangle())
                                    .onTapGesture { if let page { selection.page = page } }
                            }
                        }
                    }
                }
            }
            .padding(Overlay.unit * 1.25)
            .frame(maxWidth: 640, alignment: .leading)
        }
        .onAppear {
            store.reload()
            sensors.start()
        }
    }

    private var hero: some View {
        GroupCard {
            HStack(spacing: Overlay.unit) {
                Tile(symbol: "airplane", colour: .accentColor)
                    .scaleEffect(1.6)
                    .frame(width: 44, height: 44)
                VStack(alignment: .leading, spacing: 4) {
                    Text(readiness.headline).font(.title3.weight(.semibold))
                    Text(readiness.detail).font(.callout).foregroundColor(.secondary)
                }
                Spacer(minLength: 0)
                if let verdict = readiness.verdict {
                    StatusPill(text: verdict.text, good: verdict.good)
                }
            }
            .padding(Overlay.unit)
        }
    }
}

extension SetupPage {
    static func colour(for page: String) -> Color {
        switch page {
        case "Summary": return .accentColor
        case "Sensors": return .teal
        case "Flight Modes": return .indigo
        case "Safety": return .orange
        case "Parameters": return .gray
        case "Radio": return .purple
        case "Power": return .green
        default: return .blue
        }
    }
}

struct ParameterSectionsView: View {
    @ObservedObject var store: ParametersStore
    let page: String

    var body: some View {
        SetupPageBody(title: page,
                      note: "Settings the vehicle groups under \(page.lowercased()).",
                      connected: store.connected) {
            if store.loading {
                GroupCard { EmptyStateRow(text: VehicleSetupText.waiting(connected: store.connected, for: "parameters")) }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: VehicleSetupText.absent(connected: store.connected,
                        "reports none of the parameters this page knows about."))
                }
            } else {
                SetupSections(sections: sections, store: store)
            }
        }
        .onAppear(perform: store.load)
    }

    private var sections: [SettingsSection] { store.sections(of: page) }
}

struct VehicleSetupView: View {
    @ObservedObject var parameters: ParametersStore
    @ObservedObject var sensors: SensorsStore
    @ObservedObject var components: VehicleComponentsStore
    @ObservedObject var power: PowerStore
    @ObservedObject var frame: FrameStore
    @ObservedObject var radio: RadioStore
    @ObservedObject var motors: MotorsStore
    @ObservedObject var support: RemoteSupportStore
    @ObservedObject var modeSlots: ModeSlotsStore
    @ObservedObject var selection: PageSelection

    var body: some View {
        HStack(spacing: 0) {
            List(selection: Binding(
                get: { Optional(selection.page) },
                set: { selection.page = $0 ?? selection.page })
            ) {
                ForEach(components.groups) { group in
                    Section(group.title) {
                        ForEach(group.pages, id: \.name) { page in
                            row(page.name,
                                badge: page.name == "Sensors" && !sensors.failing.isEmpty)
                        }
                    }
                }
            }
            .listStyle(.sidebar)
            .frame(width: 210)

            Divider()

            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(minWidth: 860, minHeight: 560)
        .onAppear {
            components.startWatching()
            selection.offer(components.pageNames)
        }
        .onDisappear(perform: components.stopWatching)
        .onChange(of: components.groups) { _ in selection.offer(components.pageNames) }
    }

    private func row(_ name: String, badge: Bool = false) -> some View {
        SidebarRow(title: name,
                   symbol: SetupPage.symbol(for: name),
                   colour: SetupPage.colour(for: name),
                   badge: badge)
            .tag(name)
    }

    @ViewBuilder private var content: some View {
        let blocked = SetupCatalogue.page(selection.page, in: components.groups)?.blockedSentence
        VStack(spacing: 0) {
            if let blocked {
                Text(blocked)
                    .font(.callout.weight(.medium))
                    .foregroundColor(Overlay.blocked)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Overlay.horizontalPadding)
                    .padding(.vertical, Overlay.verticalPadding)
            }
            pages.disabled(blocked != nil)
        }
    }

    @ViewBuilder private var pages: some View {
        switch selection.page {
        case "Parameters": ParametersView(store: parameters)
        case "Safety": SafetyView(store: parameters)
        case "Power": PowerView(store: parameters, power: power)
        case "Frame": FrameView(store: parameters, frame: frame)
        case "Radio": RadioView(store: radio)
        case "Tuning": TuningView(store: parameters)
        case "Camera": CameraView(store: parameters)
        case "Lights": LightsView(store: parameters)
        case "Motors": MotorsView(motors: motors).onAppear(perform: motors.start)
            .onDisappear(perform: motors.stop)
        case "Remote Support": RemoteSupportView(support: support)
        case "Flight Modes": FlightModesView(store: parameters, modes: modeSlots)
        case "Sensors": SensorsView(store: sensors)
        case "Summary": SetupSummaryView(store: components, sensors: sensors, selection: selection)
        default:
            if SetupCatalogue.page(selection.page, in: components.groups)?.parameterSections == true {
                ParameterSectionsView(store: parameters, page: selection.page)
            } else {
                SetupSummaryView(store: components, sensors: sensors, selection: selection)
            }
        }
    }
}

final class VehicleSetupWindow: NSObject, NSWindowDelegate {
    static let shared = VehicleSetupWindow()

    private let parameters = ParametersStore()
    private let modeSlots = ModeSlotsStore()
    private let sensors = SensorsStore()
    private let components = VehicleComponentsStore()
    private let power = PowerStore()
    private let frame = FrameStore()
    private let radio = RadioStore()
    private let motors = MotorsStore()
    private let support = RemoteSupportStore()
    private let selection = PageSelection(owner: "vehicleSetup", pages: ["Summary"])
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(parameters)
        NativeProbe.register(modeSlots)
        NativeProbe.register(sensors)
        NativeProbe.register(components)
        NativeProbe.register(power)
        NativeProbe.register(frame)
        NativeProbe.register(radio)
        NativeProbe.register(motors)
        NativeProbe.register(support)
        NativeProbe.register(selection, as: selection.identifier)
    }

    @objc func showFromMenu() {
        show()
    }

    func show() {
        if let window {
            window.makeKeyAndOrderFront(nil)
            return
        }

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 820, height: 560),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Vehicle Setup"
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(rootView: VehicleSetupView(
            parameters: parameters, sensors: sensors, components: components,
            power: power, frame: frame, radio: radio, motors: motors, support: support,
            modeSlots: modeSlots, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        sensors.stop()
        motors.stop()
        components.stopWatching()
        support.stopWatching()
        modeSlots.stopWatching()
        power.stop()
        frame.stop()
        radio.stop()
        window = nil
    }
}
