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
    @State private var lastShown: DeepLinkSetup?

    var body: some View {
        let setup = deepLinkSetup(view)
        Color.clear
            .invisibleAnchor()
            .onChange(of: setup, initial: true) { _, now in if let now { lastShown = now } }
            .queuedSheet(isPresented: Binding(get: { setup != nil }, set: { shown in if !shown, let setup { answer(DECLINE, setup) } }), dialog: true) {
                if let shown = setup ?? lastShown { content(shown) }
            }
    }

    private func content(_ setup: DeepLinkSetup) -> some View {
        VStack(alignment: .leading, spacing: Space.s4) {
            Text(setup.title).font(.headlineSmall)
            Text(setup.text).font(.bodyMedium)
            HStack {
                Spacer()
                Button(setup.decline) { answer(DECLINE, setup) }
                Button(setup.accept) { answer(ACCEPT, setup) }
            }
            .buttonStyle(.text)
        }
        .padding(Space.s6)
        .fixedSize(horizontal: false, vertical: true)
        .presentationDetents([.medium])
    }

    private func answer(_ path: String, _ setup: DeepLinkSetup) {
        let host = setup.host
        offMain { _ = Qgc.invoke(path, host) }
    }
}
