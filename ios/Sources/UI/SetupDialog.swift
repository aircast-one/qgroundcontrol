import SwiftUI

// A button in a native dialog's navigation bar: Cancel / Close on the leading side, the primary action trailing.
struct DialogButton {
    let title: String
    var enabled: Bool = true
    let action: () -> Void
}

// The toolbar of a native dialog: one dismiss control (leading) and the primary action (trailing). On the Mac a sheet
// shows both as its buttons.
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

// A setup dialog inside a sheet its caller presents: its own NavigationStack with the title inline, the dismiss
// control leading and the primary action trailing, and the content as rows of a NativeForm section.
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
        .nativeSheetFrame()
    }
}

// A native modal sheet shown for as long as this view exists: the native counterpart of AircastSheet / PlanDialog
// for dialogs, queued behind other sheets like them. The content brings its own NativeForm and toolbar buttons
// (dialogButtons, pageCancelAction, pagePrimaryAction).
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

// The navigation stack, title and size of a native dialog sheet, for sheets presented by their caller.
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
        .nativeSheetFrame()
        .statusBarHidden(immersive)
        .persistentSystemOverlays(immersive ? .hidden : .automatic)
    }
}
