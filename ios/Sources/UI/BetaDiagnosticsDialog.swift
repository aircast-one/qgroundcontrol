import SwiftUI

let BETA_DIAGNOSTICS_PATH = "view.betaDiagnostics"

struct BetaDiagnostics: Equatable {
    let title: String
    let text: String
    let accept: String
    let dismiss: String
    let endpoint: String
}

func betaDiagnostics(_ view: JSON?) -> BetaDiagnostics? {
    guard let view, view["show"].bool else { return nil }
    return BetaDiagnostics(
        title: view["title"].string,
        text: view["text"].string,
        accept: view["accept"].string,
        dismiss: view["dismiss"].string,
        endpoint: view["endpoint"].string
    )
}

struct BetaDiagnosticsDialog: View {
    @QgcPath(BETA_DIAGNOSTICS_PATH) private var view

    var body: some View {
        let prompt = betaDiagnostics(view)
        Color.clear
            .invisibleAnchor()
            .alert(prompt?.title ?? "", isPresented: Binding(get: { prompt != nil }, set: { _ in }), presenting: prompt) { prompt in
                Button(prompt.accept) { offMain { AppCommands.acceptBetaDiagnostics() } }
                Button(prompt.dismiss, role: .cancel) { offMain { AppCommands.dismissBetaDiagnostics() } }
            } message: { prompt in
                Text(prompt.text)
            }
    }
}
