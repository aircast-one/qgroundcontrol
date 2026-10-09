import SwiftUI

let RAIL_WIDTH: CGFloat = 64
private let RAIL_ITEM_WIDTH: CGFloat = 56
private let RAIL_ICON: CGFloat = 22
private let RAIL_ALPHA = 0.94
private let DISABLED_ALPHA = 0.38

struct PlanRail<Content: View>: View {
    @ViewBuilder let content: () -> Content
    @Environment(\.theme) private var theme
    @State private var contentHeight: CGFloat = 0

    var body: some View {
        ScrollView {
            VStack(spacing: 0) { content() }
                .frame(width: RAIL_WIDTH)
                .padding(.vertical, 4)
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
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            Image(icon)
                .resizable()
                .scaledToFit()
                .padding(2)
                .frame(width: RAIL_ICON, height: RAIL_ICON)
            Text(label).font(.labelSmall).lineLimit(1)
        }
        .frame(width: RAIL_ITEM_WIDTH)
        .padding(.vertical, 6)
        .foregroundStyle(chosen ? theme.colors.onPrimaryContainer : theme.colors.onSurface)
        .background(chosen ? theme.colors.primaryContainer : Color.clear, in: RoundedRectangle(cornerRadius: 14))
        .contentShape(RoundedRectangle(cornerRadius: 14))
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(chosen ? .isSelected : [])
    }
}

struct RailButton: View {
    let icon: Icon
    let label: String
    let onClick: () -> Void
    var enabled: Bool = true
    var chosen: Bool = false

    var body: some View {
        Button(action: onClick) {
            RailButtonFace(icon: icon, label: label, chosen: chosen)
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : DISABLED_ALPHA)
    }
}

struct RailDivider: View {
    var body: some View {
        Divider().frame(width: 32).padding(.vertical, 4)
    }
}
