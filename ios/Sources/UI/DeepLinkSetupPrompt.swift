import SwiftUI

let DEEP_LINK_SETUP_PATH = "view.deepLinkSetup"
private let ACCEPT = "deepLinkSetup.accept"
private let DECLINE = "deepLinkSetup.decline"

struct DeepLinkSetup: Equatable {
    let host: String
    let title: String
    let text: String
    let accept: String
    let decline: String
}

func deepLinkSetup(_ view: JSON?) -> DeepLinkSetup? {
    guard let view, view["show"].bool else { return nil }
    return DeepLinkSetup(host: view["host"].string, title: view["title"].string, text: view["text"].string, accept: view["accept"].string, decline: view["decline"].string)
}

struct DeepLinkSetupPrompt: View {
    @QgcPath(DEEP_LINK_SETUP_PATH) private var view
    @State private var answered: DeepLinkSetup?

    var body: some View {
        let setup = deepLinkSetup(view)
        let asking = setup.flatMap { $0 != answered ? $0 : nil }
        Color.clear
            .invisibleAnchor()
            .onChange(of: setup == nil, initial: true) { _, gone in if gone { answered = nil } }
            .alertWhenPresenterFree(
                Binding<DeepLinkSetup?>(get: { asking }, set: { if $0 == nil, let asking { answered = asking } }),
                title: { $0.title },
                actions: { shown in
                    Button(shown.decline, role: .cancel) { answer(DECLINE, shown) }
                    Button(shown.accept) { answer(ACCEPT, shown) }
                },
                message: { Text($0.text) }
            )
    }

    private func answer(_ path: String, _ setup: DeepLinkSetup) {
        let host = setup.host
        offMain { _ = Qgc.invoke(path, host) }
    }
}
