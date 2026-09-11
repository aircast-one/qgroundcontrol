import Foundation

struct TerrainPoint: Equatable {
    let x: Double
    let missionAltitude: Double
    let terrainAltitude: Double?
    let collision: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let x = (json["x"] as? NSNumber)?.doubleValue,
              let mission = (json["missionAltitude"] as? NSNumber)?.doubleValue else { return nil }
        self.x = x
        missionAltitude = mission
        terrainAltitude = (json["terrainAltitude"] as? NSNumber)?.doubleValue
        collision = (json["collision"] as? NSNumber)?.boolValue ?? false
    }
}

struct TerrainProfile: Equatable {
    let points: [TerrainPoint]
    let minAltitude: Double
    let maxAltitude: Double
    let unknownTerrain: Int
    let usable: Bool
    let groundKnown: Bool
    let hasCollision: Bool
    let distanceText: String
    let lowestText: String
    let highestText: String

    static let empty = TerrainProfile()

    private init() {
        points = []
        minAltitude = 0
        maxAltitude = 0
        unknownTerrain = 0
        usable = false
        groundKnown = false
        hasCollision = false
        distanceText = ""
        lowestText = ""
        highestText = ""
    }

    init(_ json: [String: Any]) {
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        points = ((json["points"] as? [Any]) ?? []).compactMap(TerrainPoint.init)
        minAltitude = (json["minAltitudeMeters"] as? NSNumber)?.doubleValue ?? 0
        maxAltitude = (json["maxAltitudeMeters"] as? NSNumber)?.doubleValue ?? 0
        unknownTerrain = (json["unknownTerrain"] as? NSNumber)?.intValue ?? 0
        usable = flag("usable")
        groundKnown = flag("groundKnown")
        hasCollision = flag("hasCollision")
        distanceText = text("distanceText")
        lowestText = text("lowestText")
        highestText = text("highestText")
    }

    func x(_ point: TerrainPoint, width: Double) -> Double { point.x * width }

    // Where the mission is under the ground, as unbroken stretches rather than a mark per sample.
    // A survey samples its whole flown path, so a collision that is one continuous run arrived as
    // four hundred overlapping dots -- each covering six of its neighbours, rebuilt on every
    // terrain event, and smeared into a bar that could not be told from two separate stretches.
    //
    // Adjacency is by position in the list, not by comparing x values: the first version looked
    // up each point's predecessor by scanning for it, which is a quadratic walk of four hundred
    // samples on every redraw.
    var collisionRuns: [ClosedRange<Double>] {
        points.reduce(into: (runs: [ClosedRange<Double>](), continuing: false)) { state, point in
            guard point.collision else { return state.continuing = false }
            if state.continuing, let open = state.runs.last {
                state.runs[state.runs.count - 1] = open.lowerBound...point.x
            } else {
                state.runs.append(point.x...point.x)
            }
            state.continuing = true
        }.runs
    }

    func y(_ altitude: Double, height: Double) -> Double {
        let span = maxAltitude - minAltitude
        guard span > 0 else { return height / 2 }
        return height - (altitude - minAltitude) / span * height
    }
}

