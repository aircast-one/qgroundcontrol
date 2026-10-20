import SwiftUI

let HOLD_TO_CONFIRM_MS = 500
let HOLD_HELP_MS = 3000
let HOLD_TO_CONFIRM_HELP = "Hold to Confirm"
private let HOLD_FILL_ALPHA = 0.35

let GRIPPER_ACTIONS = ["release", "grab", "hold"]
private let GRIPPER_CODES = ["release": 0, "grab": 1, "hold": 2]

func gripperOffers(_ extras: [GuidedOffer]) -> [GuidedOffer] {
    GRIPPER_ACTIONS.compactMap { id in extras.first { $0.id == id } }
}

struct HoldToConfirmButton: View {
    let text: String
    let enabled: Bool
    let onActivated: () -> Void
    @State private var progress = 0.0
    @State private var showHelp = false
    @State private var fired = false
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack {
            GeometryReader { box in
                theme.colors.primary.opacity(HOLD_FILL_ALPHA)
                    .frame(width: box.size.width * progress)
            }
            VStack(spacing: 0) {
                Text(text).font(.labelLarge).foregroundStyle(theme.colors.onSecondaryContainer)
                if showHelp {
                    Text(HOLD_TO_CONFIRM_HELP).font(.labelSmall).foregroundStyle(theme.colors.onSecondaryContainer)
                }
            }
        }
        .frame(maxWidth: .infinity)
        .frame(height: 56)
        .background(theme.colors.secondaryContainer)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .opacity(enabled ? 1 : DISABLED_ALPHA)
        .contentShape(Rectangle())
        .onLongPressGesture(minimumDuration: Double(HOLD_TO_CONFIRM_MS) / 1000) {
            fired = true
            showHelp = false
            onActivated()
        } onPressingChanged: { pressing in
            if pressing {
                showHelp = false
                fired = false
                withAnimation(.linear(duration: Double(HOLD_TO_CONFIRM_MS) / 1000)) { progress = 1 }
            } else {
                withAnimation(nil) { progress = 0 }
                showHelp = !fired
            }
        }
        .allowsHitTesting(enabled)
        .task(id: showHelp) {
            guard showHelp, (try? await Task.sleep(for: .milliseconds(HOLD_HELP_MS))) != nil else { return }
            showHelp = false
        }
    }
}

struct PayloadFixButton: View {
    let problem: PayloadProblem
    let onRefused: (String?) -> Void

    var body: some View {
        if !problem.fix.isEmpty {
            let code = problem.code
            Button(problem.fix) { Task { onRefused(await offMain { VehicleCommands.fixPayloadRefusal(code) }) } }.buttonStyle(.bordered)
        }
    }
}

struct GripperPanel: View {
    let offers: [GuidedOffer]
    let problems: [PayloadProblem]
    let onDismiss: () -> Void
    @State private var refusal: String?
    @Environment(\.theme) private var theme

    private func ask(_ call: @escaping @Sendable () -> String?, dismissing: Bool) {
        Task {
            let refused = await offMain(call)
            refusal = refused
            if refused == nil && dismissing { onDismiss() }
        }
    }

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            VStack(spacing: Space.s2) {
                ForEach(problems, id: \.code) { problem in
                    HStack(spacing: Space.s2) {
                        Text(problem.text)
                            .font(.bodySmall)
                            .foregroundStyle(problem.blocking ? theme.colors.error : theme.colors.onSurfaceVariant)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        PayloadFixButton(problem: problem) { refusal = $0 }
                    }
                }
                if let refusal {
                    Text(refusal).font(.bodyMedium).foregroundStyle(theme.colors.error).frame(maxWidth: .infinity, alignment: .leading)
                }
                ForEach(offers, id: \.id) { offer in
                    HoldToConfirmButton(text: offer.title, enabled: offer.ready) {
                        let action = GRIPPER_CODES[offer.id] ?? 0
                        ask({ VehicleCommands.gripperRefusal(action) }, dismissing: true)
                    }
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.horizontal, Space.s5)
            .padding(.bottom, Space.s6)
        }
    }
}
