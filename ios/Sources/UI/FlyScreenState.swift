import SwiftUI

@Observable
final class FlyScreenState {
    let layout: OverlayLayoutState
    var refusal: String?
    var deckRequest: String?
    var requestedSheet: String?
    var choosingReadings = false
    var pendingMode: String?
    var instrumentEdits = 0
    var guidedPanels = 0
    var guidedPanelOpen: Bool { guidedPanels > 0 }
    var controlRequestDeadlines: [Int: Int64] = [:]
    let mapEdits = FlyMapEdits()
    let checklist = PreflightChecklistState()
    let flightActions = FlightActionsState()
    let cameraControls = CameraControlsState()
    var obstacles: [String: CGRect] = [:]
    var mapInsets = MapInsets(top: 0, bottom: 0)
    var videoTucked = false
    var pipCorner = PipCorner.TopEnd

    init(layout: OverlayLayoutState) {
        self.layout = layout
    }
}

let TRAFFIC_SHEET = "traffic"

let REQUESTABLE_SHEETS = ["more", "readings", "camera", "gimbal", "status", "status-all", "modes", TRAFFIC_SHEET]

struct OpenOnRequest: View {
    let name: String
    let open: () -> Void
    @Environment(FlyScreenState.self) private var flyScreen

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: flyScreen.requestedSheet, initial: true) { _, requested in
                guard requested == name else { return }
                flyScreen.requestedSheet = nil
                open()
            }
    }
}
