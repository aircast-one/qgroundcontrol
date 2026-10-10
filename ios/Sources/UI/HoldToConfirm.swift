import SwiftUI

private let TRACK_HEIGHT: CGFloat = 64

private let HOLD_TRACK_FILL_ALPHA = 0.45

func holdLabel(_ name: String) -> String { name.isBlank ? "Hold to confirm" : "Hold to \(name.lowercased())" }

struct HoldToConfirm: View {
    let label: String
    var destructive: Bool = false
    var enabled: Bool = true
    let onConfirm: () -> Void
    @Environment(\.theme) private var theme
    @State private var progress = 0.0

    var body: some View {
        let accent = destructive ? theme.colors.error : theme.colors.primary
        let track = destructive ? theme.colors.errorContainer : theme.colors.primaryContainer
        let onTrack = destructive ? theme.colors.onErrorContainer : theme.colors.onPrimaryContainer
        ZStack {
            GeometryReader { geo in
                accent.opacity(HOLD_TRACK_FILL_ALPHA).frame(width: geo.size.width * progress)
            }
            Text(label).font(.labelLarge).foregroundStyle(onTrack)
        }
        .frame(maxWidth: .infinity)
        .frame(height: TRACK_HEIGHT)
        .background(track)
        .clipShape(Capsule())
        .opacity(enabled ? 1 : DISABLED_ALPHA)
        .contentShape(Capsule())
        .rememberHold(label, enabled, onTap: {}, onHold: onConfirm, label: label, progress: $progress)
    }
}
