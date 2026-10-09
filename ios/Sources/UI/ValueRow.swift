import SwiftUI

let VALUE_OFF = "Off"
let VALUE_NO_LIMIT = "No limit"
private let NO_LIMIT_NOTCH = 0.1
private let NO_LIMIT_SNAP = 0.25
private let WIDE_ROW: CGFloat = 560
private let ROW_SLIDER_WIDTH: CGFloat = 240
private let SLIDER_SPAN_LIMIT = 100_000.0
private let DISABLED_AT_ZERO = "(?i)(disabled if 0|0 (disables|to disable|hides|turns .* off)|set to 0 to disable|zero disables)"

func opensAsValue(_ fact: Fact) -> Bool {
    showsAsField(fact) && !fact.isString && !fact.isBitmask && !fact.isEnum
}

private func numberOf(_ fact: Fact) -> Double? { factNumber(fact).map { Double($0) } }

func disablesAtZero(_ fact: Fact) -> Bool {
    (fact.description + " " + fact.longDescription).range(of: DISABLED_AT_ZERO, options: .regularExpression) != nil
}

func valueText(_ fact: Fact) -> String {
    disablesAtZero(fact) && numberOf(fact) == 0
        ? offLabel(fact)
        : [fact.valueString.trimmed, fact.units].filter { !$0.isBlank }.joined(separator: " ")
}

func valueDecimals(_ fact: Fact) -> Int {
    let text = fact.valueString.trimmed
    return text.firstIndex(of: ".").map { text[text.index(after: $0)...].prefix(while: \.isWholeNumber).count } ?? 0
}

func valueStep(_ fact: Fact) -> Double {
    USEFUL_BANDS[fact.name] != nil ? metresScale(fact) : pow(10, -Double(valueDecimals(fact)))
}

private func boundOf(_ text: String, _ isDefaultForType: Bool) -> Double? {
    text.doubleOrNil.flatMap { !isDefaultForType && $0.isFinite ? $0 : nil }
}

func valueBounds(_ fact: Fact) -> (Double?, Double?) {
    (boundOf(fact.minString, fact.minIsDefaultForType), boundOf(fact.maxString, fact.maxIsDefaultForType))
}

func derivedSlider(_ fact: Fact) -> FactSlider? {
    let (min, max) = valueBounds(fact)
    guard let min, let max, max > min, max - min <= SLIDER_SPAN_LIMIT else { return nil }
    return FactSlider(from: min, to: max, decimals: valueDecimals(fact), hint: "")
}

private let ALTITUDE_BAND = 20.0...500.0
private let DISTANCE_BAND = 50.0...5000.0
private let ALTITUDE_ON = 120.0
private let DISTANCE_ON = 500.0

private let USEFUL_BANDS: [String: ClosedRange<Double>] = [
    "RTL_RETURN_ALT": ALTITUDE_BAND,
    "RTL_ALT": ALTITUDE_BAND,
    "GF_MAX_VER_DIST": ALTITUDE_BAND,
    "FENCE_ALT_MAX": ALTITUDE_BAND,
    "GF_MAX_HOR_DIST": DISTANCE_BAND,
    "FENCE_RADIUS": DISTANCE_BAND,
]

private let TURN_ON_VALUES: [String: Double] = [
    "GF_MAX_VER_DIST": ALTITUDE_ON,
    "FENCE_ALT_MAX": ALTITUDE_ON,
    "GF_MAX_HOR_DIST": DISTANCE_ON,
    "FENCE_RADIUS": DISTANCE_ON,
]

private func metresScale(_ fact: Fact) -> Double { fact.units.trimmed == "cm" ? 100 : 1 }

private func clampToBounds(_ fact: Fact, _ value: Double) -> Double {
    let (min, max) = valueBounds(fact)
    return Swift.min(Swift.max(value, min ?? -.infinity), max ?? .infinity)
}

