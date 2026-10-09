import SwiftUI
import UIKit

extension EnvironmentValues {
    @Entry var immersive = false
    @Entry var sheetDepth = 0
}

let SHEET_POLL_MS = 50

private func changed(_ entry: SheetEntry, _ change: (inout SheetEntry) -> Void) -> SheetEntry {
    var next = entry
    change(&next)
    return next
}

struct SheetEntry: Equatable {
    let id: UUID
    let depth: Int
    let dialog: Bool
    var wanted = true
    var shown = false
    var pending = false
}

@MainActor
@Observable
final class SheetStack {
    static let shared = SheetStack()

    private(set) var entries: [SheetEntry] = []

    private func update(_ id: UUID, _ change: (SheetEntry) -> SheetEntry?) {
        entries = entries.compactMap { $0.id == id ? change($0) : $0 }
    }

    func request(_ id: UUID, _ depth: Int, dialog: Bool) {
        guard entries.contains(where: { $0.id == id }) else {
            entries = entries + [SheetEntry(id: id, depth: depth, dialog: dialog)]
            return
        }
        update(id) { entry in entry.wanted ? entry : changed(entry) { $0.wanted = true; $0.pending = true } }
    }

    func release(_ id: UUID) {
        update(id) { entry in entry.shown ? changed(entry) { $0.wanted = false; $0.pending = false } : nil }
    }

    func abandon(_ id: UUID) {
        update(id) { entry in entry.shown ? entry : nil }
    }

    func appeared(_ id: UUID) {
        update(id) { entry in changed(entry) { $0.shown = true } }
    }

    func disappeared(_ id: UUID) {
        update(id) { entry in entry.pending ? SheetEntry(id: entry.id, depth: entry.depth, dialog: entry.dialog) : nil }
    }

    func level(dialogs: Bool) -> Int {
        entries.filter { $0.shown && $0.wanted && (dialogs || !$0.dialog) }.map { $0.depth + 1 }.max() ?? 0
    }
}

@MainActor
final class PresenterProbe {
    weak var view: UIView?

    var controller: UIViewController? {
        view.flatMap { start in sequence(first: start as UIResponder) { $0.next }.lazy.compactMap { $0 as? UIViewController }.first }
    }
}

struct PresenterProbeView: UIViewRepresentable {
    let probe: PresenterProbe

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.isUserInteractionEnabled = false
        view.isAccessibilityElement = false
        probe.view = view
        return view
    }

    func updateUIView(_ view: UIView, context: Context) { probe.view = view }
}

@MainActor
func presenterFree(_ controller: UIViewController?) -> Bool {
    guard let controller, controller.viewIfLoaded?.window != nil else { return false }
    return controller.presentedViewController == nil && !controller.isBeingPresented && !controller.isBeingDismissed
}

private struct QueuedSheet<Sheet: View>: ViewModifier {
    @Binding var isPresented: Bool
    let dialog: Bool
    let onDismiss: (() -> Void)?
    let sheet: () -> Sheet
    @Environment(\.sheetDepth) private var depth
    @State private var id = UUID()

    func body(content: Content) -> some View {
        let stack = SheetStack.shared
        content
            .onChange(of: isPresented, initial: true) { _, wanted in
                if wanted { stack.request(id, depth, dialog: dialog) } else { stack.release(id) }
            }
            .onDisappear { stack.abandon(id) }
            .sheet(isPresented: $isPresented, onDismiss: onDismiss) {
                ZStack { sheet() }
                    .background { AppDialogsHost() }
                    .onAppear { stack.appeared(id) }
                    .onDisappear { stack.disappeared(id) }
                    .environment(\.sheetDepth, depth + 1)
            }
    }
}

private struct QueuedItemSheet<Item: Identifiable, Sheet: View>: ViewModifier {
    @Binding var item: Item?
    let onDismiss: (() -> Void)?
    let sheet: (Item) -> Sheet
    @State private var last: Item?

    func body(content: Content) -> some View {
        content
            .onChange(of: item?.id, initial: true) { if let item { last = item } }
            .queuedSheet(isPresented: Binding(get: { item != nil }, set: { shown in if !shown { item = nil } }), onDismiss: onDismiss) {
                if let shown = item ?? last { sheet(shown).id(shown.id) }
            }
    }
}

extension View {
    func queuedSheet<Sheet: View>(isPresented: Binding<Bool>, onDismiss: (() -> Void)? = nil, dialog: Bool = false, @ViewBuilder content: @escaping () -> Sheet) -> some View {
        modifier(QueuedSheet(isPresented: isPresented, dialog: dialog, onDismiss: onDismiss, sheet: content))
    }

    func queuedSheet<Item: Identifiable, Sheet: View>(item: Binding<Item?>, onDismiss: (() -> Void)? = nil, @ViewBuilder content: @escaping (Item) -> Sheet) -> some View {
        modifier(QueuedItemSheet(item: item, onDismiss: onDismiss, sheet: content))
    }
}

private struct SheetBody<Content: View>: View {
    let skipPartiallyExpanded: Bool
    let content: () -> Content
    @Environment(\.theme) private var theme
    @Environment(\.immersive) private var immersive

    var body: some View {
        VStack(alignment: .leading, spacing: 0) { content() }
            .padding(.top, Space.s6)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .presentationDetents(skipPartiallyExpanded ? [.large] : [.medium, .large])
            .presentationDragIndicator(.visible)
            .presentationCornerRadius(Corner.extraLarge)
            .presentationBackground(theme.colors.surfaceContainerLow)
            .statusBarHidden(immersive)
            .persistentSystemOverlays(immersive ? .hidden : .automatic)
    }
}

struct AircastSheet<Content: View>: View {
    let onDismissRequest: () -> Void
    var skipPartiallyExpanded = false
    @ViewBuilder let content: () -> Content

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .queuedSheet(isPresented: Binding(get: { true }, set: { shown in if !shown { onDismissRequest() } })) {
                SheetBody(skipPartiallyExpanded: skipPartiallyExpanded, content: content)
            }
    }
}
