import AppKit
import SwiftUI

struct ParametersView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        VStack(spacing: 0) {
            filters
            Divider()
            if store.loading {
                EmptyStateRow(text: "Reading parameters from the vehicle\u{2026}")
                    .frame(maxHeight: .infinity)
            } else if !store.status.isEmpty {
                EmptyStateRow(text: store.status).frame(maxHeight: .infinity)
            } else if store.visible.isEmpty {
                EmptyStateRow(text: "No parameter matches this filter.").frame(maxHeight: .infinity)
            } else {
                list
            }
        }
        .onAppear(perform: store.load)
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
                                : store.failing.map(\.name).joined(separator: ", "),
                             showSeparator: false,
                             leading: {
                                 Image(systemName: store.failing.isEmpty
                                     ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                                     .foregroundColor(store.failing.isEmpty ? .green : .orange)
                             })
                }

                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: "Reported sensors")
                    GroupCard {
                        ForEach(Array(store.sensors.enumerated()), id: \.element.id) { index, sensor in
                            GroupRow(title: sensor.name,
                                     showSeparator: index > 0,
                                     trailing: {
                                         Text(SensorsView.label(sensor.state))
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

    static func label(_ state: SensorHealth.State) -> String {
        switch state {
        case .healthy: return "Healthy"
        case .unhealthy: return "Fault"
        case .disabled: return "Not enabled"
        }
    }

    static func colour(_ state: SensorHealth.State) -> Color {
        switch state {
        case .healthy: return .green
        case .unhealthy: return .orange
        case .disabled: return .secondary
        }
    }
}

struct SafetyView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        SetupPageBody(title: "Safety",
                      note: "What the vehicle does when something goes wrong.") {
            if store.loading {
                GroupCard { EmptyStateRow(text: "Reading parameters from the vehicle\u{2026}") }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: "This vehicle reports none of the safety parameters this page knows about.")
                }
            } else {
                ForEach(sections, id: \.section.id) { entry in
                    VStack(alignment: .leading, spacing: 0) {
                        SectionLabel(text: entry.section.title)
                        GroupCard {
                            ForEach(Array(entry.names.enumerated()), id: \.element) { index, name in
                                if let parameter = store.parameter(named: name) {
                                    ParameterRow(parameter: parameter, showSeparator: index > 0) {
                                        store.write(parameter, $0)
                                    }
                                }
                            }
                        }
                        Text(entry.section.note)
                            .font(.caption).foregroundColor(.secondary)
                            .padding(.horizontal, Overlay.horizontalPadding)
                            .padding(.top, Overlay.unit * 0.35)
                    }
                }
            }
        }
        .onAppear(perform: store.load)
    }

    private var sections: [(section: SetupSection, names: [String])] {
        SetupSection.present(SetupSection.safety, in: Set(store.parameters.map(\.name)))
    }
}

struct PowerView: View {
    @ObservedObject var store: ParametersStore
    @ObservedObject var power: PowerStore

    var body: some View {
        SetupPageBody(title: "Power",
                      note: "What the vehicle measures its pack with, and how that measurement is scaled.") {
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: "Measured now")
                GroupCard {
                    if power.battery.available {
                        GroupRow(title: "Voltage", value: power.battery.voltageText, showSeparator: false)
                        GroupRow(title: "Current", value: power.battery.currentText)
                        GroupRow(title: "Remaining", value: power.battery.percentText)
                    } else {
                        EmptyStateRow(text: "This vehicle is not reporting a battery.")
                    }
                }
            }

            if store.loading {
                GroupCard { EmptyStateRow(text: "Reading parameters from the vehicle\u{2026}") }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: "This vehicle reports none of the battery parameters this page knows about.")
                }
            } else {
                ForEach(sections, id: \.section.id) { entry in
                    VStack(alignment: .leading, spacing: 0) {
                        SectionLabel(text: entry.section.title)
                        GroupCard {
                            ForEach(Array(entry.names.enumerated()), id: \.element) { index, name in
                                if let parameter = store.parameter(named: name) {
                                    ParameterRow(parameter: parameter, showSeparator: index > 0) {
                                        store.write(parameter, $0)
                                    }
                                }
                            }
                        }
                        Text(entry.section.note)
                            .font(.caption).foregroundColor(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                            .padding(.horizontal, Overlay.horizontalPadding)
                            .padding(.top, Overlay.unit * 0.35)
                    }
                }
            }
        }
        .onAppear {
            store.load()
            power.start()
        }
        .onDisappear(perform: power.stop)
    }

    private var sections: [(section: SetupSection, names: [String])] {
        SetupSection.present(SetupSection.power, in: Set(store.parameters.map(\.name)))
    }
}

