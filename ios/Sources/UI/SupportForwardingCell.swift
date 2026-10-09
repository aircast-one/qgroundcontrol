import SwiftUI

private let SUPPORT_LINKS_VIEW = "view.links"
private let SUPPORT_HOST_SETTING = settingControl("settings.mavlinkSettings.forwardMavlinkAPMSupportHostName")

struct SupportForwardingCell: View {
    @QgcPath(SUPPORT_LINKS_VIEW) private var links
    @QgcValue(SUPPORT_HOST_SETTING) private var host
    @State private var open = false
    @Environment(\.theme) private var theme

    var body: some View {
        if links?["supportForwarding"].bool == true {
            Text("Support")
                .font(.labelMedium)
                .foregroundStyle(theme.aircast.success)
                .minimumTouchTarget()
                .onTapGesture { open = true }
                .accessibilityAddTraits(.isButton)
                .background {
                    if open {
                        AircastSheet(onDismissRequest: { open = false }) {
                            VStack(alignment: .leading, spacing: 8) {
                                Text("Mavlink traffic is being forwarded to a support server").font(.bodyMedium)
                                Text("Server name:  \(host.string)").font(.bodyMedium)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.horizontal, 20)
                            .padding(.bottom, 24)
                        }
                    }
                }
        }
    }
}
