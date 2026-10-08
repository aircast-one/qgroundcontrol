import SwiftUI

let CHECKLIST = "checklist"

enum FlyDeckLayout {
    case Bottom, Rail
}

private let DISABLED_ALPHA = 0.38
private let DECK_BUTTON_HEIGHT: CGFloat = 80
private let DECK_ICON_SIZE: CGFloat = 28

struct DeckEntry {
    let id: String
    let label: String
    let icon: Icon
    let enabled: Bool
    var warning: Bool = false
    var onHold: (() -> Void)? = nil
    let onClick: () -> Void
}

let HOLD_TO_TAKE_OFF = "Hold to take off"
let TAKE_OFF = "Take off"
let DECK_HOLD_MS = 1500
private let DECK_HOLD_FILL_ALPHA = 0.3
private let LONG_PRESS_TIMEOUT_SECONDS = 0.4

func holdTakeoffHeight(_ takeoff: GuidedTakeoff?) -> Double? {
    takeoffRangeUsable(takeoff) ? takeoff?.initial : nil
}

private struct HoldPress: ViewModifier {
    let enabled: Bool
    let onTap: () -> Void
    let onHold: (() -> Void)?
    let label: String
    @Binding var progress: Double
    @State private var pressedAt: Date?
    @State private var cancelled = false
    @State private var fill: Task<Void, Never>?
    @State private var fired = false
    @State private var held = 0
    @State private var size = CGSize.zero

    private func inside(_ point: CGPoint) -> Bool { CGRect(origin: .zero, size: size).contains(point) }

    private func press(_ onHold: @escaping () -> Void) {
        pressedAt = Date()
        fired = false
        withAnimation(.linear(duration: Double(DECK_HOLD_MS) / 1000)) { progress = 1 }
        fill = Task { @MainActor in
            guard (try? await Task.sleep(for: .milliseconds(DECK_HOLD_MS))) != nil else { return }
            fired = true
            held += 1
            onHold()
        }
    }

    private func release() {
        fill?.cancel()
        fill = nil
        pressedAt = nil
        var instant = Transaction()
        instant.disablesAnimations = true
        withTransaction(instant) { progress = 0 }
    }

    func body(content: Content) -> some View {
        if let onHold, enabled {
            content
                .contentShape(Rectangle())
                .onGeometryChange(for: CGSize.self) { $0.size } action: { size = $0 }
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { drag in
                            guard !cancelled else { return }
                            if !inside(drag.location) {
                                cancelled = true
                                release()
                            } else if pressedAt == nil {
                                press(onHold)
                            }
                        }
                        .onEnded { drag in
                            let quick = pressedAt.map { Date().timeIntervalSince($0) < LONG_PRESS_TIMEOUT_SECONDS } ?? false
                            let tap = !cancelled && !fired && quick && inside(drag.location)
                            cancelled = false
                            release()
                            if tap { onTap() }
                        }
                )
                .onDisappear(perform: release)
                .sensoryFeedback(.impact(weight: .heavy), trigger: held)
                .accessibilityElement(children: .combine)
                .accessibilityAddTraits(.isButton)
                .accessibilityAction { onTap() }
                .accessibilityAction(named: Text(label)) { onHold() }
        } else {
            content
                .contentShape(Rectangle())
                .onTapGesture { if enabled { onTap() } }
                .accessibilityElement(children: .combine)
                .accessibilityAddTraits(.isButton)
                .accessibilityAction { if enabled { onTap() } }
        }
    }
}

extension View {
    func rememberHold(_ key: String, _ enabled: Bool, onTap: @escaping () -> Void, onHold: (() -> Void)?, label: String, progress: Binding<Double>) -> some View {
        modifier(HoldPress(enabled: enabled, onTap: onTap, onHold: onHold, label: label, progress: progress)).id(key)
    }
}

struct HoldFill: View {
    let progress: Double
    let color: Color

    var body: some View {
        GeometryReader { geometry in
            if progress > 0 {
                color.frame(width: geometry.size.width * progress, height: geometry.size.height)
            }
        }
        .allowsHitTesting(false)
    }
}

private struct DeckPress<Content: View>: View {
    let entry: DeckEntry
    let fill: Color
    @ViewBuilder let content: () -> Content
    @State private var progress = 0.0

