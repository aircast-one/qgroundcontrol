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

struct VehicleSetupView: View {
    @ObservedObject var parameters: ParametersStore
    @ObservedObject var sensors: SensorsStore
    @State private var page: String? = "Sensors"

    private let pages = ["Sensors", "Parameters"]

    var body: some View {
        HStack(spacing: 0) {
            List(pages, id: \.self, selection: $page) { name in
                Text(name).tag(name)
            }
            .listStyle(.sidebar)
            .frame(width: 190)
            Divider()
            if page == "Parameters" {
                ParametersView(store: parameters)
            } else {
                SensorsView(store: sensors)
            }
        }
        .frame(minWidth: 760, minHeight: 500)
    }
}

final class VehicleSetupWindow: NSObject, NSWindowDelegate {
    static let shared = VehicleSetupWindow()

    private let parameters = ParametersStore()
    private let sensors = SensorsStore()
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(parameters)
        NativeProbe.register(sensors)
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
        window.contentView = NSHostingView(rootView: VehicleSetupView(parameters: parameters, sensors: sensors))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        sensors.stop()
        window = nil
    }
}
