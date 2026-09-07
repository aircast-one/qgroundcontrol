import AppKit
import SwiftUI

struct ParametersView: View {
    @ObservedObject var store: ParametersStore
    @State private var editing: String?

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            if store.loading {
                Notice(text: "Reading parameters from the vehicle…")
            } else if !store.status.isEmpty {
                Notice(text: store.status)
            } else if store.visible.isEmpty {
                Notice(text: "No parameter matches this filter.")
            } else {
                list
            }
        }
        .onAppear(perform: store.load)
    }

    private var header: some View {
        HStack(spacing: 10) {
            Picker("", selection: $store.group) {
                Text("All groups").tag("")
                ForEach(store.groups, id: \.self) { group in
                    Text(group).tag(group)
                }
            }
            .labelsHidden()
            .frame(width: 150)

            SearchField(text: $store.search, placeholder: "Search parameters")

            Text("\(store.visible.count) of \(store.parameters.count)")
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(width: 110, alignment: .trailing)
        }
        .padding(10)
    }

    private var list: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(store.visible) { parameter in
                    Divider()
                    row(parameter)
                }
            }
            .padding(.horizontal, 14)
        }
    }

    private func row(_ parameter: Parameter) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(parameter.name).font(.body.monospaced())
                if !parameter.description.isEmpty {
                    Text(parameter.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            Spacer()
            if editing == parameter.id {
                LabelledField(label: "", value: parameter.value) {
                    store.write(parameter, $0)
                    editing = nil
                }
                .frame(width: 150)
            } else {
                Text(parameter.value)
                    .font(.body.monospacedDigit())
                Text(parameter.units)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .frame(width: 40, alignment: .leading)
                Button("Edit") { editing = parameter.id }
            }
        }
        .padding(.vertical, 7)
    }
}

struct SensorsView: View {
    @ObservedObject var store: SensorsStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                if !store.status.isEmpty {
                    Notice(text: store.status).frame(height: 200)
                } else {
                    summary
                    SectionCard(title: "Sensors") {
                        VStack(spacing: 0) {
                            ForEach(Array(store.sensors.enumerated()), id: \.element.id) { index, sensor in
                                if index > 0 { Divider() }
                                row(sensor)
                            }
                        }
                    }
                }
            }
            .padding(20)
        }
        .onAppear(perform: store.start)
        .onDisappear(perform: store.stop)
    }

    private var summary: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(store.failing.isEmpty ? Color.green : Color.red)
                .frame(width: 9, height: 9)
            Text(store.failing.isEmpty
                 ? "All enabled sensors are reporting healthy."
                 : "\(store.failing.count) sensor\(store.failing.count == 1 ? "" : "s") reporting a fault: \(store.failing.map(\.name).joined(separator: ", "))")
                .foregroundColor(store.failing.isEmpty ? .secondary : .red)
        }
    }

    private func row(_ sensor: SensorHealth) -> some View {
        HStack {
            Text(sensor.name)
                .foregroundColor(sensor.state == .disabled ? .secondary : .primary)
            Spacer()
            Text(label(sensor.state))
                .font(.caption)
                .foregroundColor(colour(sensor.state))
        }
        .padding(.vertical, 6)
    }

    private func label(_ state: SensorHealth.State) -> String {
        switch state {
        case .healthy: return "Healthy"
        case .unhealthy: return "Fault"
        case .disabled: return "Not enabled"
        }
    }

    private func colour(_ state: SensorHealth.State) -> Color {
        switch state {
        case .healthy: return .green
        case .unhealthy: return .red
        case .disabled: return .secondary
        }
    }
}

