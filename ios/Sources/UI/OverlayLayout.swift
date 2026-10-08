import SwiftUI

private let LAYOUT_STORE = "fly-overlay-layout"
private let HIDDEN_PREFIX = "OverlayRigHidden-"
private let OFFSET_PREFIX = "OverlayRigOffset-"
private let LANDSCAPE_SUFFIX = "@landscape"
private let HIDDEN_ALPHA = 0.35
private let JIGGLE_DEGREES = 1.2
private let JIGGLE_MILLIS = 120
private let JIGGLE_SPREAD = 0x1f
private let BADGE_SIZE: CGFloat = 24
private let HOLD_TO_EDIT_SECONDS = 0.5
let RESET_ARM_MILLIS: Int64 = 4000

struct ResetTap: Equatable {
    let reset: Bool
    let armed: Bool
}

func resetTap(_ armed: Bool) -> ResetTap { ResetTap(reset: armed, armed: !armed) }

func resetPillText(_ armed: Bool) -> String { armed ? "Tap again to reset" : "Reset layout" }

private let INDICATOR_ORDER = "FlyViewIndicatorOrder"

@Observable
final class OverlayLayoutState {
    @ObservationIgnored private let prefs: UserDefaults
    var editing = false
    private(set) var locked = false
    private(set) var hidden: Set<String>
    private(set) var indicatorOrder: [String]
    var valueSize = ValueSize.Default
    private(set) var offsets: [String: CGSize]

    init(_ prefs: UserDefaults) {
        self.prefs = prefs
        let stored = prefs.persistentDomain(forName: LAYOUT_STORE) ?? [:]
        hidden = hiddenKeys(stored)
        indicatorOrder = storedIndicatorOrder(stored)
        offsets = storedOffsets(stored)
    }

    func startEditing() {
        if !locked { editing = true }
    }

    func lockWhileArmed(_ armed: Bool) {
        locked = armed
        if armed { editing = false }
    }

    func setHidden(_ key: String, _ hide: Bool) {
        prefs.set(hide, forKey: HIDDEN_PREFIX + key)
        hidden = withHidden(hidden, key, hide)
    }

    func saveIndicatorOrder(_ keys: [String]) {
        prefs.set(keys.joined(separator: ","), forKey: INDICATOR_ORDER)
        indicatorOrder = keys
    }

    func nudge(_ key: String, _ dx: CGFloat, _ dy: CGFloat) {
        let current = offsets[key] ?? .zero
        offsets[key] = CGSize(width: current.width + dx, height: current.height + dy)
    }

    func saveOffset(_ key: String) {
        guard let moved = offsets[key] else { return }
        prefs.set("\(Double(moved.width)),\(Double(moved.height))", forKey: OFFSET_PREFIX + key)
    }

    func reset() {
        prefs.removePersistentDomain(forName: LAYOUT_STORE)
        hidden = []
        indicatorOrder = []
        offsets = [:]
    }
}

func overlayLayoutStore() -> UserDefaults { UserDefaults(suiteName: LAYOUT_STORE) ?? .standard }

func storedIndicatorOrder(_ stored: [String: Any]) -> [String] {
    ((stored[INDICATOR_ORDER] as? String) ?? "").components(separatedBy: ",").filter { !$0.isBlank }
}

func onScreenCorrection(_ bounds: CGRect, _ root: CGSize, _ offset: CGSize) -> CGSize {
    let pull = clampedDrag(bounds.minX, bounds.minY, bounds.maxX, bounds.maxY, root.width, root.height, 0, 0)
    return CGSize(
        width: pulledBack(bounds.width > root.width ? 0 : pull.width, offset.width),
        height: pulledBack(bounds.height > root.height ? 0 : pull.height, offset.height)
    )
}

private func coerceIn(_ value: CGFloat, _ low: CGFloat, _ high: CGFloat) -> CGFloat { min(max(value, low), high) }

func pulledBack(_ correction: CGFloat, _ offset: CGFloat) -> CGFloat {
    if offset > 0 { return coerceIn(correction, -offset, 0) }
    if offset < 0 { return coerceIn(correction, 0, -offset) }
    return 0
}

func orientedKey(_ key: String, _ landscape: Bool) -> String { landscape ? key + LANDSCAPE_SUFFIX : key }

func orderedKeys(_ available: [String], _ order: [String]) -> [String] {
    order.filter { available.contains($0) } + available.filter { !order.contains($0) }
}

func movedKey(_ keys: [String], _ key: String, _ delta: Int) -> [String]? {
    guard let from = keys.firstIndex(of: key), keys.indices.contains(from + delta) else { return nil }
    let to = from + delta
    let rest = keys.enumerated().filter { $0.offset != from }.map(\.element)
    return Array(rest.prefix(to)) + [key] + Array(rest.dropFirst(to))
}

