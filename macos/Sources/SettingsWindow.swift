import AppKit
import SwiftUI

struct SettingsView: View {
    @ObservedObject var store: SettingsStore
    @ObservedObject var links: LinksStore
    @ObservedObject var video: VideoStore

    var body: some View {
        HStack(spacing: 0) {
            sidebar
            Divider()
            detail
        }
        .frame(minWidth: 720, minHeight: 460)
        .onAppear(perform: store.load)
        .writeFailureAlert($store.writeFailure, $links.writeFailure, $video.writeFailure)
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            SearchField(text: $store.search, placeholder: "Search settings")
                .padding(8)
            List(store.pages, selection: $store.selected) { page in
                SidebarRow(title: page.title,
                           symbol: SettingsPage.symbol(for: page.title),
                           colour: SettingsPage.colour(for: page.title))
                    .tag(page.id)
            }
            .listStyle(.sidebar)
            .disabled(!store.search.isEmpty)
            .opacity(store.search.isEmpty ? 1 : 0.4)
            .onChange(of: store.selected) { _ in store.refresh() }
        }
        .frame(width: 210)
    }

    @ViewBuilder
    private var detail: some View {
        if let message = store.loadError {
            notice(message)
        } else if showsAbout {
            SetupPageBody(title: pageTitle) { about }
        } else if store.sections.isEmpty && !showsLinks {
            notice(store.search.isEmpty
                   ? "This page has no editable settings."
                   : "No setting matches “\(store.search)”.")
        } else {
            SetupPageBody(title: pageTitle) {
                if showsLinks {
                    ConnectionsSection(store: links)
                }
                if showsVideoSources {
                    videoSources
                }
                ForEach(store.sections) { section in
                    VStack(alignment: .leading, spacing: 0) {
                        SectionLabel(text: section.title)
                        GroupCard {
                            ForEach(Array(section.controls.enumerated()), id: \.element.id) { index, fact in
                                GroupRow(title: fact.label,
                                         description: section.showsUnits ? fact.units : "",
                                         showSeparator: index > 0,
                                         trailing: {
                                             FactControl(fact: fact) { store.write(fact, $0) }
                                                 .disabled(fact.readOnly)
                                                 .frame(width: 200, alignment: .trailing)
                                         })
                            }
                        }
                    }
                }
            }
        }
    }

    private var pageTitle: String {
        store.search.trimmingCharacters(in: .whitespaces).isEmpty
            ? (store.pages.first { $0.id == store.selected }?.title ?? "Settings")
            : "Search results"
    }

    private var showsLinks: Bool {
        store.search.trimmingCharacters(in: .whitespaces).isEmpty
            && store.pages.first { $0.id == store.selected }?.showsLinks == true
    }

    private var showsVideoSources: Bool {
        store.search.trimmingCharacters(in: .whitespaces).isEmpty
            && store.pages.first { $0.id == store.selected }?.showsVideoSources == true
    }

    private var videoSources: some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionLabel(text: "Cameras")
            GroupCard {
                ForEach(Array(video.sources.enumerated()), id: \.element.id) { row, source in
                    GroupRow(title: source.title,
                             description: source.enabled
                                 ? (source.misconfigured
                                     ? "\(source.source) \u{00B7} no address"
                                     : source.source)
                                 : source.source,
                             showSeparator: row > 0,
                             trailing: {
                                 HStack(spacing: Overlay.step) {
                                     if VideoSources.repairs(source) != nil {
                                         Button("Use name") { video.repair(source) }
                                             .fixedSize()
                                             .help("The address was typed into this camera's name; move it to the address")
                                     }
                                     ValueField(value: source.url, units: "") { entered in
                                         var replacement = source
                                         replacement.url = entered
                                         video.write(replacement)
                                     }
                                     .frame(width: 200)
                                     .disabled(!source.enabled)
                                 }
                             })
                }
            }
            Text("A camera needs an address before it can show anything.")
                .font(.caption).foregroundColor(.secondary)
                .padding(.horizontal, Overlay.horizontalPadding)
                .padding(.top, Overlay.unit * 0.35)
        }
        .onAppear(perform: video.loadSources)
    }

    private var showsAbout: Bool {
        store.search.trimmingCharacters(in: .whitespaces).isEmpty
            && store.pages.first { $0.id == store.selected }?.showsAbout == true
    }

    private var about: some View {
        let info = AboutInfo.read(Bundle.main.infoDictionary)
        return VStack(alignment: .leading, spacing: Overlay.unit * 0.9) {
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: "Version")
                GroupCard {
                    GroupRow(title: info.name, value: info.versionText, showSeparator: false)
                }
            }

            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: "Help")
                GroupCard {
                    ForEach(Array(HelpLink.all.enumerated()), id: \.element.id) { row, link in
                        GroupRow(title: link.name,
                                 description: link.host,
                                 showSeparator: row > 0,
                                 trailing: {
                                     Button("Open") {
                                         guard let url = URL(string: link.url) else { return }
                                         NSWorkspace.shared.open(url)
                                     }
                                 })
                    }
                }
            }
        }
    }

    private func notice(_ text: String) -> some View { Notice(text: text) }
}

struct SearchField: NSViewRepresentable {
    @Binding var text: String
    var placeholder = "Search"

