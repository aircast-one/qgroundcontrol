import SwiftUI

extension EnvironmentValues {
    @Entry var immersive = false
}

struct AircastSheet<Content: View>: View {
    let onDismissRequest: () -> Void
    var skipPartiallyExpanded = false
    @ViewBuilder let content: () -> Content
    @Environment(\.theme) private var theme
    @Environment(\.immersive) private var immersive

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .sheet(isPresented: Binding(get: { true }, set: { shown in if !shown { onDismissRequest() } })) {
                VStack(alignment: .leading, spacing: 0) { content() }
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .presentationDetents(skipPartiallyExpanded ? [.large] : [.medium, .large])
                    .presentationDragIndicator(.visible)
                    .presentationCornerRadius(Corner.extraLarge)
                    .presentationBackground(theme.colors.surfaceContainerLow)
                    .statusBarHidden(immersive)
                    .persistentSystemOverlays(immersive ? .hidden : .automatic)
            }
    }
}
