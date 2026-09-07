import AppKit
import SwiftUI

struct MissionView: View {
    @ObservedObject var store: MissionStore

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            if !store.status.isEmpty {
                Notice(text: store.status)
            } else {
                // The map answers "where does this go", the list answers "what does it
                // do"; a mission needs both and neither replaces the other.
                VSplitView {
                    MissionMap(items: store.items, vehicle: store.vehiclePosition)
                        .frame(minHeight: 220)
                    list
                        .frame(minHeight: 120)
                }
            }
        }
        .onAppear(perform: store.reload)
    }

    private var header: some View {
        HStack {
            Text("\(store.items.count) item\(store.items.count == 1 ? "" : "s")")
                .foregroundColor(.secondary)
            if store.syncing {
                ProgressView().controlSize(.small)
                Text("Reading from vehicle…").font(.caption).foregroundColor(.secondary)
            }
            Spacer()
            Button("Read from Vehicle", action: store.downloadFromVehicle)
                .disabled(store.syncing)
        }
        .padding(10)
    }

    private var list: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(store.items) { item in
                    Divider()
                    row(item)
                }
            }
            .padding(.horizontal, 14)
        }
    }

    private func row(_ item: MissionItem) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text("\(item.sequence)")
                .font(.body.monospacedDigit())
                .foregroundColor(.secondary)
                .frame(width: 28, alignment: .trailing)

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(item.command)
                    // The vehicle's current target, so an operator reading a mission
                    // mid-flight can see where it has got to.
                    if item.isCurrent {
                        Text("current")
                            .font(.caption2)
                            .padding(.horizontal, 5).padding(.vertical, 1)
                            .background(Color.accentColor.opacity(0.18))
                            .cornerRadius(3)
                    }
                }
                if !item.description.isEmpty {
                    Text(item.description)
                        .font(.caption).foregroundColor(.secondary)
                }
            }

            Spacer()
            Text(item.positionText)
                .font(.caption.monospacedDigit())
                .foregroundColor(item.hasPosition ? .secondary : Color.secondary.opacity(0.5))
            Text(item.altitudeText)
                .font(.body.monospacedDigit())
                .frame(width: 70, alignment: .trailing)
        }
        .padding(.vertical, 8)
    }
}

struct PlanView: View {
    @ObservedObject var mission: MissionStore
    @ObservedObject var selection: PageSelection

    private let pages = ["Mission"]

    var body: some View {
        HStack(spacing: 0) {
            List(pages, id: \.self, selection: Binding(
                get: { Optional(selection.page) },
                set: { selection.page = $0 ?? selection.page })
            ) { name in
                Text(name).tag(name)
            }
            .listStyle(.sidebar)
            .frame(width: 170)
            Divider()
            MissionView(store: mission)
        }
        .frame(minWidth: 820, minHeight: 620)
    }
}

final class PlanWindow: NSObject, NSWindowDelegate {
    static let shared = PlanWindow()

    private let mission = MissionStore()
    private let selection = PageSelection(owner: "plan", pages: ["Mission"])
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(mission)
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
            contentRect: NSRect(x: 0, y: 0, width: 840, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Plan"
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(rootView: PlanView(mission: mission, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        window = nil
    }
}
