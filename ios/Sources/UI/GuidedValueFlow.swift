import SwiftUI

struct GuidedReading: Equatable {
    var label: String
    var unit: String
    var range: ClosedRange<Double>?
    var initial: Double?
    var sentence: String
    var sendable: Bool
}

struct GuidedValueKind {
    var offerId: String
    var title: String
    var prompt: String?
    var commitLabel: String
    var missingRange: String
    var quickPicks: Bool
    var presets: Bool = false
    var explains: Bool = true
    var read: (Double?) -> GuidedReading?
    var commit: (Double) -> Void
}

@Observable
final class OpenGuidedValue {
    let kind: GuidedValueKind
    let reading: GuidedReading
    let range: ClosedRange<Double>
    var target: Double
    var settled: Double

    init(_ kind: GuidedValueKind, _ reading: GuidedReading, _ range: ClosedRange<Double>, _ initial: Double) {
        self.kind = kind
        self.reading = reading
        self.range = range
        self.target = initial
        self.settled = initial
    }
}

func openedGuidedValue(_ kind: GuidedValueKind, _ reading: GuidedReading?) -> OpenGuidedValue? {
    guard let reading, let range = reading.range, let initial = reading.initial else { return nil }
    return OpenGuidedValue(kind, reading, range, initial)
}

private func usableRange(_ minimum: Double?, _ maximum: Double?) -> ClosedRange<Double>? {
    guard let low = minimum, let high = maximum, high > low else { return nil }
    return low...high
}

func takeoffReading(_ takeoff: GuidedTakeoff?) -> GuidedReading? {
    takeoff.map { GuidedReading(label: $0.label, unit: $0.unit, range: usableRange($0.minimum, $0.maximum), initial: $0.initial, sentence: $0.sentence, sendable: true) }
}

func speedReading(_ speed: GuidedSpeed?) -> GuidedReading? {
    speed.map { GuidedReading(label: $0.label, unit: $0.unit, range: $0.command != nil ? usableRange($0.minimum, $0.maximum) : nil, initial: $0.initial, sentence: $0.sentence, sendable: true) }
}

func altitudeReading(_ altitude: GuidedAltitude?) -> GuidedReading? {
    altitude.map { GuidedReading(label: $0.label, unit: $0.unit, range: usableRange($0.minimum, $0.maximum), initial: $0.current, sentence: $0.sentence, sendable: $0.sends) }
}

func takeoffValue(_ offer: GuidedOffer?) -> GuidedValueKind {
    GuidedValueKind(
        offerId: "takeoff",
        title: offer.map { $0.title.ifBlank("Takeoff") } ?? "Takeoff",
        prompt: offer?.prompt,
        commitLabel: "Take off",
        missingRange: "This vehicle did not report a takeoff height range.",
        quickPicks: false,
        presets: true,
        explains: false,
        read: { target in takeoffReading(guidedTakeoff(Qgc.get(target.map(guidedTakeoffPath) ?? GUIDED_TAKEOFF))) },
        commit: { target in
            if let fresh = guidedTakeoff(Qgc.get(guidedTakeoffPath(target))) { VehicleCommands.takeoff(fresh.targetMeters) }
        }
    )
}

func speedValue(_ offer: GuidedOffer?) -> GuidedValueKind {
    GuidedValueKind(
        offerId: "changeSpeed",
        title: offer.map { $0.title.ifBlank("Change Max Ground Speed") } ?? "Change Max Ground Speed",
        prompt: offer?.prompt,
        commitLabel: "Set",
        missingRange: "This vehicle did not report a speed range.",
        quickPicks: false,
        read: { target in speedReading(guidedSpeed(Qgc.get(target.map(guidedSpeedPath) ?? GUIDED_SPEED))) },
        commit: { target in
            if let fresh = guidedSpeed(Qgc.get(guidedSpeedPath(target))), let method = fresh.command {
                VehicleCommands.changeSpeed(method, fresh.targetMetersSecond)
            }
        }
    )
}

func altitudeValue(_ pauses: Bool) -> GuidedValueKind {
    GuidedValueKind(
        offerId: pauses ? PAUSE : "changeAltitude",
        title: pauses ? "Pause" : "Change altitude",
        prompt: nil,
        commitLabel: pauses ? "Pause" : "Change",
        missingRange: "This vehicle did not report an altitude range.",
        quickPicks: true,
        read: { target in altitudeReading(guidedAltitude(Qgc.get(target.map { guidedAltitudePath($0, pause: pauses) } ?? GUIDED_ALTITUDE))) },
        commit: { target in
            if let delta = guidedAltitude(Qgc.get(guidedAltitudePath(target, pause: pauses))).flatMap(altitudeDelta) {
                VehicleCommands.changeAltitude(delta, pause: pauses)
            }
        }
    )
}

func openGuidedValue(_ kind: GuidedValueKind) async -> OpenGuidedValue? {
    let reading = await offMain { kind.read(nil) }
    return openedGuidedValue(kind, reading)
}

private struct ProbeKey: Equatable {
    let open: ObjectIdentifier
    let settled: Double
}

struct GuidedValueFlow: View {
    let open: OpenGuidedValue
    let onClose: () -> Void
    @State private var probe: GuidedReading?

    var body: some View {
        let kind = open.kind
        let unit = open.reading.unit
        let settle: (Double) -> Void = { value in
            open.target = value
            open.settled = value
        }
        GuidedValuePanel(
            title: kind.title,
            sentence: kind.explains ? [kind.prompt, probe?.sentence].compactMap { $0 }.filter { !$0.isBlank }.joined(separator: "\n") : "",
            commitLabel: "\(kind.commitLabel) \u{00B7} \(guidedValueText(open.target, unit)) \(unit)".trimmed,
            commitEnabled: probe?.sendable == true,
            onCommit: {
                let target = open.target
                onClose()
                offMain { kind.commit(target) }
            },
            onCancel: onClose
        ) {
            GuidedStepper(value: open.target, label: open.reading.label, unit: unit, minimum: open.range.lowerBound, maximum: open.range.upperBound, onValue: settle)
            if kind.presets {
                GuidedPresets(value: open.target, minimum: open.range.lowerBound, maximum: open.range.upperBound, unit: unit, onValue: settle)
            }
            Slider(
                value: Binding(get: { open.target }, set: { open.target = guidedRounded($0, unit) }),
                in: open.range,
                onEditingChanged: { editing in if !editing { open.settled = open.target } }
            )
            if kind.quickPicks {
                GuidedQuickPicks(value: open.target, minimum: open.range.lowerBound, maximum: open.range.upperBound, unit: unit, onValue: settle)
            }
            if let hint = rangeLabel(open.range.lowerBound, open.range.upperBound, unit) {
                RangeHint(text: hint)
            }
        }
        .onChange(of: ObjectIdentifier(open)) { probe = nil }
        .task(id: ProbeKey(open: ObjectIdentifier(open), settled: open.settled)) {
            let at = open.settled
            let read = await offMain { kind.read(at) }
            if !Task.isCancelled { probe = read }
        }
    }
}
