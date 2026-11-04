import SwiftUI

let MISSION_COMPLETE_PATH = "view.missionComplete"
private let COMPLETE_TITLE = "Flight plan complete"
private let BATTERY_SWAP = "If you are changing batteries for Resume Mission do not disconnect from the vehicle."

struct MissionComplete: Equatable {
    var id: Int64
    var imagesTaken: Int
    var resumeFromWaypoint: Int?
    var batteryWarning: Bool
    var interrupted = false
    var title = COMPLETE_TITLE
    var detail = ""
    var resumeLabel = ""
}

func missionComplete(_ view: JSON?) -> MissionComplete? {
    guard let view, view["open"].bool else { return nil }
    return MissionComplete(
        id: view["id"].int64 ?? 0,
        imagesTaken: view["imagesTaken"].int(0),
        resumeFromWaypoint: view["resumeFromWaypoint"].isNull ? nil : view["resumeFromWaypoint"].int(0),
        batteryWarning: view["batteryWarning"].bool,
        interrupted: view["interrupted"].bool,
        title: view["title"].string.ifBlank(COMPLETE_TITLE),
        detail: view["detail"].string,
        resumeLabel: view["resumeLabel"].string
    )
}

func imagesTakenText(_ count: Int) -> String? { count == 0 ? nil : "\(count) Images Taken" }

struct MissionCompleteDialog: View {
    @QgcPath(MISSION_COMPLETE_PATH) private var view
    @State private var closed: Int64?
    @State private var lastShown: MissionComplete?
    @Environment(\.theme) private var theme

    var body: some View {
        let notice = missionComplete(view).flatMap { $0.id != closed ? $0 : nil }
        Color.clear
            .invisibleAnchor()
            .onChange(of: notice, initial: true) { _, now in if let now { lastShown = now } }
            .queuedSheet(isPresented: Binding(get: { notice != nil }, set: { shown in if !shown, let notice { close(notice) } }), dialog: true) {
                if let shown = notice ?? lastShown { content(shown) }
            }
    }

    private func close(_ notice: MissionComplete, then work: @escaping @Sendable () -> Void = {}) {
        closed = notice.id
        let id = notice.id
        offMain {
            work()
            PlanCommands.dismissMissionComplete(id)
        }
    }

    @ViewBuilder
    private func interrupted(_ notice: MissionComplete, at waypoint: Int) -> some View {
        Section {
            Button {
                close(notice) { PlanCommands.resumeMission(waypoint) }
            } label: {
                Text(notice.resumeLabel.ifBlank("Resume mission from waypoint \(waypoint)")).fontWeight(.semibold)
            }
            Button("Leave plan on vehicle") { close(notice) }
        } footer: {
            if !notice.detail.isBlank { Text(notice.detail) }
        }
        Section {
            Button("Remove plan from vehicle", role: .destructive) {
                close(notice) { PlanCommands.removeAllFromVehicle() }
            }
        }
    }

    @ViewBuilder
    private func finished(_ notice: MissionComplete) -> some View {
        Section {
            if let images = imagesTakenText(notice.imagesTaken) {
                Text(images)
            }
            Button("Remove plan from vehicle", role: .destructive) {
                close(notice) { PlanCommands.removeAllFromVehicle() }
            }
            Button("Leave plan on vehicle") { close(notice) }
        }
        if let waypoint = notice.resumeFromWaypoint {
            Section {
                Button("Resume mission from waypoint \(waypoint)") {
                    close(notice) { PlanCommands.resumeMission(waypoint) }
                }
            } footer: {
                Text("Resume Mission will rebuild the current mission from the last flown waypoint and upload it to the vehicle for the next flight.")
            }
        }
    }

    private func content(_ notice: MissionComplete) -> some View {
        NativeDialogStack(title: notice.title) {
            NativeForm {
                if let waypoint = notice.resumeFromWaypoint, notice.interrupted {
                    interrupted(notice, at: waypoint)
                } else {
                    finished(notice)
                }
                if notice.batteryWarning {
                    FormNote(text: BATTERY_SWAP, color: theme.aircast.warning)
                }
            }
            .dialogButtons(cancel: DialogButton(title: "Close") { close(notice) })
        }
    }
}
