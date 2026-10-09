import SwiftUI

let RAIL_WIDTH: CGFloat = 52
private let RAIL_ITEM: CGFloat = 44
private let RAIL_LABELLED_ITEM: CGFloat = 52
private let RAIL_LABEL_SIZE: CGFloat = 10
private let RAIL_ICON: CGFloat = 22
private let RAIL_ALPHA = 0.94
private let TOOLTIP_MS = 1500

struct PlanRail<Content: View>: View {
    @ViewBuilder let content: () -> Content
    @Environment(\.theme) private var theme
    @State private var contentHeight: CGFloat = 0

    var body: some View {
        ScrollView {
            VStack(spacing: 0) { content() }
                .frame(width: RAIL_WIDTH)
                .padding(.vertical, 2)
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { contentHeight = $0 }
        }
        .scrollIndicators(.hidden)
        .scrollBounceBehavior(.basedOnSize)
        .frame(width: RAIL_WIDTH)
        .frame(maxHeight: contentHeight)
        .background(theme.colors.surfaceContainer.opacity(RAIL_ALPHA), in: RoundedRectangle(cornerRadius: 20))
    }
}

struct RailButtonFace: View {
    let icon: Icon
    let label: String
    var chosen: Bool = false
    var labelled: Bool = false
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            Image(icon)
                .resizable()
                .scaledToFit()
                .padding(2)
                .frame(width: RAIL_ICON, height: RAIL_ICON)
            if labelled {
                Text(label).font(.system(size: RAIL_LABEL_SIZE, weight: .medium)).lineLimit(1)
            }
        }
        .frame(width: RAIL_ITEM, height: labelled ? RAIL_LABELLED_ITEM : RAIL_ITEM)
        .foregroundStyle(chosen ? theme.colors.onPrimaryContainer : theme.colors.onSurface)
        .background(chosen ? theme.colors.primaryContainer : Color.clear, in: RoundedRectangle(cornerRadius: 14))
        .contentShape(RoundedRectangle(cornerRadius: 14))
        .padding(.vertical, 2)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label)
        .accessibilityAddTraits(chosen ? .isSelected : [])
    }
}

struct RailButton: View {
    let icon: Icon
    let label: String
    let onClick: () -> Void
    var enabled: Bool = true
    var chosen: Bool = false
    var labelled: Bool = false
    @Environment(\.theme) private var theme
    @State private var tip = false

    var body: some View {
        RailButtonFace(icon: icon, label: label, chosen: chosen, labelled: labelled)
            .onTapGesture { if enabled { onClick() } }
            .onLongPressGesture { tip = true }
            .opacity(enabled ? 1 : DISABLED_ALPHA)
            .accessibilityAddTraits(.isButton)
            .accessibilityAction { if enabled { onClick() } }
            .popover(isPresented: $tip) {
                Text(label)
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.inverseOnSurface)
                    .padding(.horizontal, Space.s2)
                    .padding(.vertical, Space.s1)
                    .presentationCompactAdaptation(.popover)
                    .presentationBackground(theme.colors.inverseSurface)
            }
            .task(id: tip) {
                guard tip else { return }
                try? await Task.sleep(for: .milliseconds(TOOLTIP_MS))
                if !Task.isCancelled { tip = false }
            }
    }
}

struct RailDivider: View {
    var body: some View {
        Divider().frame(width: 28).padding(.vertical, 2)
    }
}
