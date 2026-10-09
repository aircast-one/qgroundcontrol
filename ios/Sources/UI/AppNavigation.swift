import SwiftUI

@Observable
final class AppNavigationState {
    var settingsPage: String?
    var settingsOpen = false
    var aircraftRequested = false
    var settingsShowing: SettingsGroup?
    var setupPage: String?
    var parametersSearch = ""
    var addLinkRequested = false
    var blockedReason: String?

    func openAircraft() {
        aircraftRequested = true
        settingsOpen = true
    }
}

@Observable
final class OpenPlanDocument {
    var document: URL?
    var name: String?
}

let LOG_DOWNLOAD_BLOCK = "Wait for the log download to complete or cancel it first"
let CALIBRATION_BLOCK = "Complete or cancel the current calibration first"

func navigationRefusal(_ blockedReason: String?, _ leaving: Bool) -> String? { leaving ? blockedReason : nil }

struct BlocksNavigation: View {
    let blocking: Bool
    let reason: String
    @Environment(AppNavigationState.self) private var navigation

    var body: some View {
        Color.clear
            .invisibleAnchor()
            .onChange(of: blocking, initial: true) { _, now in
                if now {
                    navigation.blockedReason = reason
                } else if navigation.blockedReason == reason {
                    navigation.blockedReason = nil
                }
            }
            .onDisappear {
                if navigation.blockedReason == reason { navigation.blockedReason = nil }
            }
    }
}
