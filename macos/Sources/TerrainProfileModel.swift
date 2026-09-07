import Foundation

struct TerrainPoint: Equatable {
    let distance: Double
    let missionAltitude: Double
    let terrainAltitude: Double?
    let collision: Bool
}

struct TerrainProfile: Equatable {
    let points: [TerrainPoint]
    let minAltitude: Double
    let maxAltitude: Double
    let totalDistance: Double
    let unknownTerrain: Int

    static let empty = TerrainProfile(points: [], minAltitude: 0, maxAltitude: 0,
                                      totalDistance: 0, unknownTerrain: 0)

    init(points: [TerrainPoint]) {
        self.points = points
        unknownTerrain = points.filter { $0.terrainAltitude == nil }.count
        totalDistance = points.map(\.distance).max() ?? 0

        let altitudes = points.map(\.missionAltitude) + points.compactMap(\.terrainAltitude)
        let low = altitudes.min() ?? 0
        let high = altitudes.max() ?? 0
        // A flat mission over flat ground has no range at all, which would divide by zero
        // when placing a point; give it a band so the lines stay inside the plot.
        let padding = max((high - low) * 0.2, 5)
        minAltitude = low - padding
        maxAltitude = high + padding
    }

    private init(points: [TerrainPoint], minAltitude: Double, maxAltitude: Double,
                 totalDistance: Double, unknownTerrain: Int) {
        self.points = points
        self.minAltitude = minAltitude
        self.maxAltitude = maxAltitude
        self.totalDistance = totalDistance
        self.unknownTerrain = unknownTerrain
    }

    var hasCollision: Bool { points.contains(where: \.collision) }

    var usable: Bool { points.count > 1 && maxAltitude > minAltitude }

    // Drawing the ground across only the points that have terrain puts a line under part
    // of the mission and nothing under the rest, which reads as ground that stops.
    var groundKnown: Bool { unknownTerrain == 0 && points.count > 1 }

    func x(_ point: TerrainPoint, width: Double) -> Double {
        guard totalDistance > 0 else { return 0 }
        return point.distance / totalDistance * width
    }

    func y(_ altitude: Double, height: Double) -> Double {
        let span = maxAltitude - minAltitude
        guard span > 0 else { return height / 2 }
        return height - (altitude - minAltitude) / span * height
    }
}
