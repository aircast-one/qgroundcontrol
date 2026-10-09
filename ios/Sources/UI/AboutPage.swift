import SwiftUI

struct AboutPage: View {
    let links: [HelpLink]
    @QgcPath(ADVANCED_UI_PATH) private var advancedView
    @Environment(\.theme) private var theme
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            SectionHeader(text: "About")
            HStack(spacing: Space.s4) {
                Text("Aircast version").font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                Spacer()
                Text(qgcVersion()).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .padding(.horizontal, Space.s4)
            .frame(minHeight: 56)
            .contentShape(Rectangle())
            .onTapGesture { if advancedView != nil { toggleAdvancedUi(advancedUiShown(advancedView)) } }
            SectionHeader(text: "Support")
            ForEach(aboutLinks(links), id: \.url) { link in
                Button {
                    URL(string: link.url).map { openURL($0) }
                } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(link.name).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                        Text(link.host).font(.bodyMedium).foregroundStyle(theme.colors.primary)
                    }
                    .frame(maxWidth: .infinity, minHeight: 64, alignment: .leading)
                    .padding(.horizontal, Space.s4)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.vertical, Space.s2)
    }
}

let PRIVACY_POLICY_LINK = HelpLink(name: "Privacy policy", url: "https://aircast.one/privacy", host: "aircast.one")

func aboutLinks(_ links: [HelpLink]) -> [HelpLink] { links + [PRIVACY_POLICY_LINK] }

func qgcVersion() -> String {
    qgcVersion(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "", is64Bit: MemoryLayout<Int>.size == 8)
}

func qgcVersion(_ name: String, is64Bit: Bool) -> String { "\(name) \(is64Bit ? "64" : "32") bit" }
