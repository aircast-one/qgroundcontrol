import SwiftUI

struct GuidedValuePanel<Content: View>: View {
    let title: String
    let sentence: String
    let commitLabel: String
    let commitEnabled: Bool
    let onCommit: () -> Void
    let onCancel: () -> Void
    @ViewBuilder let content: () -> Content
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.theme) private var theme
    @QgcPath(FLY_STATE) private var flyJson
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @FlyIsPortrait private var portrait
    @State private var contentHeight: CGFloat = 0

    var body: some View {
        let readiness = guidedReadiness(flyState(flyJson))
        let vehicles = vehicleChoices(vehiclesJson)
        let hold = holdLabel(guidedCommitLabel(commitLabel, readiness))
        let holdEnabled = commitEnabled && readiness?.blocks != true
        VStack(spacing: Space.s2) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s2) {
                    Text(title).font(.titleLarge)
                    if let vehicle = guidedVehicle(vehicles.choices.count, vehicles.active?.name) {
                        Text(vehicle).font(.labelLarge).foregroundStyle(theme.colors.primary)
                    }
                    if !sentence.isBlank {
                        Text(sentence).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    if let warning = readiness {
                        Text(warning.text)
                            .font(.bodyMedium)
                            .foregroundStyle(warning.blocks ? theme.colors.onErrorContainer : theme.colors.onSurface)
                            .padding(Space.s3)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(warning.blocks ? theme.colors.errorContainer : theme.aircast.warningContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
                    }
                    content()
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, Space.s2)
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { contentHeight = $0 }
            }
            .scrollBounceBehavior(.basedOnSize)
            .frame(maxHeight: contentHeight > 0 ? contentHeight : nil)
            if portrait {
                HoldToConfirm(label: hold, enabled: holdEnabled, onConfirm: onCommit)
                Button("Cancel", action: onCancel).buttonStyle(.borderless)
            } else {
                HStack(spacing: Space.s2) {
                    HoldToConfirm(label: hold, enabled: holdEnabled, onConfirm: onCommit)
                        .frame(maxWidth: .infinity)
                    Button("Cancel", action: onCancel).buttonStyle(.borderless)
                }
            }
        }
        .padding(Space.s4)
        .frame(maxWidth: .infinity)
        .background(theme.colors.surfaceContainerLow, in: RoundedRectangle(cornerRadius: Corner.extraLarge))
        .onAppear { flyScreen.guidedPanels += 1 }
        .onDisappear { flyScreen.guidedPanels -= 1 }
    }
}

func guidedCommitLabel(_ label: String, _ readiness: Readiness?) -> String {
    let split = label.range(of: " \u{00B7} ")
    let action = split.map { String(label[..<$0.lowerBound]) } ?? label
    let target = split.map { String(label[$0.upperBound...]) }
    let anyway = readiness.map { !$0.blocks } == true ? "\(action) anyway" : action
    return [anyway, target].compactMap { $0 }.joined(separator: " \u{00B7} ")
}

func guidedPresets(_ unit: String, _ minimum: Double, _ maximum: Double) -> [Double] {
    (unit == "ft" ? [15.0, 30.0, 60.0, 150.0] : [5.0, 10.0, 20.0, 50.0]).filter { $0 >= minimum && $0 <= maximum }
}

private struct GuidedChip: View {
    let label: String
    var selected = false
    var enabled = true
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            Text(label)
                .font(.labelLarge)
                .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
                .padding(.horizontal, Space.s3)
                .frame(height: 32)
                .background(selected ? theme.colors.secondaryContainer : .clear, in: RoundedRectangle(cornerRadius: Corner.small))
                .overlay {
                    if !selected {
                        RoundedRectangle(cornerRadius: Corner.small).stroke(theme.colors.outline, lineWidth: 1)
                    }
                }
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : 0.38)
    }
}

struct GuidedPresets: View {
    let value: Double
    let minimum: Double
    let maximum: Double
    let unit: String
    let onValue: (Double) -> Void

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: Space.s2) {
                ForEach(guidedPresets(unit, minimum, maximum), id: \.self) { preset in
                    GuidedChip(label: "\(guidedValueText(preset, unit).removingSuffix(".0")) \(unit)", selected: preset == value) { onValue(preset) }
                }
            }
        }
    }
}

func guidedVehicle(_ vehicleCount: Int, _ activeName: String?) -> String? {
    guard let activeName, vehicleCount >= 2, !activeName.isBlank else { return nil }
    return activeName
}

private let METRIC_UNITS: Set<String> = ["m", "m/s", "km/h"]

