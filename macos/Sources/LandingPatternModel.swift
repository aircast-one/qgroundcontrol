import Foundation

struct LandingPattern: Equatable {
    let index: Int
    let landing: GeoPoint?
    let slopeStart: GeoPoint?
    let finalApproach: GeoPoint?
    let loiterRadiusMetres: Double?
    let loiterRadiusText: String
    let loiterClockwise: Bool?
    let loiterToAltitude: Bool?
    let landingAltitudeMetres: Double?
    let landingHeadingDegrees: Double?
    let landingDistanceMetres: Double?
    let reason: String

    static let none = LandingPattern()

    private init() {
        index = -1
        landing = nil
        slopeStart = nil
        finalApproach = nil
        loiterRadiusMetres = nil
        loiterRadiusText = ""
        loiterClockwise = nil
        loiterToAltitude = nil
        landingAltitudeMetres = nil
        landingHeadingDegrees = nil
        landingDistanceMetres = nil
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
        index = (read["index"] as? NSNumber)?.intValue ?? -1
        landing = GeoPoint(json: read["landing"])
        slopeStart = GeoPoint(json: read["slopeStart"])
        finalApproach = GeoPoint(json: read["finalApproach"])
        loiterRadiusMetres = number("loiterRadiusMetres")
        loiterRadiusText = (read["loiterRadiusText"] as? String) ?? ""
        loiterClockwise = truth("loiterClockwise")
        loiterToAltitude = truth("loiterToAltitude")
        landingAltitudeMetres = number("landingAltitudeMetres")
        landingHeadingDegrees = number("landingHeadingDegrees")
        landingDistanceMetres = number("landingDistanceMetres")
        reason = (read["reason"] as? String) ?? ""
    }

    var refused: Bool { !reason.isEmpty }

    var isPattern: Bool { !refused && landing != nil }

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
