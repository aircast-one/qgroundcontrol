import SwiftUI

private let REVERT_ALLOW_TAKEOVER = "vehicle.startTimerRevertAllowTakeover"

struct IncomingControlRequest: Equatable, Identifiable {
    let systemId: Int
    let timeoutMs: Int64
    let remainingMs: Int64

    var id: Int { systemId }
}

struct ControlPrompt: Equatable {
    let incoming: IncomingControlRequest?
    let revertMs: Int64?
}

func controlPrompt(_ view: JSON?) -> ControlPrompt {
    let request = view?["incomingRequest"]
    let revert = view.flatMap { $0.has("takeoverRevertMs") && $0["inControl"].bool ? $0["takeoverRevertMs"].int64 ?? 0 : nil }
    return ControlPrompt(
        incoming: request?.object == nil ? nil : request.map {
            IncomingControlRequest(systemId: $0["systemId"].int(0), timeoutMs: $0["timeoutMs"].int64 ?? 0, remainingMs: $0["remainingMs"].int64 ?? 0)
        },
        revertMs: revert
    )
}

func secondsLeft(_ ms: Int64) -> Int { Int((Double(ms) / 1000.0).rounded(.up)) }

@Observable
final class ControlRequestState {
    static let shared = ControlRequestState()

    var mounted = 0
    var ignored: Int?
    var revertIgnored = false
    var allowedFrom: Int?
    var shownRequest: IncomingControlRequest?

    func reset() {
        ignored = nil
        revertIgnored = false
        allowedFrom = nil
    }
}

struct ControlRequestPrompt: View {
    @QgcPath(OPERATOR_CONTROL_VIEW) private var view

    var body: some View {
        let prompt = controlPrompt(view)
        let state = ControlRequestState.shared
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: prompt.incoming == nil, initial: true) { _, gone in if gone { state.ignored = nil } }
            .onChange(of: prompt.revertMs == nil, initial: true) { _, gone in if gone { state.revertIgnored = false } }
            .onAppear { state.mounted += 1 }
            .onDisappear {
                state.mounted -= 1
                Task { @MainActor in if state.mounted == 0 { state.reset() } }
            }
    }
}

struct ControlRequestDialog: View {
    @QgcPath(OPERATOR_CONTROL_VIEW) private var view

    var body: some View {
        let prompt = controlPrompt(view)
        let state = ControlRequestState.shared
        let shown = state.mounted > 0
        let incoming = shown ? prompt.incoming.flatMap { $0.systemId != state.ignored ? $0 : nil } : nil
        let revert = shown && incoming == nil ? prompt.revertMs.flatMap { state.revertIgnored ? nil : $0 } : nil
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: incoming, initial: true) { _, now in if let now { state.shownRequest = now } }
            .queuedSheet(isPresented: Binding(get: { incoming != nil }, set: { if !$0, let asked = incoming { state.ignored = asked.systemId } }), dialog: true) {
                if let asked = incoming ?? state.shownRequest { request(asked, state) }
            }
            .alert(
                "",
                isPresented: Binding(get: { revert != nil }, set: { if !$0 { state.revertIgnored = true } }),
                actions: { Button("Ignore") { state.revertIgnored = true } },
                message: { Text("Reverting back to takeover not allowed if GCS \(state.allowedFrom.map(String.init) ?? "") doesn't take control in \(secondsLeft(revert ?? 0)) seconds ...") }
            )
    }

    private func request(_ asked: IncomingControlRequest, _ state: ControlRequestState) -> some View {
        VStack(alignment: .leading, spacing: Space.s4) {
            Text("GCS \(asked.systemId) is requesting control").font(.headlineSmall)
            VStack(alignment: .leading, spacing: Space.s2) {
                Text("Ignoring automatically in \(secondsLeft(asked.remainingMs)) seconds").font(.bodyMedium)
                ProgressView(value: asked.timeoutMs > 0 ? Double(asked.remainingMs) / Double(asked.timeoutMs) : 0)
            }
            HStack {
                Spacer()
                Button("Ignore") { state.ignored = asked.systemId }
                Button("Allow takeover") {
                    state.allowedFrom = asked.systemId
                    state.revertIgnored = false
                    offMain {
                        Qgc.invoke(REQUEST_CONTROL, true, 0)
                        Qgc.invoke(REVERT_ALLOW_TAKEOVER)
                    }
                }
            }
            .buttonStyle(.borderless)
        }
        .padding(Space.s6)
        .presentationDetents([.height(220)])
    }
}
