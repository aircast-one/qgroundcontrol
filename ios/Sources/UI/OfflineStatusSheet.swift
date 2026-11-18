import SwiftUI

let OFFLINE_STATUS_VIEW = "view.offlineStatus"
private let OFFLINE_LINKS_PATH = "links.linkConfigurations"
private let CONNECTIONS_SETTINGS_PAGE = "Connections"
private let OFFLINE_POLL_MS = 1000

struct OfflineLink: Equatable {
    let index: Int
    let text: String
    let description: String
    let action: String
    let actionLabel: String
    let failed: Bool
    let connected: Bool
}

struct OfflineStatus: Equatable {
    let title: String
    let footnote: String
    let busy: Bool
    let noLinks: Bool
    let editAddress: Bool
    let signInUrl: String
    let links: [OfflineLink]
}

func offlineStatus(_ view: JSON?) -> OfflineStatus? {
    guard let json = view, json.has("title") else { return nil }
    return OfflineStatus(
        title: json["title"].string,
        footnote: json["footnote"].string,
        busy: json["busy"].bool,
        noLinks: json["noLinks"].bool,
        editAddress: json["editAddress"].bool,
        signInUrl: json["signInUrl"].string,
        links: json["links"].objects.map {
            OfflineLink(
                index: $0["index"].int(0),
                text: $0["text"].string,
                description: $0["description"].string,
                action: $0["action"].string,
                actionLabel: $0["actionLabel"].string,
                failed: $0["failed"].bool,
                connected: $0["connected"].bool
            )
        }
    )
}

let OFFLINE_SHEET = "offline"

func openAddLink(_ navigation: AppNavigationState) {
    navigation.addLinkRequested = true
    navigation.settingsPage = CONNECTIONS_SETTINGS_PAGE
}

private func openConnectionSettings(_ navigation: AppNavigationState, _ onDismiss: () -> Void) {
    navigation.settingsPage = CONNECTIONS_SETTINGS_PAGE
    onDismiss()
}

private func runLinkAction(_ link: OfflineLink) {
    if link.connected {
        LinkCommands.disconnect(link.index)
    } else if link.action == "signIn" {
        LinkCommands.signIn("@\(OFFLINE_LINKS_PATH).\(link.index)")
    } else {
        LinkCommands.connect("@\(OFFLINE_LINKS_PATH).\(link.index)")
    }
}

struct OfflineStatusSheet: View {
    @Environment(\.theme) private var theme
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.openURL) private var openURL
    let onDismiss: () -> Void
    @State private var status: OfflineStatus?
    @State private var acting: Int?
    @State private var openedSignIn = ""

    var body: some View {
        let current = status
        NativeDialogSheet(title: current?.title.nonBlank ?? "Not connected", onDismissRequest: onDismiss) {
            NativeForm {
                if let shown = current {
                    Section {
                        if shown.noLinks {
                            Button("Add link\u{2026}") {
                                openAddLink(navigation)
                                onDismiss()
                            }
                        }
                        ForEach(shown.links, id: \.index) { link in
                            Button {
                                if link.action == "edit" && !link.connected {
                                    openConnectionSettings(navigation, onDismiss)
                                } else {
                                    acting = link.index
                                    Task {
                                        status = await offMain {
                                            runLinkAction(link)
                                            return offlineStatus(Qgc.get(OFFLINE_STATUS_VIEW))
                                        }
                                        acting = nil
                                    }
                                }
                            } label: {
                                NativeRowLabel(
                                    title: link.text,
                                    subtitle: link.description.isBlank
                                        ? nil
                                        : Text(link.description).foregroundStyle(link.failed ? theme.colors.error : Color.secondary),
                                    status: link.connected || acting == link.index ? "" : link.actionLabel,
                                    statusColor: theme.colors.primary,
                                    chevron: false
                                )
                                .overlay(alignment: .trailing) {
                                    if link.connected || acting == link.index { ProgressView().controlSize(.small) }
                                }
                            }
                            .listRowButton()
                        }
                    } header: {
                        Text(shown.noLinks ? LOOKING_HINT : SAVED_LOOKING_HINT).textCase(nil)
                    } footer: {
                        footnote(shown)
                    }
                    if !shown.noLinks {
                        Section {
                            Button {
                                openConnectionSettings(navigation, onDismiss)
                            } label: {
                                NativeRowLabel(title: "Connection settings", chevron: false)
                            }
                            .listRowButton()
                        }
                    }
                }
            }
            .dialogButtons(primary: DialogButton(title: "Done", action: onDismiss))
        }
        .task {
            while !Task.isCancelled {
                status = await offMain { offlineStatus(Qgc.get(OFFLINE_STATUS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(OFFLINE_POLL_MS))
            }
        }
        .onChange(of: status?.signInUrl ?? "") { _, url in
            guard !url.isBlank, url != openedSignIn, let target = URL(string: url) else { return }
            openedSignIn = url
            openURL(target)
        }
    }

    @ViewBuilder
    private func footnote(_ shown: OfflineStatus) -> some View {
        HStack(spacing: Space.s2) {
            if shown.busy { ProgressView().controlSize(.small) }
            Text(shown.footnote)
        }
        .contentShape(Rectangle())
        .onTapGesture { if shown.editAddress { openConnectionSettings(navigation, onDismiss) } }
        .accessibilityAddTraits(shown.editAddress ? .isButton : [])
    }
}
