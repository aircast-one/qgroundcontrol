import Foundation

struct AltitudeMode: Identifiable, Equatable {
    let raw: Int
    let title: String

    var id: Int { raw }

    static let mixedRaw = 0
    static let relativeRaw = 1
    static let absoluteRaw = 2
    static let calcAboveTerrainRaw = 3
    static let terrainFrameRaw = 4

    // QGroundControlQmlGlobal::AltMode 5, which QGC names AltitudeModeNone and describes as a
    // "distance value unrelated to ground"; its own menus use it for nothing-selected. It is not
    // a mode an operator picks, and QGC draws it as an empty string.
    static let unrelatedRaw = 5

    // Distinct from the above on purpose: -1 says this head has not been given a value at all,
    // and cannot be confused with a number the enum defines.
    static let none = -1

    static let choices = [
        AltitudeMode(raw: relativeRaw, title: "Relative to launch"),
        AltitudeMode(raw: absoluteRaw, title: "Above mean sea level"),
        AltitudeMode(raw: calcAboveTerrainRaw, title: "Calculated above terrain"),
        AltitudeMode(raw: terrainFrameRaw, title: "Follow terrain"),
    ]

    static let mixed = AltitudeMode(raw: mixedRaw, title: "Mixed (per item)")

    static var missionChoices: [AltitudeMode] { choices + [mixed] }

    static func isMissionChoice(_ raw: Int) -> Bool {
        missionChoices.contains { $0.raw == raw }
    }

    static func title(for raw: Int) -> String {
        guard raw != none, raw != unrelatedRaw else { return "" }
        return missionChoices.first { $0.raw == raw }?.title ?? "Mode \(raw)"
    }

    static func isChoice(_ raw: Int) -> Bool {
        choices.contains { $0.raw == raw }
    }

    static func usesTerrain(_ raw: Int) -> Bool {
        raw == calcAboveTerrainRaw || raw == terrainFrameRaw
    }

    static func read(_ json: Any?) -> Int {
        (json as? NSNumber)?.intValue ?? none
    }
}