func sliderFor(_ fact: Fact) -> FactSlider? {
    USEFUL_BANDS[fact.name].flatMap { band in
        let from = clampToBounds(fact, band.lowerBound * metresScale(fact))
        let to = clampToBounds(fact, band.upperBound * metresScale(fact))
        return to > from ? FactSlider(from: from, to: to, decimals: valueDecimals(fact), hint: "") : nil
    } ?? fact.slider ?? derivedSlider(fact)
}

func offLabel(_ fact: Fact) -> String { USEFUL_BANDS[fact.name] != nil ? VALUE_NO_LIMIT : VALUE_OFF }

struct InlineSlider: Equatable {
    let from: Double
    let top: Double
    let noLimitAt: Double?
    var end: Double { noLimitAt ?? top }
}

func inlineSlider(_ fact: Fact) -> InlineSlider? {
    sliderFor(fact).map { slider in
        InlineSlider(from: slider.from, top: slider.to, noLimitAt: disablesAtZero(fact) ? slider.to + (slider.to - slider.from) * NO_LIMIT_NOTCH : nil)
    }
}

func sliderPosition(_ slider: InlineSlider, _ value: Double?) -> Double {
    if let noLimitAt = slider.noLimitAt, value == 0 { return noLimitAt }
    return min(max(value ?? slider.from, slider.from), slider.top)
}

func valueAtPosition(_ fact: Fact, _ slider: InlineSlider, _ position: Double) -> Double {
    if let noLimitAt = slider.noLimitAt, position > slider.top + (noLimitAt - slider.top) * NO_LIMIT_SNAP { return 0 }
    return clampToBounds(fact, roundedValue(fact, min(position, slider.top)))
}

func writeValueRefusal(_ fact: Fact, _ value: Double) async -> String? {
    let rounded = roundedValue(fact, value)
    let sent: Any = valueDecimals(fact) == 0 ? Int64(exactly: rounded.rounded()).map { $0 as Any } ?? rounded : rounded
    let path = fact.path
    return await offMain { Qgc.writeRefusal(path, sent) }
}

struct ChangeNotice {
    let show: (String, (() async -> String?)?) -> Void
}

extension EnvironmentValues {
    @Entry var LocalChangeNotice = ChangeNotice { _, _ in }
}

private let DONE_MARK_MS = 3000

struct SliderValueRow<Title: View>: View {
    let fact: Fact
    let label: String
    let slider: InlineSlider
    @ViewBuilder let title: () -> Title
    let onOpen: () -> Void
    let onWrite: () -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var dragging: Double? = nil
    @State private var writing = false
    @State private var done = false

    private var shownValue: Double? { dragging.map { valueAtPosition(fact, slider, $0) } ?? numberOf(fact) }

    private func commit() {
        let before = numberOf(fact)
        guard let target = dragging.map({ valueAtPosition(fact, slider, $0) }), target != before else {
            dragging = nil
            return
        }
        let fact = fact, label = label, onWrite = onWrite, notice = notice
        Task { @MainActor in
            writing = true
            let refusal = await writeValueRefusal(fact, target)
            writing = false
            guard let refusal else {
                done = true
                onWrite()
                notice.show("\(label) \(valueTextOf(fact, target))", before.map { previous in
                    {
                        let undone = await writeValueRefusal(fact, previous)
                        if undone == nil { await MainActor.run { onWrite() } }
                        return undone
                    }
                })
                return
            }
            dragging = nil
            notice.show("\(label) not changed: \(refusal)", nil)
        }
    }