    var body: some View {
        ZStack {
            HoldFill(progress: progress, color: fill.opacity(DECK_HOLD_FILL_ALPHA))
            content()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .rememberHold(entry.id, entry.enabled, onTap: entry.onClick, onHold: entry.onHold, label: entry.label, progress: $progress)
    }
}

func deckIds(_ shown: Set<String>, _ armed: Bool) -> [(String, Bool)] {
    let flying = armed && (shown.contains("rtl") || shown.contains("land"))
    let ground = [CHECKLIST, shown.contains("takeoff") ? "takeoff" : "arm"]
    let chosen = (flying ? [PAUSE, "rtl", "land"] : ground).filter { shown.contains($0) }
    let primary = flying ? (chosen.contains("rtl") ? "rtl" : nil) : chosen.last
    return chosen.map { ($0, $0 == primary) }
}

struct DeckButton: View {
    let entry: DeckEntry
    let primary: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        let content = primary ? theme.colors.onPrimaryContainer : theme.colors.onSurface
        DeckPress(entry: entry, fill: content) {
            VStack(spacing: Space.s1) {
                Image(entry.icon)
                    .font(.system(size: 22))
                    .frame(width: DECK_ICON_SIZE, height: DECK_ICON_SIZE)
                Text(entry.label).font(.labelLarge).lineLimit(1)
            }
        }
        .foregroundStyle(content)
        .frame(height: DECK_BUTTON_HEIGHT)
        .background(primary ? theme.colors.primaryContainer : theme.colors.surfaceContainerHigh)
        .clipShape(RoundedRectangle(cornerRadius: Corner.large))
        .opacity(entry.enabled ? 1 : DISABLED_ALPHA)
    }
}

private let RAIL_BUTTON_SIZE: CGFloat = 48
private let RAIL_ICON_SIZE: CGFloat = 24
private let RAIL_PRIMARY_ICON_SIZE: CGFloat = 32

func railLabel(_ label: String) -> String { label.removingPrefix("Hold to ").capitalizedFirst }

struct RailDeckButton: View {
    let entry: DeckEntry
    let primary: Bool
    var labelled: Bool = false
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            DeckPress(entry: entry, fill: theme.aircast.outdoorForeground) {
                Image(entry.icon)
                    .font(.system(size: (primary ? RAIL_PRIMARY_ICON_SIZE : RAIL_ICON_SIZE) * 0.8))
                    .frame(width: primary ? RAIL_PRIMARY_ICON_SIZE : RAIL_ICON_SIZE, height: primary ? RAIL_PRIMARY_ICON_SIZE : RAIL_ICON_SIZE)
                    .osdShadow()
            }
            .foregroundStyle(theme.aircast.outdoorForeground)
            .frame(width: RAIL_BUTTON_SIZE, height: RAIL_BUTTON_SIZE)
            .clipShape(Circle())
            .opacity(entry.enabled ? 1 : DISABLED_ALPHA)
            .accessibilityLabel(entry.label)
            if labelled {
                Text(railLabel(entry.label))
                    .font(.labelSmall)
                    .foregroundStyle(theme.aircast.outdoorForeground)
                    .lineLimit(1)
                    .osdShadow()
            }
        }
    }
}

private let MORE_COLUMNS = 4
private let MORE_TILE_HEIGHT: CGFloat = 80

struct MoreTile {
    let label: String
    let icon: Icon
    let enabled: Bool
    var warning: Bool = false
    let onClick: () -> Void
}

func guidedIcon(_ id: String) -> Icon {
    switch id {
    case "startMission", "continueMission", "resumeMission": .route
    case "cancelRoi": .close
    case PAUSE: .pause
    case "landAbort": .flightTakeoff
    case "release": .download
    case "grab": .upload
    case "hold": .stopCircle
    case "vtolTransitionToFixedWing", "vtolTransitionToMultiRotor": .swapHoriz
    case "forceArm": .bolt
    default: .send
    }
}

struct MoreActionsSheet<Footer: View>: View {
    let tiles: [MoreTile]
    let onDismiss: () -> Void
    @ViewBuilder let footer: () -> Footer
    @Environment(\.theme) private var theme

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s3) {
                    VStack(alignment: .leading, spacing: 0) {
                        Text("More actions").font(.titleLarge)
                        Text("Everything that changes what the drone does")
                            .font(.bodyMedium)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    .padding(.horizontal, Space.s2)
                    ForEach(Array(stride(from: 0, to: tiles.count, by: MORE_COLUMNS)), id: \.self) { start in
                        let row = Array(tiles[start..<min(start + MORE_COLUMNS, tiles.count)])
                        HStack(spacing: Space.s3) {
                            ForEach(row.indices, id: \.self) { at in tile(row[at]) }
                            ForEach(0..<(MORE_COLUMNS - row.count), id: \.self) { _ in Color.clear.frame(maxWidth: .infinity, maxHeight: 1) }
                        }
                    }
                    footer()
                }
                .padding(.leading, Space.s4)
                .padding(.trailing, Space.s4)
                .padding(.bottom, Space.s6)
            }
        }
    }

    private func tile(_ tile: MoreTile) -> some View {
        Button {
            onDismiss()
            tile.onClick()
        } label: {
            VStack(spacing: Space.s2) {
                Image(tile.icon).font(.system(size: 20)).frame(width: 24, height: 24)
                Text(sentenceCase(tile.label))
                    .font(.labelMedium)
                    .lineLimit(2)
                    .multilineTextAlignment(.center)
            }
            .padding(.horizontal, Space.s1)
            .frame(maxWidth: .infinity)
            .frame(height: MORE_TILE_HEIGHT)
            .foregroundStyle(tile.warning ? theme.colors.error : theme.colors.onSurface)
            .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.large))
            .opacity(tile.enabled ? 1 : DISABLED_ALPHA)
        }
        .buttonStyle(.plain)
        .disabled(!tile.enabled)
    }
}
