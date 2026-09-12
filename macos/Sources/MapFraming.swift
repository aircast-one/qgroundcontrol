import Foundation

// Pressing the same Centre choice twice builds an IDENTICAL MapFrame, and the map re-centres only
// when the value it is handed CHANGES -- so after the operator panned away, a second press of the
// same choice did nothing at all. The serial makes every request distinct without the map having
// to know why.
struct MapFocus: Equatable {
    let frame: MapFrame
    let serial: Int

    static func next(after previous: MapFocus?, to frame: MapFrame) -> MapFocus {
        MapFocus(frame: frame, serial: (previous?.serial ?? 0) + 1)
    }
}

struct MapFrame: Equatable {
    let centreLatitude: Double
    let centreLongitude: Double
    let latitudeDelta: Double
    let longitudeDelta: Double

    static let minimumDelta = 0.004
    static let padding = 1.6

    static func normalisedLongitude(_ longitude: Double) -> Double {
        let shifted = (longitude + 180).truncatingRemainder(dividingBy: 360)
        return (shifted < 0 ? shifted + 360 : shifted) - 180
    }

    // Longitude wraps, so the points are an arc on a circle rather than a range on a line. The
    // arc they occupy is everything except the widest empty gap between neighbours.
    static func longitudeArc(_ values: [Double]) -> (centre: Double, span: Double) {
        let sorted = values.sorted()
        guard let first = sorted.first, let last = sorted.last else { return (0, 0) }
        let gaps = zip(sorted.dropFirst(), sorted).map { $0 - $1 } + [first + 360 - last]
        guard let widest = gaps.enumerated().max(by: { $0.element < $1.element })
        else { return (0, 0) }
        let span = 360 - widest.element
        let start = sorted[(widest.offset + 1) % sorted.count]
        return (normalisedLongitude(start + span / 2), span)
    }

    init(latitudes: [Double], longitudes: [Double]) {
        let finiteLatitudes = latitudes.filter { $0.isFinite && abs($0) <= 90 }
        let finiteLongitudes = longitudes.filter { $0.isFinite && abs($0) <= 180 }

        let minLatitude = finiteLatitudes.min() ?? 0
        let maxLatitude = finiteLatitudes.max() ?? 0
        let arc = MapFrame.longitudeArc(finiteLongitudes)

        centreLatitude = (minLatitude + maxLatitude) / 2
        centreLongitude = arc.centre
        latitudeDelta = min(max((maxLatitude - minLatitude) * MapFrame.padding,
                                MapFrame.minimumDelta), 180)
        longitudeDelta = min(max(arc.span * MapFrame.padding, MapFrame.minimumDelta), 360)
    }
}