    private var value: some View {
        HStack(spacing: 6) {
            if writing {
                ProgressView().controlSize(.mini)
            } else if done {
                Text("\u{2713}").font(.bodyLarge).foregroundStyle(theme.colors.primary)
            }
            Button(action: onOpen) {
                Text(valueTextOf(fact, shownValue))
                    .font(.bodyLarge)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 8)
            }
            .buttonStyle(.plain)
            .accessibilityHint("Set exactly")
        }
    }

    private var bar: some View {
        ThinSlider(
            value: dragging ?? sliderPosition(slider, numberOf(fact)),
            range: min(slider.from, slider.end)...max(slider.from, slider.end),
            enabled: fact.acceptsWrite && !writing,
            onChange: { dragging = $0 },
            onDone: commit
        )
    }

    var body: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 12) {
                title().frame(minWidth: 0, idealWidth: 0, maxWidth: .infinity, alignment: .leading)
                value
                bar.frame(width: ROW_SLIDER_WIDTH)
            }
            .frame(minWidth: WIDE_ROW)
            VStack(spacing: 0) {
                HStack {
                    title().frame(maxWidth: .infinity, alignment: .leading)
                    value
                }
                bar.frame(maxWidth: .infinity)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 6)
        .onChange(of: fact.valueString) { dragging = nil }
        .onChange(of: fact.path) {
            dragging = nil
            writing = false
            done = false
        }
        .task(id: done) {
            guard done else { return }
            try? await Task.sleep(for: .milliseconds(DONE_MARK_MS))
            if !Task.isCancelled { done = false }
        }
    }
}

private let THUMB_SIZE: CGFloat = 18
private let THUMB_GRAB: CGFloat = 24
private let TRACK_HEIGHT: CGFloat = 2
private let SLIDER_HEIGHT: CGFloat = 48
private let THUMB_SLOP: CGFloat = 6
private let THIN_SLIDER_SPACE = "thinSlider"

func thumbFraction(_ value: Double, _ range: ClosedRange<Double>) -> Double {
    let span = range.upperBound - range.lowerBound
    return span > 0 ? min(max((value - range.lowerBound) / span, 0), 1) : 0
}

func grabsThumb(_ touchX: Double, _ thumbX: Double, _ grabRadius: Double) -> Bool { abs(touchX - thumbX) <= grabRadius }

private struct ThinSlider: View {
    let value: Double
    let range: ClosedRange<Double>
    let enabled: Bool
    let onChange: (Double) -> Void
    let onDone: () -> Void
    @Environment(\.theme) private var theme
    @GestureState private var touching = false
    @State private var moving = false

    var body: some View {
        GeometryReader { geo in
            let width = geo.size.width
            let travel = max(width - THUMB_SIZE, 1)
            let thumbX = THUMB_SIZE / 2 + travel * thumbFraction(value, range)
            let active = theme.colors.onSurface.opacity(enabled ? 1 : DISABLED_SLIDER_ALPHA)
            let positionAt: (CGFloat) -> Double = { x in
                range.lowerBound + min(max((x - THUMB_SIZE / 2) / travel, 0), 1) * (range.upperBound - range.lowerBound)
            }
            ZStack(alignment: .leading) {
                Rectangle().fill(theme.colors.outlineVariant).frame(width: max(width - THUMB_SIZE, 0), height: TRACK_HEIGHT).offset(x: THUMB_SIZE / 2)
                Rectangle().fill(active).frame(width: max(thumbX - THUMB_SIZE / 2, 0), height: TRACK_HEIGHT).offset(x: THUMB_SIZE / 2)
                Circle().fill(active).frame(width: THUMB_SIZE, height: THUMB_SIZE).offset(x: thumbX - THUMB_SIZE / 2)
                Color.clear
                    .frame(width: THUMB_GRAB * 2, height: geo.size.height)
                    .contentShape(Rectangle())
                    .offset(x: thumbX - THUMB_GRAB)
                    .gesture(
                        DragGesture(minimumDistance: 0, coordinateSpace: .named(THIN_SLIDER_SPACE))
                            .updating($touching) { _, state, _ in state = true }
                            .onChanged { drag in
                                guard moving || abs(drag.translation.width) >= THUMB_SLOP else { return }
                                moving = true
                                onChange(positionAt(drag.location.x))
                            },
                        isEnabled: enabled
                    )
            }
            .frame(width: width, height: geo.size.height, alignment: .leading)
        }
        .coordinateSpace(.named(THIN_SLIDER_SPACE))
        .frame(height: SLIDER_HEIGHT)
        .onChange(of: touching) { _, now in
            guard !now, moving else { return }
            moving = false
            onDone()
        }
        .accessibilityElement()
        .accessibilityValue(String(format: "%.0f%%", thumbFraction(value, range) * 100))
        .accessibilityAdjustableAction { direction in
            guard enabled else { return }
            let step = (range.upperBound - range.lowerBound) / 20
            let next = direction == .increment ? value + step : value - step
            onChange(min(max(next, range.lowerBound), range.upperBound))
            onDone()
        }
    }
}

