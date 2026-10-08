import Foundation

let AT_END = -1
let HOME_ITEM = 0
let BEFORE_THE_REST = 1

struct InsertOutcome: Equatable {
    let ok: Bool
    let reason: String
    var index: Int? = nil
}

private let NO_ANSWER = "The plan did not answer."
private let MISSION_INSERT = "mission.insert"
private let MISSION_REMOVE = "mission.remove"

func insertOutcome(_ view: JSON?) -> InsertOutcome {
    guard let view else { return InsertOutcome(ok: false, reason: NO_ANSWER) }
    if view["ok"].bool {
        guard case .number(let index) = view["index"] else { return InsertOutcome(ok: true, reason: "") }
        return InsertOutcome(ok: true, reason: "", index: Int(exactly: index.rounded(.towardZero)))
    }
    return InsertOutcome(ok: false, reason: view["reason"].string.ifBlank(NO_ANSWER))
}

func insertMissionItem(_ kind: String, _ latitude: Double, _ longitude: Double, _ index: Int) -> InsertOutcome {
    insertOutcome(Qgc.call(MISSION_INSERT, kind, latitude, longitude, index))
}

func removeMissionItem(_ index: Int) -> InsertOutcome {
    insertOutcome(Qgc.call(MISSION_REMOVE, index))
}
