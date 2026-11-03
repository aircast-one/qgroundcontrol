import SwiftUI

// Stand-ins for the Device tab pieces that are not ported yet. Each block is replaced by the real port
// of the Android file named in its MARK; delete the block (or this whole file) when that port lands.

// MARK: - DeviceNetworkScreen.kt / DeviceNetwork.kt

struct DeviceNetworkScreen: View {
    let props: DeviceScreenProps

    var body: some View { FootNote(text: "Connection settings are not available in this app yet.") }
}

struct TailscaleCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let connect: String
}

func parseTailscale(_ json: JSON?) -> TailscaleCard {
    TailscaleCard(status: parseBadge(json?["status"].objectOrNil), summary: json?["summary"].string ?? "", connect: json?["connect"].string ?? "")
}

struct TailscaleSection: View {
    let tailscale: TailscaleCard

    var body: some View {
        DeviceSection(title: "Remote access", badge: tailscale.status) {
            DeviceText(text: tailscale.summary)
        }
    }
}

// MARK: - DeviceWifiWizard.kt

struct WifiReopen: Equatable {
    let host: String
    let hostname: String
}

struct WifiJoin: View {
    let reopen: WifiReopen
    var initiallyOpen: Bool = false

    var body: some View { EmptyView() }
}

