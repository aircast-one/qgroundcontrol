import Foundation

struct ObstacleWarning: Equatable {
    let label: String
    let close: Bool
}

func sensorSaidStale(_ view: JSON) -> Bool { !view["stale"].isNull && view["stale"].bool }

func obstacleWarning(_ view: JSON?) -> ObstacleWarning? {
    guard let view, view["available"].bool else { return nil }
    let nearest = view["nearest"]
    guard nearest.object != nil, !sensorSaidStale(nearest) else { return nil }
    let distance = nearest["distanceText"].string
    guard !distance.isBlank else { return nil }
    let sector = nearest["sectorText"].string
    return ObstacleWarning(
        label: [distance, sector].filter { !$0.isBlank }.joined(separator: " "),
        close: nearest["close"].bool
    )
}

struct ObstacleSample: Equatable {
    let bearingDegrees: Double
    let metres: Double
}

struct ObstacleRing: Equatable {
    let samples: [ObstacleSample]
    let maxMetres: Double
    let incrementDegrees: Double
    let floorMetres: Double
    let stale: Bool
}

private func positive(_ value: Double) -> Double? { value.isFinite && value > 0 ? value : nil }

func obstacleRing(_ view: JSON?) -> ObstacleRing? {
    guard let view, view["available"].bool, let ring = view["ringMetres"].arrayOrNil,
          let increment = positive(view["ringIncrement"].double(.nan)),
          let ceiling = positive(view["rangeMaxMetres"].double(.nan)) else { return nil }
    let offset = view["ringOffset"].double(0).isFinite ? view["ringOffset"].double(0) : 0
    let samples = ring.enumerated().compactMap { index, value -> ObstacleSample? in
        let metres = value.isNull ? .nan : value.double(.nan)
        return metres.isFinite && metres >= 0 ? ObstacleSample(bearingDegrees: offset + Double(index) * increment, metres: metres) : nil
    }
    let floor = positive(view["rangeMinMetres"].double(.nan)) ?? 0
    return samples.isEmpty ? nil : ObstacleRing(samples: samples, maxMetres: ceiling, incrementDegrees: increment, floorMetres: floor, stale: sensorSaidStale(view))
}
