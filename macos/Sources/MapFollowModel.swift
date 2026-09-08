import Foundation

struct MapPoint: Equatable {
    let x: Double
    let y: Double
}

struct MapRect: Equatable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double

    func contains(_ point: MapPoint) -> Bool {
        point.x >= x && point.x < x + width && point.y >= y && point.y < y + height
    }
}

struct MapInsets: Equatable {
    let top: Double
    let left: Double
    let bottom: Double
    let right: Double

    static let none = MapInsets(top: 0, left: 0, bottom: 0, right: 0)
}

enum MapFollow {
    static func centreRect(width: Double, height: Double, insets: MapInsets) -> MapRect? {
        let usableWidth = width - insets.left - insets.right
        let usableHeight = height - insets.top - insets.bottom
        guard usableWidth > 0, usableHeight > 0 else { return nil }
        return MapRect(x: insets.left, y: insets.top, width: usableWidth, height: usableHeight)
    }

    static func needsRecentre(vehicle: MapPoint?, width: Double, height: Double,
                              insets: MapInsets) -> Bool {
        guard let vehicle, let rect = centreRect(width: width, height: height, insets: insets),
              vehicle.x.isFinite, vehicle.y.isFinite else { return false }
        return !rect.contains(vehicle)
    }

    static func offset(width: Double, height: Double, insets: MapInsets) -> MapPoint? {
        guard let rect = centreRect(width: width, height: height, insets: insets) else { return nil }
        return MapPoint(x: width / 2 - (rect.x + rect.width / 2),
                        y: height / 2 - (rect.y + rect.height / 2))
    }

    static func follows(setting: Bool, tracking: Bool) -> Bool { setting && tracking }

    static func nudges(setting: Bool, tracking: Bool) -> Bool { !setting && tracking }
}
