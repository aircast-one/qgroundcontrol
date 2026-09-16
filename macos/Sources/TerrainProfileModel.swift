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
    let sequences: [Int]
    let x: Double

    var id: Int { sequences.first ?? -1 }

    // An en dash is range notation, and these sequences share an x rather than a run: the
    // populated plan groups 0, 1, 3 and 107 at the launch point, which the old label spelled
    // "0-107" -- a claim about a hundred and eight items, two of which are elsewhere on the axis.
    var label: String {
        TerrainMarker.spans(sequences).joined(separator: ", ")
    }

    static func spans(_ sequences: [Int]) -> [String] {
        sequences.sorted()
            .reduce(into: [[Int]]()) { runs, sequence in
                guard let end = runs.last?.last, sequence == end + 1 else {
                    return runs.append([sequence])
                }
                runs[runs.count - 1].append(sequence)
            }
            .compactMap { run in
                guard let first = run.first, let last = run.last else { return nil }
                return first == last ? "\(first)" : "\(first)\u{2013}\(last)"
            }
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
    let clearanceText: String
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
        clearanceComplete = flag("clearanceComplete")
        lowestText = text("lowestText")
        highestText = text("highestText")
    }

    static let below = "Mission is below terrain"
    static let clears = "Clears terrain"

    // Drawn only when the clearance sentence is not, so this is the profile's second-choice line
    // and the one an operator reads when the panel cannot answer the question it exists for.
    // nil rather than an empty string: no points missing is not a sentence with nothing in it.
    var unknownSentence: String? {
        guard unknownTerrain > 0 else { return nil }
        return "\(unknownTerrain) point" + (unknownTerrain == 1 ? "" : "s") + " without terrain data"
    }

    var clearanceSentence: String {
        guard !clearanceText.isEmpty else { return hasCollision ? TerrainProfile.below : "" }
        return hasCollision
            ? "\(TerrainProfile.below) by up to \(clearanceText)"
            : "\(TerrainProfile.clears) by \(clearanceText)"
    }

    var showsClearance: Bool { hasCollision || (clearanceComplete && !clearanceText.isEmpty) }

    func x(_ point: TerrainPoint, width: Double) -> Double { point.x * width }

    func labelX(_ marker: TerrainMarker, width: Double, inset: Double) -> Double {
        guard width > inset * 2 else { return width / 2 }
        return min(max(marker.x * width, inset), width - inset)
    }

    var markers: [TerrainMarker] {
        let firsts = points.reduce(into: [(sequence: Int, x: Double)]()) { found, point in
            guard point.sequence >= 0,
                  !found.contains(where: { $0.sequence == point.sequence }) else { return }
            found.append((point.sequence, point.x))
        }
        return firsts.reduce(into: [TerrainMarker]()) { grouped, entry in
            guard let last = grouped.last, last.x == entry.x else {
                return grouped.append(TerrainMarker(sequences: [entry.sequence], x: entry.x))
            }
            grouped[grouped.count - 1] = TerrainMarker(sequences: last.sequences + [entry.sequence],
                                                       x: last.x)
        }
    }

    var groundRuns: [[TerrainPoint]] {
        points.reduce(into: (runs: [[TerrainPoint]](), open: false)) { state, point in
            guard point.terrainAltitude != nil else { return state.open = false }
            if state.open {
                state.runs[state.runs.count - 1].append(point)
            } else {
                state.runs.append([point])
                state.open = true
            }
        }.runs.filter { $0.count > 1 }
    }

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

