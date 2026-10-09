import SwiftUI

private let PLAN_VIEW_PATH = "view.plan"
private let GLOBAL_ALTITUDE_MODE = "plan.missionController.globalAltitudeMode"

func missionFramePicks(_ view: AltitudeModesView?) -> [AltitudeModeOffer] { view?.offers ?? [] }

func missionFrameChoice(_ view: AltitudeModesView?) -> Bool { missionFramePicks(view).filter(\.enabled).count > 1 }

func globalAltitudeFrame(_ plan: JSON?) -> Int? {
    plan.flatMap { $0.has("globalAltitudeFrame") ? $0["globalAltitudeFrame"].int(0) : nil }
}

struct MissionAltitudeFrame: View {
    @QgcPath(PLAN_VIEW_PATH) private var plan

    var body: some View {
        if let current = globalAltitudeFrame(plan) {
            MissionFrameMenu(current: current)
        }
    }
}

private struct MissionFrameMenu: View {
    let current: Int
    @QgcPath private var json: JSON?

    init(current: Int) {
        self.current = current
        _json = QgcPath(altitudeModesPath(MISSION_CONTEXT, current))
    }

    var body: some View {
        let view = altitudeModesView(json)
        let picks = missionFramePicks(view)
        HStack {
            Text("Altitude frame").font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                ForEach(picks, id: \.raw) { offer in
                    Button {
                        offMain { Qgc.set(GLOBAL_ALTITUDE_MODE, offer.raw) }
                    } label: {
                        Text(offer.title)
                        let note = refusalFor(view, offer.raw) ?? offer.help
                        if !note.isBlank { Text(note) }
                    }
                    .disabled(!offer.enabled)
                }
            } label: {
                Text(picks.first(where: \.current)?.title ?? "Frame")
                    .frame(minHeight: 48)
                    .contentShape(Rectangle())
            }
            .disabled(!missionFrameChoice(view))
        }
        .padding(.horizontal, Space.s5)
    }
}