struct SafetyView: View {
    @ObservedObject var store: ParametersStore
    @State private var editing: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                if store.loading {
                    Notice(text: "Reading parameters from the vehicle…").frame(height: 200)
                } else if sections.isEmpty {
                    Notice(text: "This vehicle reports none of the safety parameters this page knows about.")
                        .frame(height: 200)
                } else {
                    ForEach(sections, id: \.section.id) { entry in
                        SectionCard(title: entry.section.title) {
                            VStack(alignment: .leading, spacing: 0) {
                                Text(entry.section.note)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                    .padding(.bottom, 8)
                                ForEach(Array(entry.names.enumerated()), id: \.element) { index, name in
                                    if index > 0 { Divider() }
                                    if let parameter = store.parameter(named: name) {
                                        row(parameter)
                                    }
                                }
                            }
                        }
                    }
                }
            }
            .padding(20)
        }
        .onAppear(perform: store.load)
    }

    private var sections: [(section: SafetySection, names: [String])] {
        SafetySection.present(in: Set(store.parameters.map(\.name)))
    }

    private func row(_ parameter: Parameter) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(parameter.description.isEmpty ? parameter.name : parameter.description)
                Text(parameter.name)
                    .font(.caption.monospaced())
                    .foregroundColor(.secondary)
            }
            Spacer()
            if editing == parameter.id {
                LabelledField(label: "", value: parameter.value) {
                    store.write(parameter, $0)
                    editing = nil
                }
                .frame(width: 150)
            } else {
                Text(parameter.value).font(.body.monospacedDigit())
                Text(parameter.units)
                    .font(.caption).foregroundColor(.secondary)
                    .frame(width: 40, alignment: .leading)
                Button("Edit") { editing = parameter.id }
            }
        }
        .padding(.vertical, 7)
    }
}

struct FlightModesView: View {
    @ObservedObject var store: ParametersStore
    @State private var editing: Int?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                if store.loading {
                    Notice(text: "Reading parameters from the vehicle…").frame(height: 200)
                } else if positions.isEmpty {
                    Notice(text: "This vehicle does not report a six-position mode switch.")
                        .frame(height: 200)
                } else {
                    if let channel = store.parameter(named: FlightModePosition.channelParameter) {
                        SectionCard(title: "Mode switch") {
                            VStack(alignment: .leading, spacing: 0) {
                                Text("Which transmitter channel selects the flight mode.")
                                    .font(.caption).foregroundColor(.secondary)
                                    .padding(.bottom, 8)
                                MetricRow(label: channel.description.isEmpty ? channel.name : channel.description,
                                          value: channel.value, units: "")
                            }
                        }
                    }

                    SectionCard(title: "Switch positions") {
                        VStack(alignment: .leading, spacing: 0) {
                            Text("The mode each position of that switch selects, with the PWM band the firmware uses for it.")
                                .font(.caption).foregroundColor(.secondary)
                                .padding(.bottom, 8)
                            ForEach(positions) { position in
                                if position.index > 1 { Divider() }
                                row(position)
                            }
                        }
                    }
                }
            }
            .padding(20)
        }
        .onAppear(perform: store.load)
    }

    private var positions: [FlightModePosition] {
        FlightModePosition.present(in: Set(store.parameters.map(\.name)))
    }

    @ViewBuilder
    private func row(_ position: FlightModePosition) -> some View {
        if let parameter = store.parameter(named: position.parameter) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Position \(position.index)")
                    Text(position.pwmRange)
                        .font(.caption).foregroundColor(.secondary)
                }
                .frame(width: 130, alignment: .leading)

                Spacer()

                if editing == position.index {
                    LabelledField(label: "", value: parameter.value) {
                        store.write(parameter, $0)
                        editing = nil
                    }
                    .frame(width: 180)
                } else {
                    // The active mode is what the operator is checking against the
                    // switch in their hand, so it is called out rather than inferred.
                    if parameter.value == store.currentFlightMode {
                        Text("now")
                            .font(.caption2)
                            .padding(.horizontal, 5).padding(.vertical, 1)
                            .background(Color.accentColor.opacity(0.18))
                            .cornerRadius(3)
                    }
                    Text(parameter.value)
                    Button("Edit") { editing = position.index }
                }
            }
            .padding(.vertical, 8)
        }
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
    static let all = ["Summary", "Sensors", "Flight Modes", "Safety", "Parameters"]

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
                    row("Flight Modes")
                    row("Safety")
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
    private let selection = PageSelection(owner: "vehicleSetup", pages: SetupPage.all)
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(parameters)
        NativeProbe.register(sensors)
        NativeProbe.register(components)
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
            parameters: parameters, sensors: sensors, components: components, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        sensors.stop()
        window = nil
    }
}
