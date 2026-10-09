import SwiftUI

private let OBSTACLE = "view.obstacle"

struct ObstacleReadout: View {
    @QgcPath(OBSTACLE) private var view
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        if let warning = obstacleWarning(view) {
            Text(warning.label)
                .font(.labelLarge)
                .foregroundStyle(flyOsd ? AnyShapeStyle(HierarchicalShapeStyle.primary) : AnyShapeStyle(warning.close ? theme.colors.onErrorContainer : theme.colors.onSurfaceVariant))
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(osdBackdrop(warning.close ? theme.colors.errorContainer : theme.colors.surfaceVariant, flyOsd))
                .contentShape(Rectangle())
        }
    }
}
