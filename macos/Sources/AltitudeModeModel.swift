import Foundation

struct AltitudeMode: Identifiable, Equatable {
    let raw: String
    let title: String

    var id: String { raw }

    // Mixed is only meaningful for a whole mission and None means the distance is not
    // about the ground at all, so neither is a choice for a survey.
    static let choices = [
        AltitudeMode(raw: "AltitudeModeRelative", title: "Relative to launch"),
        AltitudeMode(raw: "AltitudeModeAbsolute", title: "Above mean sea level"),
        AltitudeMode(raw: "AltitudeModeCalcAboveTerrain", title: "Calculated above terrain"),
        AltitudeMode(raw: "AltitudeModeTerrainFrame", title: "Follow terrain"),
    ]

    // A mission can be mixed -- each item measured from its own frame -- which is not a
    // thing one survey can be.
    static let mixed = AltitudeMode(raw: "AltitudeModeMixed", title: "Mixed (per item)")

    static var missionChoices: [AltitudeMode] { choices + [mixed] }

    static func isMissionChoice(_ raw: String) -> Bool {
        missionChoices.contains { $0.raw == raw }
    }

    static func title(for raw: String) -> String {
        missionChoices.first { $0.raw == raw }?.title ?? raw
    }

    static func isChoice(_ raw: String) -> Bool {
        choices.contains { $0.raw == raw }
    }

    // Only the terrain-relative modes make the terrain adjustment settings matter.
    static func usesTerrain(_ raw: String) -> Bool {
        raw == "AltitudeModeTerrainFrame" || raw == "AltitudeModeCalcAboveTerrain"
    }
}
