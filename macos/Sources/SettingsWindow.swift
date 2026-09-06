import AppKit
import SwiftUI

struct SettingsView: View {
    @ObservedObject var store: SettingsStore

    var body: some View {
        HStack(spacing: 0) {
            sidebar
            Divider()
            detail
        }
        .frame(minWidth: 720, minHeight: 460)
        .onAppear(perform: store.load)
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            SearchField(text: $store.search)
                .padding(8)
            List(store.pages, selection: $store.selected) { page in
                Text(page.title).tag(page.id)
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
        } else if store.sections.isEmpty {
            notice(store.search.isEmpty
                   ? "This page has no editable settings."
                   : "No setting matches “\(store.search)”.")
        } else {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    ForEach(store.sections) { section in
                        VStack(alignment: .leading, spacing: 0) {
                            Text(section.title)
                                .font(.headline)
                                .padding(.bottom, 8)
                            ForEach(Array(section.facts.enumerated()), id: \.element.id) { index, fact in
                                if index > 0 { Divider() }
                                FactRow(fact: fact, showsUnits: section.showsUnits) { store.write(fact, $0) }
                                    .padding(.vertical, 7)
                            }
                        }
                    }
                }
                .padding(20)
            }
        }
    }

    private func notice(_ text: String) -> some View {
        VStack {
            Spacer()
            Text(text).foregroundColor(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity)
    }
}

// NSSearchField rather than a TextField: it brings the magnifier, the clear button
// and the Escape-to-clear behaviour macOS users already expect.
struct SearchField: NSViewRepresentable {
    @Binding var text: String

    func makeNSView(context: Context) -> NSSearchField {
        let field = NSSearchField()
        field.placeholderString = "Search settings"
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

struct FactRow: View {
    let fact: Fact
    let showsUnits: Bool
    let write: (Any) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 16) {
            Text(fact.title)
                .frame(maxWidth: .infinity, alignment: .leading)
            control
                .frame(width: 230, alignment: .trailing)
                .disabled(fact.readOnly)
            if showsUnits {
                Text(fact.units)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .frame(width: 44, alignment: .leading)
            }
        }
    }

    @ViewBuilder
    private var control: some View {
        switch fact.kind {
        case .toggle:
            Toggle("", isOn: Binding(get: { fact.boolValue }, set: { write($0) }))
                .labelsHidden()
                .frame(maxWidth: .infinity, alignment: .trailing)

        case let .choice(labels, values):
            Picker("", selection: Binding(
                get: { values.firstIndex(of: fact.intValue) ?? 0 },
                set: { write(values[$0]) })
            ) {
                ForEach(Array(labels.enumerated()), id: \.offset) { index, label in
                    Text(label).tag(index)
                }
            }
            .labelsHidden()

        case .text, .number:
            TextField("", text: $draft)
                .multilineTextAlignment(.trailing)
                .focused($editing)
                .onAppear { draft = fact.stringValue }
                .onChange(of: fact.stringValue) { latest in
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
        guard case let .number(_, minimum, maximum) = fact.kind else { return "" }
        switch (minimum, maximum) {
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
        guard draft != fact.stringValue else { return }
        switch fact.kind {
        case .text:
            write(draft)
        case .number:
            // Reject junk locally rather than sending it and silently getting the old
            // value back; restore what is actually stored so the field never lies.
            guard let number = Double(draft.replacingOccurrences(of: ",", with: ".")) else {
                draft = fact.stringValue
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
        window.contentView = NSHostingView(rootView: SettingsView(store: store))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    // Facts are cached per group, so anything changed elsewhere in the app (QML, a
    // vehicle, another window) would otherwise show stale until reopened.
    func windowDidBecomeKey(_ notification: Notification) {
        store.load()
    }

    func windowWillClose(_ notification: Notification) {
        window = nil
    }
}