func hiddenKeys(_ stored: [String: Any]) -> Set<String> {
    Set(stored.filter { key, value in key.hasPrefix(HIDDEN_PREFIX) && (value as? Bool) == true }.keys.map { $0.removingPrefix(HIDDEN_PREFIX) })
}

func withHidden(_ hidden: Set<String>, _ key: String, _ hide: Bool) -> Set<String> {
    hide ? hidden.union([key]) : hidden.subtracting([key])
}

func storedOffsets(_ stored: [String: Any]) -> [String: CGSize] {
    Dictionary(uniqueKeysWithValues: stored.filter { $0.key.hasPrefix(OFFSET_PREFIX) }.compactMap { key, value in
        ((value as? String)?.components(separatedBy: ",").compactMap { Double($0) }).flatMap { parts in
            parts.count == 2 ? (key.removingPrefix(OFFSET_PREFIX), CGSize(width: parts[0], height: parts[1])) : nil
        }
    })
}

func clampedDrag(_ left: CGFloat, _ top: CGFloat, _ right: CGFloat, _ bottom: CGFloat, _ width: CGFloat, _ height: CGFloat, _ dx: CGFloat, _ dy: CGFloat) -> CGSize {
    CGSize(
        width: coerceIn(dx, -left, max(width - right, -left)),
        height: coerceIn(dy, -top, max(height - bottom, -top))
    )
}

private func jiggleSpread(_ key: String) -> Int {
    key.unicodeScalars.reduce(0) { ($0 &* 31 &+ Int($1.value)) & 0xffff } & JIGGLE_SPREAD
}

private struct Jiggle: ViewModifier {
    let key: String
    let active: Bool
    @State private var swing = false

    func body(content: Content) -> some View {
        content
            .rotationEffect(.degrees(active ? (swing ? JIGGLE_DEGREES : -JIGGLE_DEGREES) : 0))
            .onChange(of: active, initial: true) { _, now in
                if now {
                    withAnimation(.easeInOut(duration: Double(JIGGLE_MILLIS + jiggleSpread(key)) / 1000).repeatForever(autoreverses: true)) { swing = true }
                } else {
                    withAnimation(.easeOut(duration: 0.1)) { swing = false }
                }
            }
    }
}

private struct KeptOnScreen: ViewModifier {
    let key: String
    let active: Bool
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.LocalRootSize) private var root

    func body(content: Content) -> some View {
        content.onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { bounds in
            guard active, root.width > 0, root.height > 0 else { return }
            let layout = flyScreen.layout
            let correction = onScreenCorrection(bounds, root, layout.offsets[key] ?? .zero)
            guard correction != .zero else { return }
            layout.nudge(key, correction.width, correction.height)
            layout.saveOffset(key)
        }
    }
}

private struct HoldToEditLayout: ViewModifier {
    @Environment(FlyScreenState.self) private var flyScreen

    func body(content: Content) -> some View {
        let layout = flyScreen.layout
        content.simultaneousGesture(
            LongPressGesture(minimumDuration: HOLD_TO_EDIT_SECONDS).onEnded { _ in layout.startEditing() },
            including: layout.editing || layout.locked ? .subviews : .all
        )
    }
}

extension View {
    func holdToEditLayout() -> some View { modifier(HoldToEditLayout()) }

    func layoutPlacement(_ key: String, keepOnScreen: Bool, active: Bool = true) -> some View {
        modifier(LayoutPlacement(key: key, keepOnScreen: keepOnScreen, active: active))
    }

    func osdShadow(_ on: Bool) -> some View {
        shadow(color: on ? .black : .clear, radius: on ? 3 : 0, x: 0, y: on ? 1 : 0)
            .shadow(color: on ? .black : .clear, radius: on ? 3 : 0, x: 0, y: on ? 1 : 0)
    }
}

private struct LayoutPlacement: ViewModifier {
    let key: String
    let keepOnScreen: Bool
    let active: Bool
    @Environment(FlyScreenState.self) private var flyScreen
    @FlyIsPortrait private var portrait

    func body(content: Content) -> some View {
        let layout = flyScreen.layout
        let placedKey = orientedKey(key, !portrait)
        content
            .modifier(Jiggle(key: key, active: active && layout.editing))
            .modifier(KeptOnScreen(key: placedKey, active: active && keepOnScreen))
            .offset(active ? layout.offsets[placedKey] ?? .zero : .zero)
    }
}

