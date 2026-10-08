import SwiftUI

struct OrbitReadout: View {
    @QgcPath(ORBIT_VIEW) private var view
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        if let label = orbitLabel(orbitReading(view)) {
            Text(label)
                .font(.labelLarge)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(osdBackdrop(theme.colors.secondaryContainer.opacity(0.92), flyOsd))
        }
    }
}
