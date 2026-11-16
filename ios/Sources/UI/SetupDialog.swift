import SwiftUI

struct DialogButton {
    let title: String
    var enabled: Bool = true
    let action: () -> Void
}

private struct DialogButtons: ViewModifier {
    let cancel: DialogButton?
    let primary: DialogButton?

    func body(content: Content) -> some View {
        content.toolbar {
            if let cancel {
                ToolbarItem(placement: .cancellationAction) {
                    Button(cancel.title, action: cancel.action).disabled(!cancel.enabled)
                }
            }
            if let primary {
                ToolbarItem(placement: .confirmationAction) {
                    Button(primary.title, action: primary.action).disabled(!primary.enabled)
                }
            }
        }
    }
}

extension View {
    func dialogButtons(cancel: DialogButton? = nil, primary: DialogButton? = nil) -> some View {
        modifier(DialogButtons(cancel: cancel, primary: primary))
    }
}

struct SetupDialog<Content: View>: View {
    let title: String
    var cancel: DialogButton? = nil
    var primary: DialogButton? = nil
    @ViewBuilder let content: () -> Content

    var body: some View {
        NavigationStack {
            NativeForm {
                Section { content() }
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .dialogButtons(cancel: cancel, primary: primary)
        }
        .presentationDetents([.medium, .large])
    }
}

struct NativeDialogSheet<Content: View>: View {
    let title: String
    let onDismissRequest: () -> Void
    var detents: Set<PresentationDetent> = [.medium, .large]
    @ViewBuilder let content: () -> Content

    var body: some View {
        Color.clear
            .invisibleAnchor()
            .queuedSheet(isPresented: Binding(get: { true }, set: { shown in if !shown { onDismissRequest() } })) {
                NativeDialogStack(title: title, detents: detents, content: content)
            }
    }
}

struct NativeDialogStack<Content: View>: View {
    let title: String
    var detents: Set<PresentationDetent> = [.medium, .large]
    @ViewBuilder let content: () -> Content
    @Environment(\.immersive) private var immersive

    var body: some View {
        NavigationStack {
            content()
                .navigationTitle(title)
                .navigationBarTitleDisplayMode(.inline)
        }
        .presentationDetents(detents)
        .statusBarHidden(immersive)
        .persistentSystemOverlays(immersive ? .hidden : .automatic)
    }
}
