import Foundation

// FIVE FIELDS WERE DECODED HERE AND READ BY NOTHING, and they are gone rather than drawn.
// landingAltitudeMetres, landingHeadingDegrees and landingDistanceMetres arrive from landing.rs
// as raw numbers with no *Text sibling, so this head cannot spell them without inventing a
// precision the core did not choose -- which is the coupling d07d9779e removed from the item
// altitude field. loiterToAltitude and index had no drawing and no caller at all.
struct LandingPattern: Equatable {
    let landing: GeoPoint?
    let slopeStart: GeoPoint?
    let finalApproach: GeoPoint?
    let loiterRadiusMetres: Double?
    let loiterRadiusText: String
    let loiterClockwise: Bool?
    let reason: String

    static let none = LandingPattern()

    private init() {
        landing = nil
        slopeStart = nil
        finalApproach = nil
        loiterRadiusMetres = nil
        loiterRadiusText = ""
        loiterClockwise = nil
        reason = ""
    }

    init(_ json: Any?) {
        let read = (json as? [String: Any]) ?? [:]
        let number: (String) -> Double? = { key in
            guard let value = (read[key] as? NSNumber)?.doubleValue, value.isFinite else {
                return nil
            }
            return value
        }
        let truth: (String) -> Bool? = { key in (read[key] as? NSNumber)?.boolValue }
        landing = GeoPoint(json: read["landing"])
        slopeStart = GeoPoint(json: read["slopeStart"])
        finalApproach = GeoPoint(json: read["finalApproach"])
        loiterRadiusMetres = number("loiterRadiusMetres")
        loiterRadiusText = (read["loiterRadiusText"] as? String) ?? ""
        loiterClockwise = truth("loiterClockwise")
        reason = (read["reason"] as? String) ?? ""
    }

    var refused: Bool { !reason.isEmpty }


    var descent: [GeoPoint] {
        guard let slopeStart, let landing else { return [] }
        return [slopeStart, landing]
    }

    var loiterCentre: GeoPoint? {
        guard loiterRadiusMetres != nil else { return nil }
        return finalApproach
    }

    var drawnRadius: Double? {
        guard finalApproach != nil else { return nil }
        return loiterRadiusMetres
    }

    static let clockwiseWord = "clockwise"
    static let anticlockwiseWord = "anticlockwise"

    var turnWord: String {
        guard let loiterClockwise else { return "" }
        return loiterClockwise ? LandingPattern.clockwiseWord : LandingPattern.anticlockwiseWord
    }

    var loiterDetail: String {
        guard !loiterRadiusText.isEmpty else { return "" }
        let turn = turnWord
        return turn.isEmpty ? loiterRadiusText : "\(loiterRadiusText) \(turn)"
    }
}