    func makeNSView(context: Context) -> NSSearchField {
        let field = NSSearchField()
        field.placeholderString = placeholder
        field.delegate = context.coordinator
        return field
    }

    func updateNSView(_ field: NSSearchField, context: Context) {
        if field.stringValue != text { field.stringValue = text }
    }

    func makeCoordinator() -> Coordinator { Coordinator(text: $text) }

    final class Coordinator: NSObject, NSSearchFieldDelegate {
        private let text: Binding<String>
        init(text: Binding<String>) { self.text = text }

        func controlTextDidChange(_ notification: Notification) {
            guard let field = notification.object as? NSSearchField else { return }
            text.wrappedValue = field.stringValue
        }
    }
}

struct FactControl: View {
    let fact: SettingsControl
    let write: (Any) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    @ViewBuilder
    var body: some View {
        switch fact.kind {
        case .toggle:
            Toggle("", isOn: Binding(get: { fact.boolValue }, set: { write($0) }))
                .labelsHidden()
                .frame(maxWidth: .infinity, alignment: .trailing)

        case .choice:
            Picker("", selection: Binding(
                get: { fact.options.firstIndex { $0.raw == fact.valueString } ?? 0 },
                set: { if fact.options.indices.contains($0) { write(fact.options[$0].writable) } })
            ) {
                ForEach(Array(fact.options.enumerated()), id: \.offset) { index, option in
                    Text(option.label).tag(index)
                }
            }
            .labelsHidden()

        case .bitmask where fact.drawsBits:
            VStack(alignment: .leading, spacing: 2) {
                ForEach(fact.bits) { bit in
                    Toggle(bit.label, isOn: Binding(
                        get: { bit.set },
                        set: { write(fact.toggling(bit, on: $0)) }))
                        .disabled(fact.readOnly)
                }
            }
            .frame(maxWidth: .infinity, alignment: .trailing)

        case .text, .number, .bitmask, .unknown:
            TextField("", text: $draft)
                .textFieldStyle(.roundedBorder)
                .multilineTextAlignment(.trailing)
                .focused($editing)
                .onAppear { draft = fact.valueString }
                .onChange(of: fact.valueString) { latest in
                    if !editing { draft = latest }
                }
                .onSubmit(commit)
                .onChange(of: editing) { focused in
                    if !focused { commit() }
                }
                .help(rangeHint)
        }
    }

    private var rangeHint: String {
        guard fact.kind == .number else { return "" }
        switch (fact.minimum, fact.maximum) {
        case let (min?, max?): return "\(compact(min)) to \(compact(max))"
        case let (min?, nil): return "at least \(compact(min))"
        case let (nil, max?): return "at most \(compact(max))"
        default: return ""
        }
    }

    private func compact(_ value: Double) -> String {
        value == value.rounded() ? String(Int(value)) : String(format: "%g", value)
    }

    private func commit() {
        guard draft != fact.valueString else { return }
        switch fact.kind {
        case .text:
            write(draft)
        case .number:
            guard let number = Double(draft.replacingOccurrences(of: ",", with: ".")) else {
                draft = fact.valueString
                return
            }
            write(number)
        default:
            break
        }
    }
}

final class SettingsWindow: NSObject, NSWindowDelegate {
    static let shared = SettingsWindow()

    private let store = SettingsStore()
    private let links = LinksStore()
    private let video = VideoStore.shared

    override init() {
        super.init()
        NativeProbe.register(store)
        NativeProbe.register(links)
    }
    private var window: NSWindow?

    @objc func showFromMenu() {
        show()
    }

    func show() {
        if let window {
            store.load()
            window.makeKeyAndOrderFront(nil)
            return
        }

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 760, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Settings"
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(rootView: SettingsView(store: store, links: links, video: video))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowDidBecomeKey(_ notification: Notification) {
        store.load()
    }

    func windowWillClose(_ notification: Notification) {
        window = nil
    }
}

extension SettingsPage {
    static func symbol(for title: String) -> String {
        switch title {
        case "General": return "gearshape.fill"
        case "Fly View": return "paperplane.fill"
        case "Plan View": return "map.fill"
        case "Video": return "video.fill"
        case "Maps": return "globe"
        case "Connections": return "cable.connector"
        case "MAVLink": return "antenna.radiowaves.left.and.right"
        case "Flight Modes": return "slider.horizontal.3"
        case "ADSB Server": return "dot.radiowaves.up.forward"
        case "Packet Radio": return "wifi"
        case "Remote ID": return "person.text.rectangle"
        case "RTK GPS": return "location.fill"
        case "Firmware Upgrade": return "arrow.down.circle.fill"
        case "3D Viewer": return "cube.fill"
        default: return "gearshape.fill"
        }
    }

    static func colour(for title: String) -> Color {
        switch title {
        case "General": return .gray
        case "Fly View": return .accentColor
        case "Plan View": return .green
        case "Video": return .pink
        case "Maps": return .teal
        case "Connections": return .indigo
        case "MAVLink": return .purple
        case "Flight Modes": return .indigo
        case "ADSB Server": return .orange
        case "Packet Radio": return .blue
        case "Remote ID": return .brown
        case "RTK GPS": return .green
        case "Firmware Upgrade": return .red
        case "3D Viewer": return .cyan
        default: return .gray
        }
    }
}
