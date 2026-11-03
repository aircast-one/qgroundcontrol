import SwiftUI

let HOLD_TO_CONFIRM_MS = 500
let HOLD_HELP_MS = 3000
let HOLD_TO_CONFIRM_HELP = "Hold to Confirm"
private let HOLD_FILL_ALPHA = 0.35

let GRIPPER_ACTIONS = ["release", "grab", "hold"]
private let GRIPPER_CODES = ["release": 0, "grab": 1, "hold": 2]
let PAYLOAD_STATE = "vehicle.gripper"
private let PAYLOAD_STATUS_TICK_SECONDS = 1.0
private let NO_PAYLOAD_COMMAND = "No payload command sent yet"

func gripperOffers(_ extras: [GuidedOffer]) -> [GuidedOffer] {
    GRIPPER_ACTIONS.compactMap { id in extras.first { $0.id == id } }
}

struct PayloadStatus: Equatable {
    let text: String
    let warning: Bool
}

func agoText(_ atMs: Int64, _ nowMs: Int64) -> String {
    let seconds = max((nowMs - atMs) / 1000, 0)
    if seconds < 60 { return "\(seconds) s ago" }
    let minutes = seconds / 60
    return minutes < 60 ? "\(minutes) min ago" : "\(minutes / 60) h ago"
}

private func payloadVerbs(_ action: String) -> (verb: String, done: String) {
    switch action {
    case "release": (verb: "Drop", done: "Dropped")
    case "grab": (verb: "Grab", done: "Grabbed")
    default: (verb: "Hold", done: "Holding")
    }
}

func payloadStatus(_ state: JSON?, _ nowMs: Int64) -> PayloadStatus? {
    guard let state, state.object != nil else { return nil }
    let action = state["action"].string
    let bay = state["bay"].int(0)
    let plain = payloadVerbs(action)
    let verb = bay > 0 ? "\(plain.verb) bay \(bay)" : plain.verb
    let done = bay > 0 ? "Bay \(bay) \(plain.done.lowercased())" : plain.done
    let ago = agoText(state["atMs"].int64 ?? 0, nowMs)
    switch state["outcome"].string {
    case "sent": return PayloadStatus(text: "\(verb) sent, waiting for the aircraft", warning: false)
    case "confirmed": return PayloadStatus(text: "\(done) \(ago)", warning: false)
    case "refused": return PayloadStatus(text: "\(verb) refused by the flight controller \(ago)", warning: true)
    case "unanswered":
        let advice = action == "release" ? "It may have dropped, check before dropping again." : "Check the gripper before trying again."
        return PayloadStatus(text: "\(verb) not confirmed \(ago). \(advice)", warning: true)
    default: return nil
    }
}

private struct PayloadStatusLine: View {
    let refusal: String?
    @QgcPath(PAYLOAD_STATE) private var stateJson
    @Environment(\.theme) private var theme

    var body: some View {
        TimelineView(.periodic(from: .now, by: PAYLOAD_STATUS_TICK_SECONDS)) { context in
            let nowMs = Int64(context.date.timeIntervalSince1970 * 1000)
            let status = refusal.map { PayloadStatus(text: $0, warning: true) } ?? payloadStatus(stateJson?["value"], nowMs)
            Text(status?.text ?? NO_PAYLOAD_COMMAND)
                .font(.bodyMedium)
                .foregroundStyle(status?.warning == true ? theme.colors.error : theme.colors.onSurfaceVariant)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
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
    let bays: PayloadBays
    let onDismiss: () -> Void
    @State private var refusal: String?
    @Environment(\.theme) private var theme

    private func ready(_ id: String) -> Bool { offers.contains { $0.id == id && $0.ready } }

    private func ask(_ call: @escaping @Sendable () -> String?) {
        Task { refusal = await offMain(call) }
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
                PayloadStatusLine(refusal: refusal)
                if bays.multiBay {
                    ForEach(bays.numbers, id: \.self) { bay in
                        VStack(alignment: .leading, spacing: Space.s1) {
                            if !bays.output(of: bay).isEmpty {
                                Text(bays.output(of: bay)).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                            HStack(spacing: Space.s2) {
                                HoldToConfirmButton(text: bays.dropText(bay), enabled: ready("release")) {
                                    ask({ VehicleCommands.gripperRefusal(GRIPPER_CODES["release"] ?? 0, bay: bay) })
                                }
                                Button("Grab") { ask({ VehicleCommands.gripperRefusal(GRIPPER_CODES["grab"] ?? 1, bay: bay) }) }
                                    .buttonStyle(.bordered)
                                    .disabled(!ready("grab"))
                            }
                        }
                    }
                } else {
                    ForEach(offers, id: \.id) { offer in
                        HoldToConfirmButton(text: offer.title, enabled: offer.ready) {
                            let action = GRIPPER_CODES[offer.id] ?? 0
                            ask({ VehicleCommands.gripperRefusal(action) })
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.horizontal, Space.s5)
            .padding(.bottom, Space.s6)
        }
    }
}
