import SwiftUI
#if os(macOS)
import AppKit
#endif

// Control sizes for the two ways the shared app is driven: a finger on iOS, a pointer on the Mac. The touch sizes
// are the ones the iOS app has always used; the desktop ones follow the Mac's denser control metrics. Controls
// whose mistaken press is costly (hold-to-confirm, emergency stop, arm and take-off) keep their own large sizes
// on both platforms and do not read these.
struct ControlMetrics: Equatable {
    // A row with a leading badge or several lines of text.
    let rowLarge: CGFloat
    // A page or sheet top bar, or a two-line row.
    let rowTall: CGFloat
    // A single-line list row, search field or setting row.
    let row: CGFloat
    // A compact row: menu entries, disclosure toggles, inline choices.
    let rowCompact: CGFloat
    // A dense row: a text-button list in a dialog.
    let rowDense: CGFloat
    // A floating action button that is not a hold or slide.
    let floatingButton: CGFloat
    // A full-width filled action button at the foot of a page.
    let primaryButton: CGFloat
    // An icon-only button: back, close, search, copy, remove.
    let iconButton: CGFloat
    // A smaller icon-only button or step button.
    let iconButtonSmall: CGFloat
    // A round leading badge on a row.
    let avatar: CGFloat
    // A press-and-repeat value step button.
    let stepper: CGFloat
    // The smallest hit area a text button gets.
    let minTarget: CGFloat

    static let touch = ControlMetrics(
        rowLarge: 72, rowTall: 64, row: 56, rowCompact: 48, rowDense: 40,
        floatingButton: 56, primaryButton: 52, iconButton: 48, iconButtonSmall: 40, avatar: 40, stepper: 48, minTarget: 48
    )

    static let desktop = ControlMetrics(
        rowLarge: 52, rowTall: 44, row: 36, rowCompact: 32, rowDense: 28,
        floatingButton: 36, primaryButton: 32, iconButton: 28, iconButtonSmall: 28, avatar: 28, stepper: 32, minTarget: 28
    )
}

enum Metrics {
    #if os(macOS)
    static let current = ControlMetrics.desktop
    #else
    static let current = ControlMetrics.touch
    #endif

    static var rowLarge: CGFloat { current.rowLarge }
    static var rowTall: CGFloat { current.rowTall }
    static var row: CGFloat { current.row }
    static var rowCompact: CGFloat { current.rowCompact }
    static var rowDense: CGFloat { current.rowDense }
    static var floatingButton: CGFloat { current.floatingButton }
    static var primaryButton: CGFloat { current.primaryButton }
    static var iconButton: CGFloat { current.iconButton }
    static var iconButtonSmall: CGFloat { current.iconButtonSmall }
    static var avatar: CGFloat { current.avatar }
    static var stepper: CGFloat { current.stepper }
    static var minTarget: CGFloat { current.minTarget }
}

enum HoverCursor: Equatable {
    case push, pop, keep
}

// Keeps the pointing-hand cursor pushed exactly while the pointer is over an enabled control, so every push is
// matched by one pop however hover and enablement change.
func hoverCursor(hovering: Bool, enabled: Bool, pushed: Bool) -> HoverCursor {
    let wanted = hovering && enabled
    return wanted == pushed ? .keep : wanted ? .push : .pop
}

#if os(macOS)
private let HOVER_ALPHA = 0.08

private struct MacHover<S: Shape>: ViewModifier {
    let shape: S
    let tint: Color?
    let enabled: Bool
    let cursor: Bool
    @Environment(\.theme) private var theme
    @Environment(\.isEnabled) private var isEnabled
    @State private var hovering = false
    @State private var pushed = false

    private var live: Bool { enabled && isEnabled }

    func body(content: Content) -> some View {
        content
            .overlay {
                shape
                    .fill((tint ?? theme.colors.onSurface).opacity(hovering && live ? HOVER_ALPHA : 0))
                    .allowsHitTesting(false)
            }
            .onHover { inside in
                hovering = inside
                updateCursor()
            }
            .onChange(of: live) { updateCursor() }
            .onDisappear {
                hovering = false
                updateCursor()
            }
    }

    private func updateCursor() {
        guard cursor else { return }
        switch hoverCursor(hovering: hovering, enabled: live, pushed: pushed) {
        case .push:
            NSCursor.pointingHand.push()
            pushed = true
        case .pop:
            NSCursor.pop()
            pushed = false
        case .keep:
            break
        }
    }
}
#endif

extension View {
    // A Material hover state layer on the Mac: the shape tints while the pointer is over the control, and with
    // `cursor` the pointer becomes a pointing hand. A disabled control (`.disabled`, or `enabled` false for one
    // driven by gestures) shows neither. Nothing on iOS.
    func macHover<S: Shape>(_ shape: S, tint: Color? = nil, enabled: Bool = true, cursor: Bool = false) -> some View {
        #if os(macOS)
        return modifier(MacHover(shape: shape, tint: tint, enabled: enabled, cursor: cursor))
        #else
        return self
        #endif
    }

    func macHover(tint: Color? = nil, enabled: Bool = true, cursor: Bool = false) -> some View {
        macHover(Rectangle(), tint: tint, enabled: enabled, cursor: cursor)
    }

    // The Mac's hover tooltip for a control that shows no text of its own. Left off iOS, where help text would
    // also be read out as an accessibility hint.
    func macTooltip(_ text: String) -> some View {
        #if os(macOS)
        return help(text)
        #else
        return self
        #endif
    }
}
