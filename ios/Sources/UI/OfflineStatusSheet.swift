import SwiftUI

let OFFLINE_STATUS_VIEW = "view.offlineStatus"
private let OFFLINE_LINKS_PATH = "links.linkConfigurations"
private let CONNECTIONS_SETTINGS_PAGE = "Connections"
private let OFFLINE_POLL_MS = 1000

struct OfflineLink: Equatable {
    let index: Int
    let text: String
    let description: String
    let retry: Bool
    let failed: Bool
    let connected: Bool
}

struct OfflineStatus: Equatable {
    let title: String
    let footnote: String
    let busy: Bool
    let noLinks: Bool
    let editAddress: Bool
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
        links: json["links"].array.filter { $0.object != nil }.map {
            OfflineLink(index: $0["index"].int(0), text: $0["text"].string, description: $0["description"].string, retry: $0["retry"].bool, failed: $0["failed"].bool, connected: $0["connected"].bool)
        }
    )
}

private func openConnectionSettings(_ navigation: AppNavigationState, _ onDismiss: () -> Void) {
    navigation.settingsPage = CONNECTIONS_SETTINGS_PAGE
    onDismiss()
}

struct OfflineStatusSheet: View {
    @Environment(\.theme) private var theme
    @Environment(AppNavigationState.self) private var navigation
    let onDismiss: () -> Void
    @State private var status: OfflineStatus?

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            if let shown = status {
                VStack(alignment: .leading, spacing: Space.s1) {
                    Text(shown.title).font(.titleMedium).padding(.horizontal, Space.s5)
                    HStack(spacing: Space.s2) {
                        if shown.busy { ProgressView().controlSize(.small) }
                        Text(shown.footnote).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    .padding(.horizontal, Space.s5)
                    .contentShape(Rectangle())
                    .onTapGesture { if shown.editAddress { openConnectionSettings(navigation, onDismiss) } }
                    if shown.noLinks {
                        Button {
                            navigation.addLinkRequested = true
                            openConnectionSettings(navigation, onDismiss)
                        } label: {
                            Text("Add link\u{2026}").frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.filled)
                        .padding(.horizontal, Space.s5)
                        .padding(.vertical, Space.s2)
                    }
                    ForEach(shown.links, id: \.index) { link in
                        StatusListItem(
                            headline: link.text,
                            supporting: link.description,
                            supportingColor: link.failed ? theme.colors.error : theme.colors.onSurfaceVariant
                        ) {
                            if link.connected {
                                ProgressView().controlSize(.small)
                            } else if link.retry {
                                Text("Retry").font(.labelMedium)
                            }
                        }
                        .contentShape(Rectangle())
                        .onTapGesture {
                            offMain {
                                if link.connected {
                                    Qgc.invoke("\(OFFLINE_LINKS_PATH).\(link.index).link.disconnect")
                                } else {
                                    LinkCommands.connect("@\(OFFLINE_LINKS_PATH).\(link.index)")
                                }
                            }
                        }
                    }
                    Divider()
                    StatusListItem(headline: "Connection settings") { EmptyView() }
                        .contentShape(Rectangle())
                        .onTapGesture { openConnectionSettings(navigation, onDismiss) }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.bottom, Space.s6)
            }
        }
        .task {
            while !Task.isCancelled {
                status = await offMain { offlineStatus(Qgc.get(OFFLINE_STATUS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(OFFLINE_POLL_MS))
            }
        }
    }
}
