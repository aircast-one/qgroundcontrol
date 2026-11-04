import SwiftUI

// About as a native form: the version (tapping it shows or hides the advanced settings, as on Android) and the
// support links, each opening in the browser.
struct AboutPage: View {
    let links: [HelpLink]
    @QgcPath(ADVANCED_UI_PATH) private var advancedView
    @Environment(\.theme) private var theme
    @Environment(\.openURL) private var openURL

    var body: some View {
        NativeForm {
            Section {
                LabeledContent("Aircast version", value: qgcVersion())
                    .contentShape(Rectangle())
                    .onTapGesture { if advancedView != nil { toggleAdvancedUi(advancedUiShown(advancedView)) } }
            } header: {
                SectionHeader(text: "About")
            }
            Section {
                ForEach(aboutLinks(links), id: \.url) { link in
                    Button {
                        URL(string: link.url).map { openURL($0) }
                    } label: {
                        HStack(spacing: Space.s3) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(link.name).foregroundStyle(Color.primary)
                                Text(link.host).font(.subheadline).foregroundStyle(Color.secondary)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            Image(systemName: "arrow.up.forward.square")
                                .foregroundStyle(Color.secondary)
                                .accessibilityHidden(true)
                        }
                        .contentShape(Rectangle())
                    }
                    .listRowButton()
                }
            } header: {
                SectionHeader(text: "Support")
            }
        }
    }
}

let PRIVACY_POLICY_LINK = HelpLink(name: "Privacy policy", url: "https://aircast.one/privacy", host: "aircast.one")

func aboutLinks(_ links: [HelpLink]) -> [HelpLink] { links + [PRIVACY_POLICY_LINK] }

func qgcVersion() -> String {
    qgcVersion(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "", is64Bit: MemoryLayout<Int>.size == 8)
}

func qgcVersion(_ name: String, is64Bit: Bool) -> String { "\(name) \(is64Bit ? "64" : "32") bit" }
