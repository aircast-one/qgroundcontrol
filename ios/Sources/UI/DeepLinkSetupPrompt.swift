import SwiftUI

let DEEP_LINK_SETUP_PATH = "view.deepLinkSetup"
private let ACCEPT = "deepLinkSetup.accept"
private let DECLINE = "deepLinkSetup.decline"

struct DeepLinkSetup: Equatable {
    let title: String
    let text: String
    let accept: String
    let decline: String
}

func deepLinkSetup(_ view: JSON?) -> DeepLinkSetup? {
    guard let view, view["show"].bool else { return nil }
    return DeepLinkSetup(title: view["title"].string, text: view["text"].string, accept: view["accept"].string, decline: view["decline"].string)
}

struct DeepLinkSetupPrompt: View {
    @QgcPath(DEEP_LINK_SETUP_PATH) private var view

    var body: some View {
        let setup = deepLinkSetup(view)
        Color.clear
            .invisibleAnchor()
            .alert(setup?.title ?? "", isPresented: Binding(get: { setup != nil }, set: { _ in })) {
                Button(setup?.accept ?? "") { answer(ACCEPT) }
                Button(setup?.decline ?? "", role: .cancel) { answer(DECLINE) }
            } message: {
                Text(setup?.text ?? "")
            }
    }

    private func answer(_ path: String) {
        offMain { _ = Qgc.invoke(path) }
    }
}