struct TuningView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        SetupPageBody(title: "Tuning",
                      note: "The gains that decide how the vehicle answers the sticks. Change one thing at a time and fly it.") {
            if store.loading {
                GroupCard { EmptyStateRow(text: "Reading parameters from the vehicle\u{2026}") }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: "This vehicle reports none of the tuning parameters this page knows about.")
                }
            } else {
                ForEach(sections, id: \.section.id) { entry in
                    VStack(alignment: .leading, spacing: 0) {
                        SectionLabel(text: entry.section.title)
                        GroupCard {
                            ForEach(Array(entry.names.enumerated()), id: \.element) { index, name in
                                if let parameter = store.parameter(named: name) {
                                    ParameterRow(parameter: parameter, showSeparator: index > 0) {
                                        store.write(parameter, $0)
                                    }
                                }
                            }
                        }
                        Text(entry.section.note)
                            .font(.caption).foregroundColor(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                            .padding(.horizontal, Overlay.horizontalPadding)
                            .padding(.top, Overlay.unit * 0.35)
                    }
                }

                Text("AutoTune and in-flight tuning are flown, not configured, and stay in the Qt view.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Overlay.horizontalPadding)
            }
        }
        .onAppear(perform: store.load)
    }

    private var sections: [(section: SetupSection, names: [String])] {
        SetupSection.present(SetupSection.tuning, in: Set(store.parameters.map(\.name)))
    }
}

struct FrameView: View {
    @ObservedObject var store: ParametersStore
    @ObservedObject var frame: FrameStore

