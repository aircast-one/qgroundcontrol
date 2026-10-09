import SwiftUI

struct SetupDialog<Content: View, Buttons: View>: View {
    let title: String
    @ViewBuilder let content: () -> Content
    @ViewBuilder let buttons: () -> Buttons
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s4) {
            Text(title).font(.headlineSmall)
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s3) { content() }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s6)
            }
            .padding(.horizontal, -Space.s6)
            HStack(spacing: Space.s2) {
                Spacer(minLength: 0)
                buttons()
            }
            .buttonStyle(.text)
        }
        .padding(Space.s6)
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
        .presentationBackground(theme.colors.surfaceContainerLow)
    }
}
