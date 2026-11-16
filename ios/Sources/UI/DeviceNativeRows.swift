import SwiftUI

struct DeviceGroup<Content: View>: View {
    let title: String
    let summary: String
    let badge: DeviceBadge?
    let content: () -> Content
    @Environment(\.LocalNativeList) private var nativeList
    @State private var open: Bool

    init(_ title: String, summary: String = "", badge: DeviceBadge? = nil, initiallyOpen: Bool = false, @ViewBuilder content: @escaping () -> Content) {
        self.title = title
        self.summary = summary
        self.badge = badge
        self.content = content
        _open = State(initialValue: initiallyOpen)
    }

    var body: some View {
        if nativeList {
            DisclosureGroup(isExpanded: $open) {
                content()
            } label: {
                HStack(spacing: Space.s3) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(title).foregroundStyle(Color.primary).multilineTextAlignment(.leading)
                        if !summary.isEmpty {
                            Text(summary).font(.subheadline).foregroundStyle(Color.secondary).multilineTextAlignment(.leading)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    if let badge { DeviceBadgeText(badge: badge) }
                }
                .accessibilityElement(children: .combine)
            }
        } else {
            DeviceDisclosure(title, summary: summary, badge: badge, initiallyOpen: open, content: content)
        }
    }
}

struct DeviceDestructiveButton: View {
    let label: String
    let request: DeviceRequest
    var success: String = ""
    let confirm: ActionConfirm
    var onDone: () -> Void = {}
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var asking = false

    private func go() {
        let request = request, success = success, notice = notice, onDone = onDone, action = action
        Task { @MainActor in if await action.run(request, success: success, notice: notice) { onDone() } }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button(label, role: .destructive) { asking = true }
                .disabled(action.busy)
            if let refusal = action.refusal, refusal.field.isEmpty {
                DeviceRefusalText(refusal: refusal)
            }
        }
        .deviceConfirm(confirm, isPresented: $asking) { go() }
    }
}

struct DeviceRemoveButton: View {
    let label: String
    let action: () -> Void

    var body: some View {
        Button(role: .destructive, action: action) {
            Image(systemName: "minus.circle.fill")
                .font(.title3)
                .foregroundStyle(Color.red)
                .frame(width: Metrics.iconButton, height: Metrics.iconButton)
                .contentShape(Rectangle())
        }
        .buttonStyle(.borderless)
        .accessibilityLabel(label)
    }
}
