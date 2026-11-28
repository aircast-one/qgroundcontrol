import SwiftUI

struct ActionStep: Equatable {
    let kind: String
    let title: String
    let seconds: Double?
    let item: Int?
    let places: [Int]
    let refusal: String?
}

func actionSteps(_ view: JSON?) -> [ActionStep] {
    (view?["actions"].array ?? []).map { step in
        ActionStep(
            kind: step["kind"].string,
            title: step["title"].string,
            seconds: step["seconds"].isNull ? nil : step["seconds"].nonNanDouble,
            item: step["item"].isNull ? nil : step["item"].int(0),
            places: step["places"].array.compactMap { $0.isNull ? nil : $0.int(-1) }.filter { $0 >= 0 },
            refusal: step["refusal"].isNull || step["refusal"].string.isBlank ? nil : step["refusal"].string
        )
    }
}

let HOVER = "Hover"
let WAIT = "Wait"

struct ShownAction: Equatable {
    let action: WaypointAction?
    let kind: String
    let title: String
    let value: String
    let icon: Icon
    let place: Int?
    let item: Int?
    let seconds: Double?
    let places: [Int]
    let refusal: String?
}

private func secondsText(_ seconds: Double?) -> String { seconds.map { "\(trimmedNumber($0)) s" } ?? "" }

func orderedActions(_ steps: [ActionStep], _ active: [WaypointAction]) -> [ShownAction] {
    let listed = steps.enumerated().map { place, step in
        let action = active.first { "\($0.kind)" == step.kind }
        return ShownAction(
            action: action,
            kind: step.kind,
            title: action?.title ?? step.title,
            value: action?.value ?? secondsText(step.seconds),
            icon: action?.icon ?? (step.kind == WAIT ? .pause : .planTune),
            place: place,
            item: step.item,
            seconds: step.seconds,
            places: step.places,
            refusal: step.refusal
        )
    }
    let fixed = active.filter { action in !steps.contains { $0.kind == "\(action.kind)" } }.map { action in
        ShownAction(action: action, kind: "\(action.kind)", title: action.title, value: action.value, icon: action.icon, place: nil, item: nil, seconds: nil, places: [], refusal: nil)
    }
    return listed + fixed
}

func movedOrder<T>(_ list: [T], _ from: Int, _ to: Int) -> [T] {
    guard list.indices.contains(from), list.indices.contains(to) else { return list }
    let without = list.enumerated().filter { $0.offset != from }.map(\.element)
    return Array(without.prefix(to)) + [list[from]] + Array(without.dropFirst(to))
}

func canDrop(_ step: ShownAction?, _ to: Int?) -> Bool {
    guard let step, step.place != nil, let to else { return false }
    return step.places.contains(to)
}

func projectedPlaces(_ steps: [ShownAction], _ from: Int?, _ over: Int?) -> [Int] {
    let identity = Array(steps.indices)
    guard let from, let over, steps.indices.contains(from), canDrop(steps[from], over) else { return identity }
    let order = movedOrder(identity, from, over)
    return identity.map { order.firstIndex(of: $0) ?? $0 }
}

func shownAt(_ step: ShownAction, _ place: Int) -> ShownAction {
    guard step.kind == HOVER, place > 0 else { return step }
    return ShownAction(action: step.action, kind: step.kind, title: WAIT, value: secondsText(step.seconds), icon: .pause, place: step.place, item: step.item, seconds: step.seconds, places: step.places, refusal: step.refusal)
}

struct ActionMove: Equatable {
    let waypoint: Int
    let from: Int
    let to: Int
}

func dropMove(_ waypoint: Int, _ step: ShownAction?, _ to: Int?) -> ActionMove? {
    guard let step, let from = step.place, let to, from != to, canDrop(step, to) else { return nil }
    return ActionMove(waypoint: waypoint, from: from, to: to)
}

func slotUnder(_ centres: [CGFloat], _ lifted: Int, _ dy: CGFloat, top: CGFloat, bottom: CGFloat, margin: CGFloat) -> Int? {
    guard centres.indices.contains(lifted) else { return nil }
    let centre = centres[lifted] + dy
    guard centre >= top - margin, centre <= bottom + margin else { return nil }
    return centres.indices.min { abs(centres[$0] - centre) < abs(centres[$1] - centre) }
}

struct ReorderDrag: Equatable {
    let lifted: Int
    let dy: CGFloat
    let over: Int?

    init(lifted: Int) {
        self.init(lifted: lifted, dy: 0, over: lifted)
    }

    private init(lifted: Int, dy: CGFloat, over: Int?) {
        self.lifted = lifted
        self.dy = dy
        self.over = over
    }

    func moved(to dy: CGFloat, _ centres: [CGFloat], top: CGFloat, bottom: CGFloat, margin: CGFloat) -> ReorderDrag {
        ReorderDrag(lifted: lifted, dy: dy, over: slotUnder(centres, lifted, dy, top: top, bottom: bottom, margin: margin))
    }

    func released(_ waypoint: Int, _ steps: [ShownAction]) -> ActionMove? {
        dropMove(waypoint, steps.indices.contains(lifted) ? steps[lifted] : nil, over)
    }
}

private let OUTSIDE_MARGIN: CGFloat = 32
private let ROW_GAP: CGFloat = 8
private let SPACE = "action-order"

