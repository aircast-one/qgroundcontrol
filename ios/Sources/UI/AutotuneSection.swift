import SwiftUI

let AUTOTUNE_VIEW = "view.autotune"
let AUTOTUNE_REQUEST = "vehicle.autotune.autotuneRequest"

struct AutotuneState: Equatable {
    let canStart: Bool
    let status: String
    let progress: Float
    let warning: String
}

func autotuneState(_ view: JSON?) -> AutotuneState? {
    guard let it = view, it["available"].bool else { return nil }
    return AutotuneState(canStart: it["canStart"].bool, status: it["status"].string, progress: Float(it["progress"].double(0)), warning: it["warning"].string)
}

struct AutotuneSection: View {
    @QgcPath(AUTOTUNE_VIEW) private var view
    @State private var confirming = false

    var body: some View {
        if let state = autotuneState(view) {
            VStack(alignment: .leading, spacing: 8) {
                Button("Start AutoTune") { confirming = true }
                    .buttonStyle(.borderedProminent)
                    .disabled(!state.canStart)
                Text(state.status)
                ProgressView(value: Double(min(max(state.progress, 0), 1)))
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .alert("Start AutoTune", isPresented: $confirming) {
                Button("Ok") { offMain { Qgc.invoke(AUTOTUNE_REQUEST) } }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text(state.warning)
            }
        }
    }
}
