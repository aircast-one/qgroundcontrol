import SwiftUI

struct RadioIndicator: View {
    let selected: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        Image(selected ? .radioChecked : .radioUnchecked)
            .foregroundStyle(selected ? theme.colors.primary : theme.colors.onSurfaceVariant)
    }
}

struct RadioChoiceRow: View {
    let label: String
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack {
                RadioIndicator(selected: selected)
                Text(label).foregroundStyle(theme.colors.onSurface)
            }
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}
