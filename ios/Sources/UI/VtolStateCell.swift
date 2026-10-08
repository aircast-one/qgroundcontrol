import SwiftUI

struct VtolState: Equatable {
    let forward: Bool
    let flying: Bool

    var label: String { forward ? "FW(vtol)" : "MR(vtol)" }
    var transition: String { forward ? "vtolTransitionToMultiRotor" : "vtolTransitionToFixedWing" }
}

func vtolState(_ view: JSON?) -> VtolState? {
    guard let it = view, it["vtol"].bool else { return nil }
    return VtolState(forward: it["vtolInFwdFlight"].bool, flying: it["flying"].bool)
}

struct VtolStateCell: View {
    @QgcPath(GUIDED_ACTIONS) private var view
    @State private var confirming = false

    var body: some View {
        if let state = vtolState(view) {
            let offer = guidedOffers(view)[state.transition]
            let named = state.forward ? "Transition to Multi-Rotor" : "Transition to Fixed Wing"
            Text(state.label)
                .font(state.flying ? .titleMedium : .labelLarge)
                .onTapGesture { if state.flying { confirming = true } }
                .alert(
                    offer.map { $0.title.isBlank ? named : $0.title } ?? named,
                    isPresented: $confirming,
                    actions: {
                        let command = guidedCommand(state.transition, nil)
                        Button(named) {
                            confirming = false
                            command?()
                        }
                        .disabled(offer?.ready != true || command == nil)
                        Button("Cancel", role: .cancel) { confirming = false }
                    },
                    message: { offer.map { Text($0.prompt) } }
                )
        }
    }
}