struct ActionOrderList: View {
    let waypoint: Int
    let steps: [ShownAction]
    let onMove: (ActionMove) -> Void
    let onEdit: (ShownAction) -> (() -> Void)?
    let onRemove: (ShownAction) -> (() -> Void)?
    @Environment(\.theme) private var theme
    @State private var drag: ReorderDrag?
    @State private var frames: [Int: CGRect] = [:]

    private var orderable: [ShownAction] { steps.filter { $0.place != nil } }

    var body: some View {
        let rows = orderable
        let projected = projectedPlaces(rows, drag?.lifted, drag?.over)
        let active = drag.flatMap { rows.indices.contains($0.lifted) ? rows[$0.lifted] : nil }
        let refused = drag.map { $0.over != nil && $0.over != $0.lifted && !canDrop(active, $0.over) } ?? false
        VStack(alignment: .leading, spacing: ROW_GAP) {
            ForEach(Array(rows.enumerated()), id: \.offset) { at, step in
                let lifted = drag?.lifted == at
                let shown = shownAt(step, at < projected.count ? projected[at] : at)
                ActionOrderRow(step: shown, grip: true, lifted: lifted, refused: lifted && refused, onEdit: drag == nil ? onEdit(step) : nil, onRemove: drag == nil ? onRemove(step) : nil)
                    .offset(y: lifted ? drag?.dy ?? 0 : shift(at, projected))
                    .zIndex(lifted ? 1 : 0)
                    .animation(lifted ? nil : .snappy, value: projected)
                    .onGeometryChange(for: CGRect.self) { $0.frame(in: .named(SPACE)) } action: { frame in
                        if drag == nil { frames[at] = frame }
                    }
                    .simultaneousGesture(gesture(at))
                    .accessibilityActions {
                        if canDrop(step, at - 1) { Button("Move \(shown.title) earlier") { dropMove(waypoint, step, at - 1).map(onMove) } }
                        if canDrop(step, at + 1) { Button("Move \(shown.title) later") { dropMove(waypoint, step, at + 1).map(onMove) } }
                    }
            }
            if refused, let reason = active?.refusal {
                Text(reason).font(.bodySmall).foregroundStyle(theme.colors.error).fixedSize(horizontal: false, vertical: true)
            }
            if drag != nil, drag?.over == nil {
                Text("Let go to put it back.").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            ForEach(steps.filter { $0.place == nil }, id: \.kind) { step in
                ActionOrderRow(step: step, grip: false, lifted: false, refused: false, onEdit: onEdit(step), onRemove: onRemove(step))
            }
        }
        .coordinateSpace(.named(SPACE))
        .sensoryFeedback(.impact(weight: .medium), trigger: drag?.lifted)
        .sensoryFeedback(.selection, trigger: drag?.over)
    }

    private func shift(_ at: Int, _ projected: [Int]) -> CGFloat {
        guard let drag, at < projected.count else { return 0 }
        let height = frames[drag.lifted]?.height ?? 0
        return CGFloat(projected[at] - at) * (height + ROW_GAP)
    }

    private func gesture(_ at: Int) -> some Gesture {
        holdToReorder(holding: 0.4) { moved in
            let rows = orderable
            let centres = rows.indices.map { frames[$0]?.midY ?? 0 }
            let top = rows.indices.compactMap { frames[$0]?.minY }.min() ?? 0
            let bottom = rows.indices.compactMap { frames[$0]?.maxY }.max() ?? 0
            drag = (drag ?? ReorderDrag(lifted: at)).moved(to: moved.height, centres, top: top, bottom: bottom, margin: OUTSIDE_MARGIN)
        } onEnded: { moved in
            let move = moved == nil ? nil : drag?.released(waypoint, orderable)
            withAnimation(.snappy) { drag = nil }
            move.map(onMove)
        }
    }
}

private struct ActionOrderRow: View {
    let step: ShownAction
    let grip: Bool
    let lifted: Bool
    let refused: Bool
    let onEdit: (() -> Void)?
    let onRemove: (() -> Void)?
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 0) {
            if grip {
                Image(.dragHandle).font(.system(size: 14)).frame(width: 36, height: 40).accessibilityLabel("Hold to reorder \(step.title)")
            }
            Button { onEdit?() } label: {
                HStack(spacing: Space.s2) {
                    Image(step.icon).font(.system(size: 16)).frame(width: 20, height: 20)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(step.title).font(.labelLarge)
                        if !step.value.isBlank {
                            Text(step.value).font(.labelSmall)
                        }
                    }
                    Spacer(minLength: 0)
                }
                .padding(.leading, grip ? 0 : 12)
                .padding(.vertical, 6)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(onEdit == nil)
            if let onRemove {
                Button(action: onRemove) {
                    Image(.close).font(.system(size: 14)).frame(width: 40, height: 40).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Remove \(step.title)")
            }
        }
        .foregroundStyle(theme.colors.onSecondaryContainer)
        .background(lifted ? theme.colors.surfaceContainerHighest : theme.colors.secondaryContainer, in: RoundedRectangle(cornerRadius: 14))
        .overlay {
            if refused { RoundedRectangle(cornerRadius: 14).stroke(theme.colors.error, lineWidth: 2) }
        }
        .shadow(color: .black.opacity(lifted ? 0.45 : 0), radius: lifted ? 10 : 0, y: lifted ? 6 : 0)
    }
}
