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

struct VehicleSetupView: View {
    @ObservedObject var parameters: ParametersStore
    @State private var page: String? = "Parameters"

    private let pages = ["Parameters"]

    var body: some View {
        HStack(spacing: 0) {
            List(pages, id: \.self, selection: $page) { name in
                Text(name).tag(name)
            }
            .listStyle(.sidebar)
            .frame(width: 190)
            Divider()
            ParametersView(store: parameters)
        }
        .frame(minWidth: 760, minHeight: 500)
    }
}

final class VehicleSetupWindow: NSObject, NSWindowDelegate {
    static let shared = VehicleSetupWindow()

    private let parameters = ParametersStore()
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(parameters)
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
        window.contentView = NSHostingView(rootView: VehicleSetupView(parameters: parameters))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        window = nil
    }
}
