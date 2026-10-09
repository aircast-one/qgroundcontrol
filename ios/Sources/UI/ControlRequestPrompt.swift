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

struct ControlRequestPrompt: View {
    @Environment(\.theme) private var theme
    @QgcPath(OPERATOR_CONTROL_VIEW) private var view
    @State private var ignored: Int?
    @State private var revertIgnored = false
    @State private var allowedFrom: Int?

    var body: some View {
        let prompt = controlPrompt(view)
        let incoming = prompt.incoming.flatMap { $0.systemId != ignored ? $0 : nil }
        let revert = incoming == nil ? prompt.revertMs.flatMap { revertIgnored ? nil : $0 } : nil
        Color.clear
            .frame(width: 0, height: 0)
            .onChange(of: prompt.incoming == nil, initial: true) { _, gone in if gone { ignored = nil } }
            .onChange(of: prompt.revertMs == nil, initial: true) { _, gone in if gone { revertIgnored = false } }
            .queuedSheet(item: Binding(get: { incoming }, set: { if $0 == nil, let shown = incoming { ignored = shown.systemId } })) { asked in
                VStack(alignment: .leading, spacing: Space.s4) {
                    Text("GCS \(asked.systemId) is requesting control").font(.headlineSmall)
                    VStack(alignment: .leading, spacing: Space.s2) {
                        Text("Ignoring automatically in \(secondsLeft(asked.remainingMs)) seconds").font(.bodyMedium)
                        ProgressView(value: asked.timeoutMs > 0 ? Double(asked.remainingMs) / Double(asked.timeoutMs) : 0)
                    }
                    HStack {
                        Spacer()
                        Button("Ignore") { ignored = asked.systemId }
                        Button("Allow takeover") {
                            allowedFrom = asked.systemId
                            revertIgnored = false
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
            .alert(
                "",
                isPresented: Binding(get: { revert != nil }, set: { if !$0 { revertIgnored = true } }),
                actions: { Button("Ignore") { revertIgnored = true } },
                message: { Text("Reverting back to takeover not allowed if GCS \(allowedFrom.map(String.init) ?? "") doesn't take control in \(secondsLeft(revert ?? 0)) seconds ...") }
            )
    }
}
