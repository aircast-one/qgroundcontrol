import Foundation

struct MapFrame: Equatable {
    let centreLatitude: Double
    let centreLongitude: Double
    let latitudeDelta: Double
    let longitudeDelta: Double

    static let minimumDelta = 0.004
    static let padding = 1.6

    init(latitudes: [Double], longitudes: [Double]) {
        let finiteLatitudes = latitudes.filter { $0.isFinite && abs($0) <= 90 }
        let finiteLongitudes = longitudes.filter { $0.isFinite && abs($0) <= 180 }

        let minLatitude = finiteLatitudes.min() ?? 0
        let maxLatitude = finiteLatitudes.max() ?? 0
        let minLongitude = finiteLongitudes.min() ?? 0
        let maxLongitude = finiteLongitudes.max() ?? 0

        centreLatitude = (minLatitude + maxLatitude) / 2
        centreLongitude = (minLongitude + maxLongitude) / 2
        latitudeDelta = max((maxLatitude - minLatitude) * MapFrame.padding, MapFrame.minimumDelta)
        longitudeDelta = max((maxLongitude - minLongitude) * MapFrame.padding, MapFrame.minimumDelta)
    }

    var isUsable: Bool {
        centreLatitude.isFinite && centreLongitude.isFinite
            && latitudeDelta <= 180 && longitudeDelta <= 360
    }
}