    var body: some View {
        SetupPageBody(title: "Frame",
                      note: "Which airframe this is, and what the firmware made of it.") {
            if store.loading {
                GroupCard { EmptyStateRow(text: "Reading parameters from the vehicle\u{2026}") }
            } else if sections.isEmpty {
                GroupCard {
                    EmptyStateRow(text: "This vehicle does not report a frame class.")
                }
            } else {
                if needsFrameClass {
                    Label("No airframe is selected. The vehicle will not arm until one is.",
                          systemImage: "exclamationmark.triangle.fill")
                        .font(.callout)
                        .foregroundColor(.orange)
                        .fixedSize(horizontal: false, vertical: true)
                }

                ForEach(sections, id: \.section.id) { entry in
                    VStack(alignment: .leading, spacing: 0) {
                        SectionLabel(text: entry.section.title)
                        GroupCard {
                            ForEach(Array(entry.names.enumerated()), id: \.element) { index, name in
                                if let parameter = store.parameter(named: name) {
                                    ParameterRow(parameter: parameter, showSeparator: index > 0) {
                                        store.write(parameter, $0)
                                    }
                                }
                            }
                        }
                        Text(entry.section.note)
                            .font(.caption).foregroundColor(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                            .padding(.horizontal, Overlay.horizontalPadding)
                            .padding(.top, Overlay.unit * 0.35)
                    }
                }
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

    private var sections: [(section: SetupSection, names: [String])] {
        SetupSection.present(SetupSection.frame, in: Set(store.parameters.map(\.name)))
    }

    private var needsFrameClass: Bool {
        FrameSetup.needsFrameClass(store.parameter(named: "FRAME_CLASS")?.selectedOption?.raw)
    }
}

struct FlightModesView: View {
    @ObservedObject var store: ParametersStore

    var body: some View {
        SetupPageBody(title: "Flight Modes",
                      note: "Which mode each position of the transmitter switch selects.") {
            if store.loading {
                GroupCard { EmptyStateRow(text: "Reading parameters from the vehicle\u{2026}") }
            } else if positions.isEmpty {
                GroupCard { EmptyStateRow(text: "This vehicle does not report a six-position mode switch.") }
            } else {
                if let channel = store.parameter(named: FlightModePosition.channelParameter) {
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
                    GroupCard {
                        ForEach(positions, id: \.index) { position in
                            if let parameter = store.parameter(named: position.parameter) {
                                let active = parameter.value == store.currentFlightMode
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
                                                 ParameterEditor(parameter: parameter) {
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
        .onAppear(perform: store.load)
    }

    private var positions: [FlightModePosition] {
        FlightModePosition.present(in: Set(store.parameters.map(\.name)))
    }
}

struct SetupSummaryView: View {
    @ObservedObject var store: VehicleComponentsStore
    @ObservedObject var sensors: SensorsStore
    @ObservedObject var selection: PageSelection

    private var readiness: VehicleReadiness {
        store.readiness(sensorFaults: sensors.failing.map(\.name))
    }

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
                                let opens = SetupPage.all.contains(component.name)
                                let faulted = component.name == "Sensors" && !sensors.failing.isEmpty
                                let good = component.setupComplete && !faulted
                                GroupRow(title: component.name,
                                         value: !component.setupComplete ? "Needs setup"
                                             : faulted ? "Reporting a fault" : "",
                                         showSeparator: component.id != store.components.first?.id,
                                         leading: {
                                             Tile(symbol: SetupPage.symbol(for: component.name),
                                                  colour: SetupPage.colour(for: component.name))
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
                                    .onTapGesture { if opens { selection.page = component.name } }
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
                StatusPill(text: readiness.ready ? "Ready" : "Check", good: readiness.ready)
            }
            .padding(Overlay.unit)
        }
    }
}

enum SetupPage {
    static let all = ["Summary", "Sensors", "Frame", "Flight Modes", "Safety", "Power", "Tuning", "Parameters"]

    static func symbol(for page: String) -> String {
        switch page {
        case "Summary": return "airplane"
        case "Sensors": return "gauge"
        case "Flight Modes": return "slider.horizontal.3"
        case "Safety": return "shield.fill"
        case "Parameters": return "list.bullet"
        case "Radio": return "antenna.radiowaves.left.and.right"
        case "Frame": return "square.on.square"
        case "Power": return "bolt.fill"
        case "Motors": return "fan.fill"
        case "Camera": return "camera.fill"
        case "Tuning": return "dial.min"
        default: return "gearshape.fill"
        }
    }

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

struct VehicleSetupView: View {
    @ObservedObject var parameters: ParametersStore
    @ObservedObject var sensors: SensorsStore
    @ObservedObject var components: VehicleComponentsStore
    @ObservedObject var power: PowerStore
    @ObservedObject var frame: FrameStore
    @ObservedObject var selection: PageSelection

    var body: some View {
        HStack(spacing: 0) {
            List(selection: Binding(
                get: { Optional(selection.page) },
                set: { selection.page = $0 ?? selection.page })
            ) {
                Section("Vehicle") {
                    row("Summary")
                }
                Section("Setup") {
                    row("Sensors", badge: !sensors.failing.isEmpty)
                    row("Frame")
                    row("Flight Modes")
                    row("Safety")
                    row("Power")
                    row("Tuning")
                }
                Section("Advanced") {
                    row("Parameters")
                }
            }
            .listStyle(.sidebar)
            .frame(width: 210)

            Divider()

            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(minWidth: 860, minHeight: 560)
        .onAppear(perform: components.reload)
    }

    private func row(_ name: String, badge: Bool = false) -> some View {
        SidebarRow(title: name,
                   symbol: SetupPage.symbol(for: name),
                   colour: SetupPage.colour(for: name),
                   badge: badge)
            .tag(name)
    }

    @ViewBuilder private var content: some View {
        switch selection.page {
        case "Parameters": ParametersView(store: parameters)
        case "Safety": SafetyView(store: parameters)
        case "Power": PowerView(store: parameters, power: power)
        case "Frame": FrameView(store: parameters, frame: frame)
        case "Tuning": TuningView(store: parameters)
        case "Flight Modes": FlightModesView(store: parameters)
        case "Sensors": SensorsView(store: sensors)
        default: SetupSummaryView(store: components, sensors: sensors, selection: selection)
        }
    }
}

final class VehicleSetupWindow: NSObject, NSWindowDelegate {
    static let shared = VehicleSetupWindow()

    private let parameters = ParametersStore()
    private let sensors = SensorsStore()
    private let components = VehicleComponentsStore()
    private let power = PowerStore()
    private let frame = FrameStore()
    private let selection = PageSelection(owner: "vehicleSetup", pages: SetupPage.all)
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(parameters)
        NativeProbe.register(sensors)
        NativeProbe.register(components)
        NativeProbe.register(power)
        NativeProbe.register(frame)
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
            power: power, frame: frame, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        sensors.stop()
        window = nil
    }
}
