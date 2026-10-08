import Foundation

let TRACK_VIEW = "view.track"
let TRACK_TAIL_VIEW = "view.track(tail)"

struct TrackReading: Equatable {
    var points: [TrackPoint]
    var count: Int
    var available: Bool
    var vehicleId: Int64? = nil
    var generation: Int64 = 0
    var from: Int = 0
}

func trackReading(_ view: JSON?) -> TrackReading {
    let listed = view?["points"].arrayOrNil ?? []
    return TrackReading(
        points: listed.map { point in TrackPoint(latitude: point["latitude"].double(.nan), longitude: point["longitude"].double(.nan)) },
        count: view?["count"].int(0) ?? 0,
        available: view?["available"].bool == true,
        vehicleId: view.flatMap { $0.has("vehicleId") ? $0["vehicleId"].int64 : nil },
        generation: view?["generation"].int64 ?? 0,
        from: view?["from"].int(0) ?? 0
    )
}

func mergedTrack(_ held: TrackReading?, _ tail: TrackReading) -> TrackReading? {
    if tail.from == 0 { return tail }
    guard let held, held.from == 0, held.vehicleId == tail.vehicleId, held.generation == tail.generation,
          held.points.count >= tail.from else { return nil }
    return withChanges(tail) {
        $0.points = Array(held.points.prefix(tail.from)) + tail.points
        $0.from = 0
    }
}

func plottedTrack(_ reading: TrackReading) -> [TrackPoint] { reading.points.filter { isPlottable($0.latitude, $0.longitude) } }

func trackDraws(_ reading: TrackReading) -> Bool { reading.available && plottedTrack(reading).count > 1 }