private let DISABLED_SLIDER_ALPHA = 0.4

private let DANGEROUS_CHOICE = "(?i)(terminat|disarm|stop motors|kill)"

func dangerousChoice(_ label: String) -> Bool { label.range(of: DANGEROUS_CHOICE, options: .regularExpression) != nil }

func safeFirst(_ options: [String]) -> [Int] {
    options.indices.filter { !dangerousChoice(options[$0]) } + options.indices.filter { dangerousChoice(options[$0]) }
}

let DANGER_NOTE = "Motors stop \u{2014} the aircraft falls"

struct SafeChoiceMenu<Label: View>: View {
    let options: [String]
    let onPick: (Int) -> Void
    @ViewBuilder let label: () -> Label
    @State private var confirming: Int? = nil

    var body: some View {
        let order = safeFirst(options)
        let firstDanger = order.first { dangerousChoice(options[$0]) }
        Menu {
            ForEach(order, id: \.self) { index in
                if index == firstDanger { Divider() }
                if dangerousChoice(options[index]) {
                    Button(role: .destructive) { confirming = index } label: {
                        Text(options[index])
                        Text(DANGER_NOTE)
                    }
                } else {
                    Button(options[index]) { onPick(index) }
                }
            }
        } label: {
            label()
        }
        .alert(
            Text(confirming.flatMap { options.indices.contains($0) ? "\(options[$0])?" : nil } ?? ""),
            isPresented: Binding(get: { confirming != nil }, set: { if !$0 { confirming = nil } }),
            presenting: confirming
        ) { index in
            Button("Choose it", role: .destructive) { onPick(index) }
            Button("Cancel", role: .cancel) {}
        } message: { _ in
            Text("If this happens in flight, the motors stop and the aircraft falls. Only choose it if a falling aircraft is safer than a flying one.")
        }
    }
}

func turnOnValue(_ fact: Fact) -> Double {
    let declared = fact.defaultValueString.doubleOrNil.flatMap { $0 > 0 ? $0 : nil }
    let named = TURN_ON_VALUES[fact.name].map { $0 * metresScale(fact) }
    let smallest = valueBounds(fact).0.flatMap { $0 > 0 ? $0 : nil } ?? valueStep(fact)
    return clampToBounds(fact, declared ?? named ?? smallest)
}

func roundedValue(_ fact: Fact, _ value: Double) -> Double {
    (value / valueStep(fact)).rounded(.toNearestOrEven) * valueStep(fact)
}

private let FAST_AFTER_REPEATS = 10
private let FAST_STEP_FACTOR = 10

func nudged(_ fact: Fact, _ from: Double, _ direction: Int, _ repeats: Int) -> Double {
    disablesAtZero(fact) && from == 0 && direction > 0
        ? turnOnValue(fact)
        : clampToBounds(fact, roundedValue(fact, from + Double(direction) * valueStep(fact) * Double(repeats >= FAST_AFTER_REPEATS ? FAST_STEP_FACTOR : 1)))
}

func steppedValue(_ fact: Fact, _ direction: Int) -> Double? { numberOf(fact).map { nudged(fact, $0, direction, 0) } }

func valueTextOf(_ fact: Fact, _ value: Double?) -> String {
    guard let value else { return valueText(fact) }
    return disablesAtZero(fact) && value == 0 ? offLabel(fact) : sliderValue(value, valueDecimals(fact), fact.units.trimmed)
}

