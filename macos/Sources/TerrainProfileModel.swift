import Foundation

struct TerrainPoint: Equatable {
    let x: Double
    let missionAltitude: Double
    let terrainAltitude: Double?
    let collision: Bool
    let sequence: Int

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let x = (json["x"] as? NSNumber)?.doubleValue,
              let mission = (json["missionAltitude"] as? NSNumber)?.doubleValue else { return nil }
        self.x = x
        missionAltitude = mission
        terrainAltitude = (json["terrainAltitude"] as? NSNumber)?.doubleValue
        collision = (json["collision"] as? NSNumber)?.boolValue ?? false
        sequence = (json["sequence"] as? NSNumber)?.intValue ?? -1
    }
}

struct TerrainMarker: Equatable, Identifiable {
    let sequence: Int
    let x: Double

    var id: Int { sequence }
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
    let clearanceText: String
    let minClearance: Double?
    let clearanceComplete: Bool

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
        clearanceText = ""
        minClearance = nil
        clearanceComplete = false
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
        clearanceText = text("clearanceText")
        minClearance = (json["minClearanceMetres"] as? NSNumber)?.doubleValue
        clearanceComplete = flag("clearanceComplete")
        lowestText = text("lowestText")
        highestText = text("highestText")
    }

    // Saying a mission is below terrain and not by how much leaves the operator to pick a new
    // altitude by eye off a plot 90 points tall spanning three hundred metres. The core signs the
    // clearance -- headroom above, intrusion below -- and spells the magnitude, so this is one
    // sentence rather than a branch on two numbers.
    static let below = "Mission is below terrain"
    static let clears = "Clears terrain"

    var clearanceSentence: String {
        guard !clearanceText.isEmpty else { return hasCollision ? TerrainProfile.below : "" }
        return hasCollision
            ? "\(TerrainProfile.below) by up to \(clearanceText)"
            : "\(TerrainProfile.clears) by \(clearanceText)"
    }

    // A margin is worth stating only when it is the whole story, and whether it is belongs to
    // whoever measured it: the head worked that out from the unknown count until the core began
    // answering it, which meant the second head had to rediscover it or reassure wrongly.
    //
    // A collision needs no such qualification. Being below the ground somewhere is not made
    // uncertain by not knowing the rest.
    var showsClearance: Bool { hasCollision || (clearanceComplete && !clearanceText.isEmpty) }

    func x(_ point: TerrainPoint, width: Double) -> Double { point.x * width }

    // Which item each stretch of the profile belongs to. Without these the plot answers "the
    // mission is below terrain somewhere" and leaves the operator to find where by eye, which is
    // the one question the panel exists to answer. QGC draws the same ticks and positions them
    // from distanceFromStart; the core already stamps every sample with its sequence, so the
    // marks come from the profile itself rather than from a second measurement that could
    // disagree with it.
    //
    // First sample per item, not every sample: a survey collapses to one sequence carrying four
    // hundred of them, so marking each would redraw the whole strip as a solid rule.
    // The tick sits exactly where the item begins; its number cannot, because a label centred on
    // x=0 renders half outside the plot and item 0 -- always at the very start -- lost its left
    // half every time. Pulled inside by half its own width at both ends, so the first and last
    // marks read as numbers rather than as fragments.
    func labelX(_ marker: TerrainMarker, width: Double, inset: Double) -> Double {
        guard width > inset * 2 else { return width / 2 }
        return min(max(marker.x * width, inset), width - inset)
    }

    var markers: [TerrainMarker] {
        points.reduce(into: [TerrainMarker]()) { found, point in
            guard point.sequence >= 0,
                  !found.contains(where: { $0.sequence == point.sequence }) else { return }
            found.append(TerrainMarker(sequence: point.sequence, x: point.x))
        }
    }

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

