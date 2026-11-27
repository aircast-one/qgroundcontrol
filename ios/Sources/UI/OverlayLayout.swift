import SwiftUI

private let LAYOUT_STORE = "fly-overlay-layout"
private let HIDDEN_PREFIX = "OverlayRigHidden-"
private let OFFSET_PREFIX = "OverlayRigOffset-"
private let SIZE_PREFIX = "OverlayRigSize-"
private let LANDSCAPE_SUFFIX = "@landscape"
private let HIDDEN_ALPHA = 0.35
private let JIGGLE_DEGREES = 1.2
private let JIGGLE_MILLIS = 120
private let JIGGLE_SPREAD = 0x1f
private let JIGGLE_REACH: CGFloat = 4
let BADGE_SIZE: CGFloat = 24

enum BadgeSpot {
    case inside, aboveTopTrailing, onTopLeading

    var alignment: Alignment { self == .onTopLeading ? .topLeading : .topTrailing }

    var offset: CGSize {
        switch self {
        case .inside: .zero
        case .aboveTopTrailing: CGSize(width: BADGE_SIZE / 2, height: -BADGE_SIZE * 3 / 4)
        case .onTopLeading: CGSize(width: -BADGE_SIZE / 2, height: -BADGE_SIZE / 2)
        }
    }
}
private let HOLD_TO_EDIT_SECONDS = 0.5
let RESET_ARM_MILLIS: Int64 = 4000

struct ResetTap: Equatable {
    let reset: Bool
    let armed: Bool
}

func resetTap(_ armed: Bool) -> ResetTap { ResetTap(reset: armed, armed: !armed) }

func resetPillText(_ armed: Bool) -> String { armed ? "Tap again to reset" : "Reset positions and hidden items" }

let LAYOUT_EDITING_IN_FLIGHT = "Editing the layout. Return, Land and Kill motors still work."

let MINIMAP_KEY = "minimap"

let HEADING_TAPE_KEY = "headingTape"

let MINIMAP_NAME = "Mini map"

let ATTITUDE_KEY = "attitude"

let ATTITUDE_NAME = "Attitude and compass"

let BATTERY_BAR_KEY = "batteryBar"

func batteryBarPillText(_ hidden: Bool) -> String { "Battery bar: \(hidden ? "Hidden" : "Shown")" }

func badgeLabel(_ hidden: Bool, _ name: String?) -> String { [hidden ? "Show" : "Hide", name?.lowercased()].compactMap { $0 }.joined(separator: " ") }

private let HIDDEN_BY_DEFAULT = [MINIMAP_KEY, ATTITUDE_KEY]

private let INDICATOR_ORDER = "FlyViewIndicatorOrder"

@Observable
final class OverlayLayoutState {
    @ObservationIgnored private let prefs: UserDefaults
    var editing = false
    private var gesturesHeld = 0
    private(set) var hidden: Set<String>
    private(set) var indicatorOrder: [String]
    var valueSize = ValueSize.Default
    private(set) var offsets: [String: CGSize]
    private(set) var sizes: [String: Double]

    init(_ prefs: UserDefaults) {
        self.prefs = prefs
        let stored = prefs.persistentDomain(forName: LAYOUT_STORE) ?? [:]
        hidden = hiddenKeys(stored)
        indicatorOrder = storedIndicatorOrder(stored)
        offsets = storedOffsets(stored)
        sizes = storedSizes(stored)
    }

    func resize(_ key: String, _ size: Double) {
        prefs.set(String(size), forKey: SIZE_PREFIX + key)
        sizes[key] = size
    }

    func startEditing() {
        editing = true
    }

    var gestureHeld: Bool { gesturesHeld > 0 }