private let HOLD_DELAY_MS = 400
private let REPEAT_MS = 90

private struct StepButton: View {
    let label: String
    let description: String
    let enabled: Bool
    let onStep: (Int) -> Void
    let onRelease: () -> Void
    @Environment(\.theme) private var theme
    @GestureState private var pressed = false
    @State private var holding: Task<Void, Never>? = nil

    private func press(_ down: Bool) {
        holding?.cancel()
        holding = nil
        guard down else { return onRelease() }
        onStep(0)
        let step = onStep
        holding = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(HOLD_DELAY_MS))
            for repeats in 1... {
                guard !Task.isCancelled else { return }
                step(repeats)
                try? await Task.sleep(for: .milliseconds(REPEAT_MS))
            }
        }
    }

    var body: some View {
        Text(label)
            .font(.titleLarge)
            .frame(width: 48, height: 48)
            .background(theme.colors.secondaryContainer, in: Circle())
            .contentShape(Circle())
            .opacity(enabled ? 1 : DISABLED_SLIDER_ALPHA)
            .gesture(DragGesture(minimumDistance: 0).updating($pressed) { _, state, _ in state = true }, isEnabled: enabled)
            .onChange(of: pressed) { press(pressed) }
            .onDisappear { holding?.cancel() }
            .accessibilityElement()
            .accessibilityLabel(description)
            .accessibilityAddTraits(.isButton)
            .accessibilityAction {
                guard enabled else { return }
                onStep(0)
                onRelease()
            }
    }
}

struct ValueControls<TypedEntry: View>: View {
    let fact: Fact
    let onWrite: () -> Void
    @ViewBuilder let typedEntry: () -> TypedEntry
    @Environment(\.theme) private var theme
    @State private var refusal: String? = nil
    @State private var pending: Double? = nil
    @State private var typing = false

    private var shown: Double? { pending ?? numberOf(fact) }

    private func write(_ value: Double) {
        let fact = fact, onWrite = onWrite
        Task { @MainActor in
            refusal = await writeValueRefusal(fact, value)
            if refusal == nil { onWrite() } else { pending = nil }
        }
    }

    private func commit() {
        guard let target = pending, target != numberOf(fact) else { return }
        write(target)
    }

    private func nudge(_ direction: Int, _ repeats: Int) {
        guard let current = pending ?? numberOf(fact) else { return }
        pending = nudged(fact, current, direction, repeats)
    }

    var body: some View {
        VStack(spacing: 8) {
            HStack(spacing: 16) {
                StepButton(label: "\u{2212}", description: "Decrease", enabled: fact.acceptsWrite, onStep: { nudge(-1, $0) }, onRelease: commit)
                Button { typing.toggle() } label: {
                    Text(valueTextOf(fact, shown))
                        .font(.headlineSmall)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                }
                .buttonStyle(.plain)
                .accessibilityHint("Type a value")
                StepButton(label: "+", description: "Increase", enabled: fact.acceptsWrite, onStep: { nudge(1, $0) }, onRelease: commit)
            }
            .frame(maxWidth: .infinity)
            if let slider = sliderFor(fact), !(disablesAtZero(fact) && shown == 0) {
                FieldSlider(value: shown, slider: slider, enabled: fact.acceptsWrite, onWrite: write)
            }
            if disablesAtZero(fact) {
                let limit = offLabel(fact) == VALUE_NO_LIMIT
                if shown == 0 {
                    Button(limit ? "Set a limit" : "Turn on") { write(turnOnValue(fact)) }.buttonStyle(.borderless)
                } else {
                    Button(limit ? "Remove the limit" : "Turn off") { write(0) }.buttonStyle(.borderless)
                }
            }
            if typing { typedEntry() }
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .onChange(of: fact.valueString) { pending = nil }
        .onChange(of: fact.path) {
            pending = nil
            refusal = nil
            typing = false
        }
    }
}
