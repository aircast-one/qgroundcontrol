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

    static func title(for raw: String) -> String {
        choices.first { $0.raw == raw }?.title ?? raw
    }

    static func isChoice(_ raw: String) -> Bool {
        choices.contains { $0.raw == raw }
    }

    // Only the terrain-relative modes make the terrain adjustment settings matter.
    static func usesTerrain(_ raw: String) -> Bool {
        raw == "AltitudeModeTerrainFrame" || raw == "AltitudeModeCalcAboveTerrain"
    }
}