func guidedBounds(_ minimum: Double?, _ maximum: Double?) -> (Double, Double)? {
    guard let minimum, let maximum else { return nil }
    return (minimum, maximum)
}

func guidedDecimals(_ unit: String) -> Int { METRIC_UNITS.contains(unit) ? 1 : 0 }

private func halfUp(_ digits: String, _ decimals: Int) -> Double? {
    guard var exact = Decimal(string: digits, locale: Locale(identifier: "en_US_POSIX")) else { return nil }
    var rounded = Decimal()
    NSDecimalRound(&rounded, &exact, decimals, .plain)
    return Double(rounded.description)
}

func guidedRounded(_ value: Double, _ unit: String) -> Double {
    halfUp(String(format: "%.80f", value), guidedDecimals(unit)) ?? value
}

func guidedStepped(_ value: Double, _ delta: Int, _ minimum: Double, _ maximum: Double, _ unit: String) -> Double {
    guidedRounded(min(max(value + Double(delta), minimum), maximum), unit)
}

func guidedTyped(_ text: String, _ minimum: Double, _ maximum: Double, _ unit: String) -> Double? {
    typedNumber(text).map { guidedRounded(min(max($0, minimum), maximum), unit) }
}

func guidedValueText(_ value: Double, _ unit: String) -> String {
    String(format: "%.\(guidedDecimals(unit))f", halfUp("\(value)", guidedDecimals(unit)) ?? value)
}

func guidedQuickPicks(_ value: Double, _ minimum: Double, _ maximum: Double, _ unit: String) -> [(String, Double)] {
    [10, 20].map { step in ("+\(step) \(unit)".trimmed, guidedStepped(value, step, minimum, maximum, unit)) } +
        [("Max \(guidedValueText(maximum, unit).removingSuffix(".0")) \(unit)".trimmed, guidedRounded(maximum, unit))]
}

struct GuidedQuickPicks: View {
    let value: Double
    let minimum: Double
    let maximum: Double
    let unit: String
    let onValue: (Double) -> Void

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: Space.s2) {
                ForEach(Array(guidedQuickPicks(value, minimum, maximum, unit).enumerated()), id: \.offset) { _, pick in
                    GuidedChip(label: pick.0, enabled: pick.1 != value) { onValue(pick.1) }
                }
            }
        }
    }
}

struct GuidedStepper: View {
    let value: Double
    let label: String
    let unit: String
    let minimum: Double
    let maximum: Double
    let onValue: (Double) -> Void
    @State private var typing: String?
    @State private var width: CGFloat = .infinity
    @FocusState private var focused: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        let narrow = width < GUIDED_STEPPER_WIDE
        Group {
            if narrow {
                VStack(alignment: .leading, spacing: Space.s2) {
                    reading(narrow)
                    steps
                }
            } else {
                HStack {
                    reading(narrow).frame(maxWidth: .infinity, alignment: .leading)
                    steps
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width = $0 }
    }

    private func reading(_ narrow: Bool) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            if let typed = typing {
                TextField("", text: Binding(get: { typing ?? "" }, set: { typing = $0 }))
                    .keyboardType(.numbersAndPunctuation)
                    .submitLabel(.done)
                    .textFieldStyle(.roundedBorder)
                    .focused($focused)
                    .frame(width: 160)
                    .onAppear { focused = true }
                    .onSubmit {
                        if let entered = guidedTyped(typed, minimum, maximum, unit) { onValue(entered) }
                        focused = false
                        typing = nil
                    }
            } else {
                Text(guidedReading(value, unit))
                    .font(narrow ? .headlineMedium : .displaySmall)
                    .monospacedDigit()
                    .lineLimit(1)
                    .onTapGesture { typing = guidedValueText(value, unit) }
            }
            if !label.isBlank {
                Text(label).font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
    }

    private var steps: some View {
        HStack(spacing: Space.s2) {
            ForEach([(-1, "\u{2212}"), (1, "+")], id: \.0) { delta, sign in
                Button { onValue(guidedStepped(value, delta, minimum, maximum, unit)) } label: {
                    Text(sign)
                        .font(.titleLarge)
                        .foregroundStyle(theme.colors.onSurface)
                        .frame(width: 40, height: 40)
                        .background(theme.colors.surfaceContainerHighest, in: Circle())
                }
                .buttonStyle(.plain)
            }
        }
    }
}

private let GUIDED_STEPPER_WIDE: CGFloat = 220

func guidedReading(_ value: Double, _ unit: String) -> String {
    [guidedValueText(value, unit), unit].filter { !$0.isBlank }.joined(separator: " ")
}