struct LayoutDragArea: View {
    let key: String
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.LocalRootSize) private var root
    @FlyIsPortrait private var portrait
    @State private var bounds = CGRect.zero
    @State private var dragged = CGSize.zero

    var body: some View {
        let layout = flyScreen.layout
        let placedKey = orientedKey(key, !portrait)
        Color.clear
            .contentShape(Rectangle())
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { bounds = $0 }
            .gesture(
                DragGesture(coordinateSpace: .global)
                    .onChanged { drag in
                        let step = CGSize(width: drag.translation.width - dragged.width, height: drag.translation.height - dragged.height)
                        dragged = drag.translation
                        let allowed = clampedDrag(bounds.minX, bounds.minY, bounds.maxX, bounds.maxY, root.width, root.height, step.width, step.height)
                        layout.nudge(placedKey, allowed.width, allowed.height)
                    }
                    .onEnded { _ in
                        dragged = .zero
                        layout.saveOffset(placedKey)
                    }
            )
    }
}

struct LayoutPipEditor: View {
    let key: String
    let shape: AnyShape
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.theme) private var theme

    var body: some View {
        if flyScreen.layout.editing {
            LayoutDragArea(key: key)
                .overlay(shape.stroke(theme.colors.primary, lineWidth: 2))
        }
    }
}

struct LayoutWidget<Content: View>: View {
    let key: String
    var movable: Bool = true
    var hideable: Bool = true
    @ViewBuilder let content: () -> Content
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.LocalAvoidedByVideoMessage) private var avoided
    @Environment(\.flyOsd) private var flyOsd
    @Environment(\.theme) private var theme
    @FlyIsPortrait private var portrait
    @State private var shown = false

    var body: some View {
        let layout = flyScreen.layout
        let editing = layout.editing
        let hidden = layout.hidden.contains(key)
        if !hidden || editing {
            let placedKey = orientedKey(key, !portrait)
            let decorated = editing && shown
            content()
                .onGeometryChange(for: Bool.self) { $0.size.width > 0 && $0.size.height > 0 } action: { shown = $0 }
                .overlay {
                    if decorated {
                        RoundedRectangle(cornerRadius: Corner.medium).stroke(theme.colors.outline, lineWidth: 1)
                    }
                }
                .opacity(hidden ? HIDDEN_ALPHA : 1)
                .osdShadow(flyOsd)
                .overlay {
                    if decorated && movable { LayoutDragArea(key: key) }
                }
                .overlay(alignment: .topTrailing) {
                    if decorated { badge(layout, hidden) }
                }
                .modifier(Jiggle(key: key, active: decorated))
                .holdToEditLayout()
                .modifier(KeptOnScreen(key: placedKey, active: movable))
                .offset(movable ? layout.offsets[placedKey] ?? .zero : .zero)
                .modifier(AvoidedByVideoMessage(key: key, active: avoided))
        }
    }

    @ViewBuilder
    private func badge(_ layout: OverlayLayoutState, _ hidden: Bool) -> some View {
        if hideable {
            Button { layout.setHidden(key, !hidden) } label: {
                Image(hidden ? .add : .close)
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(hidden ? theme.colors.onPrimary : theme.colors.inverseOnSurface)
                    .frame(width: BADGE_SIZE, height: BADGE_SIZE)
                    .background(hidden ? theme.colors.primary : theme.colors.inverseSurface, in: Circle())
                    .shadow(radius: 2)
            }
            .buttonStyle(.plain)
            .accessibilityLabel(hidden ? "Show" : "Hide")
        } else {
            Image(.lock)
                .font(.system(size: 11, weight: .bold))
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .frame(width: BADGE_SIZE, height: BADGE_SIZE)
                .background(theme.colors.surfaceContainerHighest, in: Circle())
                .shadow(radius: 2)
                .accessibilityLabel("Always shown")
        }
    }
}

struct OverlayEditBar: View {
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.theme) private var theme
    @QgcPath(INSTRUMENTS_VIEW) private var classView
    @State private var armed = false

    var body: some View {
        let layout = flyScreen.layout
        if layout.editing {
            HStack(spacing: Space.s2) {
                Button(valueSizePillText(layout.valueSize)) {
                    layout.valueSize = nextValueSize(layout.valueSize)
                    writeValueSize(instrumentVehicleClass(classView), layout.valueSize)
                }
                .buttonStyle(.borderless)
                Button(resetPillText(armed)) {
                    let tap = resetTap(armed)
                    if tap.reset { layout.reset() }
                    armed = tap.armed
                }
                .buttonStyle(.borderless)
                .foregroundStyle(armed ? theme.colors.error : theme.colors.primary)
                Spacer(minLength: 0)
                Button("Done") { layout.editing = false }
                    .buttonStyle(.borderedProminent)
            }
            .font(.labelLarge)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .frame(maxWidth: .infinity)
            .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.large))
            .shadow(color: .black.opacity(0.2), radius: 3, y: 1)
            .task(id: armed) {
                guard armed, (try? await Task.sleep(for: .milliseconds(RESET_ARM_MILLIS))) != nil else { return }
                armed = false
            }
        }
    }
}
