import SwiftUI

func connectingTitle(_ name: String?) -> String {
    name.flatMap { $0.isBlank ? nil : "Connecting to \($0)" } ?? "Connecting"
}

struct ConnectingCard: View {
    @Environment(\.theme) private var theme
    @VehicleLoading private var loading
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @State private var dismissed = false

    var body: some View {
        ZStack(alignment: .topLeading) {
            if let progress = loading, !dismissed {
                card(progress)
            }
        }
        .onChange(of: loading == nil) { _, done in if done { dismissed = false } }
    }

    private func card(_ progress: Float) -> some View {
        VStack(spacing: 12) {
            Image(.link)
                .font(.system(size: 30))
                .foregroundStyle(theme.colors.onPrimaryContainer)
                .frame(width: 72, height: 72)
                .background(theme.colors.primaryContainer, in: Circle())
            Text(connectingTitle(vehicleChoices(vehiclesJson).active?.name))
                .font(.headlineSmall)
                .multilineTextAlignment(.center)
            Text("Loading its settings.")
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
            HStack {
                Text("Parameters").font(.titleSmall)
                Spacer(minLength: 0)
                Text("\(Int(progress * 100))%")
                    .font(.labelLarge)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
            }
            ProgressView(value: Double(progress))
            Button("Fly now, finish loading in background") { dismissed = true }
                .buttonStyle(.text)
        }
        .padding(24)
        .frame(maxWidth: .infinity)
        .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.extraLarge))
        .padding(16)
    }
}

let LOOKING_TITLE = "Looking for your aircraft"
let LOOKING_HINT = "Turn on the aircraft. For Wi-Fi or a network connection, add a link."
private let CONNECTION_SETTINGS = "Connections"
private let LOOKING_MAX_WIDTH: CGFloat = 360

struct LookingForAircraft: View {
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: 10) {
            ProgressView()
                .tint(theme.aircast.outdoorForeground)
                .frame(width: 28, height: 28)
            Text(LOOKING_TITLE)
                .font(.titleMedium)
                .foregroundStyle(theme.aircast.outdoorForeground)
                .multilineTextAlignment(.center)
            Text(LOOKING_HINT)
                .font(.bodyMedium)
                .foregroundStyle(theme.aircast.outdoorForeground)
                .multilineTextAlignment(.center)
            Button("Add a link") { navigation.settingsPage = CONNECTION_SETTINGS }
                .buttonStyle(.text)
        }
        .frame(maxWidth: LOOKING_MAX_WIDTH)
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