    func holdGesture(_ held: Bool) {
        gesturesHeld = max(gesturesHeld + (held ? 1 : -1), 0)
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
        hidden = hiddenKeys([:])
        indicatorOrder = []
        offsets = [:]
        sizes = [:]
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

func pulledBack(_ correction: CGFloat, _ offset: CGFloat) -> CGFloat {
    if offset > 0 { return correction.clamped(to: -offset...0) }
    if offset < 0 { return correction.clamped(to: 0...(-offset)) }
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

func reorderedAmongShown(_ keys: [String], _ shownOrder: [String]) -> [String] {
    let slots = keys.indices.filter { shownOrder.contains(keys[$0]) }
    let placed = Dictionary(uniqueKeysWithValues: zip(slots, shownOrder))
    return keys.indices.map { placed[$0] ?? keys[$0] }
}

func movedAmongShown(_ keys: [String], _ centres: [String: CGFloat], _ key: String, _ delta: Int) -> [String]? {
    movedKey(keys.filter { centres[$0] != nil }, key, delta).map { reorderedAmongShown(keys, $0) }
}

func droppedOrder(_ keys: [String], _ centres: [String: CGFloat], _ key: String, _ dx: CGFloat) -> [String] {
    let at = { (shown: String) in (centres[shown] ?? 0) + (shown == key ? dx : 0) }
    return reorderedAmongShown(keys, keys.filter { centres[$0] != nil }.sorted { at($0) < at($1) })
}

func hiddenKeys(_ stored: [String: Any]) -> Set<String> {
    Set(HIDDEN_BY_DEFAULT.filter { (stored[HIDDEN_PREFIX + $0] as? Bool) != false })
        .union(stored.filter { key, value in key.hasPrefix(HIDDEN_PREFIX) && (value as? Bool) == true }.keys.map { $0.removingPrefix(HIDDEN_PREFIX) })
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

let ITEM_SIZES = [0.7, 0.85, 1, 1.25, 1.5, 2]

func largerItemSize(_ size: Double) -> Double? { ITEM_SIZES.first { $0 > size } }

func smallerItemSize(_ size: Double) -> Double? { ITEM_SIZES.last { $0 < size } }

func storedSizes(_ stored: [String: Any]) -> [String: Double] {
    Dictionary(uniqueKeysWithValues: stored.filter { $0.key.hasPrefix(SIZE_PREFIX) }.compactMap { key, value in
        (value as? String).flatMap(Double.init).flatMap { size in ITEM_SIZES.contains(size) ? (key.removingPrefix(SIZE_PREFIX), size) : nil }
    })
}

private struct ScaledLayout: Layout {
    let scale: CGFloat

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let size = subviews.first?.sizeThatFits(ProposedViewSize(width: proposal.width.map { $0 / scale }, height: proposal.height.map { $0 / scale })) ?? .zero
        return CGSize(width: size.width * scale, height: size.height * scale)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        subviews.first?.place(
            at: CGPoint(x: bounds.midX, y: bounds.midY),
            anchor: .center,
            proposal: ProposedViewSize(width: bounds.width / scale, height: bounds.height / scale)
        )
    }
}

func clampedDrag(_ left: CGFloat, _ top: CGFloat, _ right: CGFloat, _ bottom: CGFloat, _ width: CGFloat, _ height: CGFloat, _ dx: CGFloat, _ dy: CGFloat) -> CGSize {
    CGSize(
        width: dx.clamped(to: -left...max(width - right, -left)),
        height: dy.clamped(to: -top...max(height - bottom, -top))
    )
}

private func jiggleSpread(_ key: String) -> Int {
    key.unicodeScalars.reduce(0) { ($0 &* 31 &+ Int($1.value)) & 0xffff } & JIGGLE_SPREAD
}

func jiggleAngle(_ seconds: Double, _ key: String) -> Double {
    JIGGLE_DEGREES * sin(seconds * .pi * 1000 / Double(JIGGLE_MILLIS + jiggleSpread(key)))
}

func jiggleDegrees(_ width: CGFloat, _ height: CGFloat) -> Double {
    let reach = hypot(width, height) / 2
    return reach <= 0 ? JIGGLE_DEGREES : min(JIGGLE_DEGREES, Double(JIGGLE_REACH / reach) * 180 / .pi)
}

private struct Jiggle: ViewModifier {
    let key: String
    let active: Bool
    @State private var size = CGSize.zero

    func body(content: Content) -> some View {
        TimelineView(.animation(paused: !active)) { timeline in
            content
                .onGeometryChange(for: CGSize.self) { $0.size } action: { size = $0 }
                .rotationEffect(.degrees(active ? jiggleAngle(timeline.date.timeIntervalSinceReferenceDate, key) / JIGGLE_DEGREES * jiggleDegrees(size.width, size.height) : 0))
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
    @State private var heldIntoEditing = false
    @GestureState private var pressing = false

    func body(content: Content) -> some View {
        let layout = flyScreen.layout
        content
            .disabled(heldIntoEditing)
            .simultaneousGesture(
                LongPressGesture(minimumDuration: HOLD_TO_EDIT_SECONDS)
                    .onEnded { _ in
                        guard !layout.gestureHeld else { return }
                        heldIntoEditing = true
                        layout.startEditing()
                    }
                    .sequenced(before: DragGesture(minimumDistance: 0))
                    .updating($pressing) { _, pressing, _ in pressing = true },
                including: layout.editing && !heldIntoEditing ? .subviews : .all
            )
            .onChange(of: pressing) { _, now in if !now { heldIntoEditing = false } }
    }
}

private struct OwnsLayoutHold: ViewModifier {
    @Environment(FlyScreenState.self) private var flyScreen
    @GestureState private var pressing = false
    @State private var held = false

    private func hold(_ now: Bool) {
        guard now != held else { return }
        held = now
        flyScreen.layout.holdGesture(now)
    }

    func body(content: Content) -> some View {
        content
            .simultaneousGesture(DragGesture(minimumDistance: 0).updating($pressing) { _, pressing, _ in pressing = true })
            .onChange(of: pressing) { _, now in hold(now) }
            .onDisappear { hold(false) }
    }
}

extension View {
    func holdToEditLayout() -> some View { modifier(HoldToEditLayout()) }

    func ownsLayoutHold() -> some View { modifier(OwnsLayoutHold()) }

    func layoutPlacement(_ key: String, keepOnScreen: Bool, active: Bool = true) -> some View {
        modifier(LayoutPlacement(key: key, keepOnScreen: keepOnScreen, active: active))
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

struct LayoutGroup<Content: View>: View {
    let key: String
    @ViewBuilder let content: () -> Content
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.LocalAvoidedByVideoMessage) private var avoided
    @FlyIsPortrait private var portrait

    var body: some View {
        let placedKey = orientedKey(key, !portrait)
        content()
            .holdToEditLayout()
            .modifier(KeptOnScreen(key: placedKey, active: true))
            .offset(flyScreen.layout.offsets[placedKey] ?? .zero)
            .modifier(AvoidedByVideoMessage(key: key, active: avoided))
    }
}

struct LayoutWidget<Content: View>: View {
    let key: String
    var movable: Bool = true
    var hideable: Bool = true
    var badged: Bool = true
    var badgeSpot: BadgeSpot = .inside
    var dragsWith: String? = nil
    var name: String? = nil
    var resizable: Bool = false
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
            let scale = layout.sizes[key] ?? 1
            sized(scale)
                .onGeometryChange(for: Bool.self) { $0.size.width > 0 && $0.size.height > 0 } action: { shown = $0 }
                .overlay {
                    if decorated {
                        RoundedRectangle(cornerRadius: Corner.medium).stroke(theme.colors.outline, lineWidth: 1)
                    }
                }
                .opacity(hidden ? HIDDEN_ALPHA : 1)
                .osdShadow(flyOsd)
                .overlay {
                    if decorated && (movable || dragsWith != nil) { LayoutDragArea(key: dragsWith ?? key) }
                }
                .overlay(alignment: badgeSpot.alignment) {
                    if decorated && badged {
                        badge(layout, hidden).offset(badgeSpot.offset)
                    }
                }
                .overlay(alignment: .bottomTrailing) {
                    if decorated && resizable && !hidden {
                        HStack(spacing: Space.s1) {
                            sizeBadge(layout, smallerItemSize(scale), "Make smaller", .closeFullscreen)
                            sizeBadge(layout, largerItemSize(scale), "Make bigger", .fullscreen)
                        }
                        .offset(x: BADGE_SIZE / 2, y: BADGE_SIZE / 2)
                    }
                }
                .modifier(Jiggle(key: key, active: decorated))
                .holdToEditLayout()
                .modifier(KeptOnScreen(key: placedKey, active: movable))
                .offset(movable ? layout.offsets[placedKey] ?? .zero : .zero)
                .modifier(AvoidedByVideoMessage(key: key, active: avoided))
        }
    }

    @ViewBuilder
    private func sized(_ scale: Double) -> some View {
        if resizable {
            ScaledLayout(scale: scale) { content().scaleEffect(scale) }
        } else {
            content()
        }
    }

    @ViewBuilder
    private func sizeBadge(_ layout: OverlayLayoutState, _ to: Double?, _ label: String, _ icon: Icon) -> some View {
        if let to {
            Button { layout.resize(key, to) } label: {
                Image(icon)
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(theme.colors.inverseOnSurface)
                    .frame(width: BADGE_SIZE, height: BADGE_SIZE)
                    .background(theme.colors.inverseSurface, in: Circle())
                    .shadow(radius: 2)
                    .contentShape(BadgeTouchTarget())
            }
            .buttonStyle(.plain)
            .accessibilityLabel(label)
        }
    }

    @ViewBuilder
    private func badge(_ layout: OverlayLayoutState, _ hidden: Bool) -> some View {
        if hideable {
            Button { layout.setHidden(key, !hidden) } label: {
                HStack(spacing: 0) {
                    Image(hidden ? .add : .close)
                        .font(.system(size: 12, weight: .bold))
                        .frame(width: BADGE_SIZE, height: BADGE_SIZE)
                    if let name {
                        Text(name)
                            .font(.labelSmall)
                            .padding(.trailing, 8)
                    }
                }
                .foregroundStyle(hidden ? theme.colors.onPrimary : theme.colors.inverseOnSurface)
                .frame(height: BADGE_SIZE)
                .background(hidden ? theme.colors.primary : theme.colors.inverseSurface, in: Capsule())
                .shadow(radius: 2)
                .contentShape(BadgeTouchTarget())
            }
            .buttonStyle(.plain)
            .fixedSize()
            .accessibilityLabel(badgeLabel(hidden, name))
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

private struct BadgeTouchTarget: Shape {
    func path(in rect: CGRect) -> Path {
        let reach = max(0, (MINIMUM_TOUCH_TARGET - rect.height) / 2)
        return Path { path in
            path.addRects([
                rect,
                CGRect(x: rect.minX - reach, y: rect.minY - reach, width: rect.width + 2 * reach, height: reach),
                CGRect(x: rect.maxX, y: rect.minY - reach, width: reach, height: rect.height + 2 * reach),
            ])
        }
    }
}

struct OverlayEditBar: View {
    @Environment(FlyScreenState.self) private var flyScreen
    @QgcPath(FLY_STATE) private var flyJson

    var body: some View {
        if flyScreen.layout.editing && !flyScreen.guidedPanelOpen {
            OverlayEditBarContent(layout: flyScreen.layout, inFlight: flyState(flyJson).map { $0.connected && $0.armed } == true)
        }
    }
}

struct OverlayEditBarContent: View {
    let layout: OverlayLayoutState
    let inFlight: Bool
    @Environment(\.theme) private var theme
    @QgcPath(INSTRUMENTS_VIEW) private var classView
    @State private var armed = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if inFlight {
                Text(LAYOUT_EDITING_IN_FLIGHT)
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurface)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, 16)
                    .padding(.top, 12)
            }
            PlanFlowRow(spacing: Space.s2, lineSpacing: 0, lastAtEnd: true) {
                options
                Button("Done") { layout.editing = false }
                    .buttonStyle(.filled)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
        }
        .font(.labelLarge)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.large))
        .shadow(color: .black.opacity(0.2), radius: 3, y: 1)
        .task(id: armed) {
            guard armed, (try? await Task.sleep(for: .milliseconds(RESET_ARM_MILLIS))) != nil else { return }
            armed = false
        }
    }

    @ViewBuilder private var options: some View {
        let barHidden = layout.hidden.contains(BATTERY_BAR_KEY)
        Button(valueSizePillText(layout.valueSize)) {
            layout.valueSize = nextValueSize(layout.valueSize)
            writeValueSize(instrumentVehicleClass(classView), layout.valueSize)
        }
        .buttonStyle(.text)
        Button(batteryBarPillText(barHidden)) { layout.setHidden(BATTERY_BAR_KEY, !barHidden) }
            .buttonStyle(.text)
        Button(resetPillText(armed)) {
            let tap = resetTap(armed)
            if tap.reset { layout.reset() }
            armed = tap.armed
        }
        .buttonStyle(.text)
        .foregroundStyle(armed ? theme.colors.error : theme.colors.primary)
    }
}
