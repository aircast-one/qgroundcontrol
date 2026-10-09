import SwiftUI

private let REPEAT_DELAY_MS = 400
private let COMMIT_DELAY_MS = 500
private let REPEAT_INTERVAL_MS = 90
private let DISABLED_ALPHA = 0.38
private let STEP_BUTTON: CGFloat = 40
private let VALUE_MIN_WIDTH: CGFloat = 72

func steppedValue(_ current: Double, _ step: Double, _ direction: Int, _ range: ClosedRange<Double>?) -> Double {
    let next = current + step * Double(direction)
    return range.map { min(max(next, $0.lowerBound), $0.upperBound) } ?? next
}

func stepperText(_ value: Double?, _ decimals: Int) -> String {
    value.flatMap { $0.isFinite ? String(format: "%.\(decimals)f", $0) : nil } ?? "\u{2014}"
}

func stepperDecimals(_ value: Double?, _ step: Double) -> Int {
    [value, step].compactMap { $0 }.allSatisfy { $0 == $0.rounded(.down) } ? 0 : 1
}

private struct StepButton: View {
    let sign: String
    let description: String
    let enabled: Bool
    let onStep: () -> Void
    @Environment(\.theme) private var theme
    @GestureState private var pressed = false
    @State private var holding: Task<Void, Never>?

    var body: some View {
        Text(sign)
            .font(.titleLarge)
            .foregroundStyle(theme.colors.onSurface)
            .frame(width: STEP_BUTTON, height: STEP_BUTTON)
            .background(theme.colors.surfaceContainerHighest, in: Circle())
            .contentShape(Circle())
            .opacity(enabled ? 1 : DISABLED_ALPHA)
            .gesture(DragGesture(minimumDistance: 0).updating($pressed) { _, down, _ in down = true }, isEnabled: enabled)
            .onChange(of: pressed) { _, down in down ? press() : release() }
            .onDisappear(perform: release)
            .accessibilityElement()
            .accessibilityLabel(description)
            .accessibilityAddTraits(.isButton)
            .accessibilityAction { if enabled { onStep() } }
    }

    private func press() {
        release()
        onStep()
        holding = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(REPEAT_DELAY_MS))
            while !Task.isCancelled {
                onStep()
                try? await Task.sleep(for: .milliseconds(REPEAT_INTERVAL_MS))
            }
        }
    }

    private func release() {
        holding?.cancel()
        holding = nil
    }
}

struct SettingStepper: View {
    let label: String
    let value: Double?
    let unit: String
    let step: Double
    let onSet: (Double) -> Void
    var note: String? = nil
    var range: ClosedRange<Double>? = nil
    var slider: Bool = false
    var enabled: Bool = true
    var trailing: (() -> AnyView)? = nil
    @Environment(\.theme) private var theme
    @State private var shown: Double?
    @State private var typing = false
    @State private var stepped: Double?
    @State private var text = ""

    var body: some View {
        let decimals = stepperDecimals(shown, step)
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 0) {
                    Text(label).font(.bodyLarge)
                    if let note {
                        Text(note).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                trailing?()
                HStack(spacing: Space.s1) {
                    StepButton(sign: "\u{2212}", description: "Decrease \(label)", enabled: enabled && shown != nil) { stepTo(-1) }
                    Button {
                        text = shown == nil ? "" : stepperText(shown, decimals)
                        typing = true
                    } label: {
                        HStack(alignment: .lastTextBaseline, spacing: 0) {
                            Text(stepperText(shown, decimals)).font(.titleLarge).multilineTextAlignment(.center)
                            if !unit.isBlank {
                                Text(" \(unit)").font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                        }
                        .foregroundStyle(theme.colors.onSurface)
                        .frame(minWidth: VALUE_MIN_WIDTH)
                        .padding(.horizontal, Space.s1)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .disabled(!enabled)
                    StepButton(sign: "+", description: "Increase \(label)", enabled: enabled && shown != nil) { stepTo(1) }
                }
            }
            .frame(minHeight: 56)
            if slider, let range {
                Slider(
                    value: Binding(
                        get: { min(max(shown ?? range.lowerBound, range.lowerBound), range.upperBound) },
                        set: { shown = ($0 / step).rounded() * step }
                    ),
                    in: range
                ) { editing in
                    guard !editing, let shown else { return }
                    onSet(shown)
                }
                .disabled(!enabled)
            }
        }
        .padding(.vertical, Space.s1)
        .onChange(of: value, initial: true) { _, now in shown = now }
        .task(id: stepped) {
            guard let wanted = stepped else { return }
            try? await Task.sleep(for: .milliseconds(COMMIT_DELAY_MS))
            guard !Task.isCancelled else { return }
            stepped = nil
            onSet(wanted)
        }
        .alert(label, isPresented: $typing) {
            TextField(unit, text: $text).keyboardType(.decimalPad)
            Button("Set") { typed.map(set) }.disabled(typed == nil)
            Button("Cancel", role: .cancel) {}
        }
    }

    private var typed: Double? {
        typedNumber(text).map { wanted in range.map { min(max(wanted, $0.lowerBound), $0.upperBound) } ?? wanted }
    }

    private func set(_ wanted: Double) {
        shown = wanted
        onSet(wanted)
    }

    private func stepTo(_ direction: Int) {
        guard let current = shown else { return }
        let wanted = steppedValue(current, step, direction, range)
        shown = wanted
        stepped = wanted
    }
}
