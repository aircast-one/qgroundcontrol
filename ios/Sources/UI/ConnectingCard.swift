import SwiftUI

func connectingTitle(_ name: String?) -> String {
    name.flatMap { $0.isBlank ? nil : "Connecting to \($0)" } ?? "Connecting"
}

struct LoadingWords: Equatable {
    let title: String
    let detail: String
    let dismiss: String
}

func loadingWords(_ name: String?, lost: Bool) -> LoadingWords {
    lost
        ? LoadingWords(title: SIGNAL_LOST, detail: "Loading stopped. It picks up where it left off when the aircraft answers. \(LOST_LINK_HINT)", dismiss: "Hide")
        : LoadingWords(title: connectingTitle(name), detail: "Loading its settings.", dismiss: "Fly now, finish loading in background")
}

struct ConnectingCard: View {
    @Environment(\.theme) private var theme
    @VehicleLoading private var loading
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @QgcPath(FLY_STATE) private var flyJson
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
        let lost = flyState(flyJson)?.contactLost == true
        let words = loadingWords(vehicleChoices(vehiclesJson).active?.name, lost: lost)
        return VStack(spacing: 12) {
            Image(lost ? .warning : .link)
                .font(.system(size: 30))
                .foregroundStyle(theme.colors.onPrimaryContainer)
                .frame(width: 72, height: 72)
                .background(theme.colors.primaryContainer, in: Circle())
            Text(words.title)
                .font(.headlineSmall)
                .foregroundStyle(lost ? theme.colors.error : theme.colors.onSurface)
                .multilineTextAlignment(.center)
            Text(words.detail)
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .multilineTextAlignment(.center)
            HStack {
                Text("Parameters").font(.titleSmall)
                Spacer(minLength: 0)
                Text("\(Int(progress * 100))%")
                    .font(.labelLarge)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
            }
            ProgressView(value: Double(progress))
                .tint(lost ? theme.colors.outline : nil)
            Button(words.dismiss) { dismissed = true }
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
