import CoreLocation
import MapLibre
import UIKit

typealias MapGestureRecognizer = UIGestureRecognizer
typealias MapGestureDelegate = UIGestureRecognizerDelegate
typealias BareMapView = MLNMapView
let MAP_COLOR_CLASS = "UIColor"

extension MLNMapView {
    func screenPoint(_ coordinate: CLLocationCoordinate2D) -> CGPoint {
        return convert(coordinate, toPointTo: self)
    }

    func screenCoordinate(_ point: CGPoint) -> CLLocationCoordinate2D {
        return convert(point, toCoordinateFrom: self)
    }

    func screenLocation(_ gesture: MapGestureRecognizer) -> CGPoint {
        return gesture.location(in: self)
    }

    func screenFeatures(_ rect: CGRect, _ layers: Set<String>) -> [MLNFeature] {
        return visibleFeatures(in: rect, styleLayerIdentifiers: layers)
    }

    var screenScale: CGFloat {
        return contentScaleFactor
    }

}

