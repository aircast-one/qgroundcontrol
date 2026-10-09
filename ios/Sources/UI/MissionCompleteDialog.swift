import SwiftUI

let MISSION_COMPLETE_PATH = "view.missionComplete"

struct MissionComplete: Equatable {
    var id: Int64
    var imagesTaken: Int
    var resumeFromWaypoint: Int?
    var batteryWarning: Bool
}

func missionComplete(_ view: JSON?) -> MissionComplete? {
    guard let view, view["open"].bool else { return nil }
    return MissionComplete(
        id: view["id"].int64 ?? 0,
        imagesTaken: view["imagesTaken"].int(0),
        resumeFromWaypoint: view["resumeFromWaypoint"].isNull ? nil : view["resumeFromWaypoint"].int(0),
        batteryWarning: view["batteryWarning"].bool
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
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: notice, initial: true) { _, now in if let now { lastShown = now } }
            .queuedSheet(isPresented: Binding(get: { notice != nil }, set: { shown in if !shown, let notice { close(notice) } }), dialog: true) {
                if let shown = notice ?? lastShown { content(shown) }
            }
    }

    private func close(_ notice: MissionComplete) {
        closed = notice.id
        offMain { PlanCommands.dismissMissionComplete(notice.id) }
    }

    private func content(_ notice: MissionComplete) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Space.s3) {
                Text("Flight plan complete").font(.headlineSmall)
                if let images = imagesTakenText(notice.imagesTaken) {
                    Text(images).multilineTextAlignment(.center).frame(maxWidth: .infinity)
                }
                Button {
                    closed = notice.id
                    offMain {
                        PlanCommands.removeAllFromVehicle()
                        PlanCommands.dismissMissionComplete(notice.id)
                    }
                } label: {
                    Text("Remove plan from vehicle").frame(maxWidth: .infinity)
                }
                .buttonStyle(.filled)
                Button { close(notice) } label: {
                    Text("Leave plan on vehicle").frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                if let waypoint = notice.resumeFromWaypoint {
                    Divider()
                    Button {
                        closed = notice.id
                        offMain {
                            PlanCommands.resumeMission(waypoint)
                            PlanCommands.dismissMissionComplete(notice.id)
                        }
                    } label: {
                        Text("Resume mission from waypoint \(waypoint)").frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    Text("Resume Mission will rebuild the current mission from the last flown waypoint and upload it to the vehicle for the next flight.")
                        .font(.bodySmall)
                }
                if notice.batteryWarning {
                    Text("If you are changing batteries for Resume Mission do not disconnect from the vehicle.")
                        .font(.bodySmall)
                        .foregroundStyle(theme.aircast.warning)
                }
                HStack {
                    Spacer()
                    Button("Close") { close(notice) }.buttonStyle(.borderless)
                }
            }
            .padding(Space.s6)
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
        .presentationBackground(theme.colors.surfaceContainerHigh)
    }
}
