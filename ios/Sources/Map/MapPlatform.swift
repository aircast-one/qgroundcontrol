import CoreLocation
import MapLibre
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

// The map layer's UIKit / AppKit seam. Map code keeps every on-screen point in top-left-origin view points, as UIKit
// and SwiftUI do; AppKit's MLNMapView is unflipped (bottom-left origin), so these helpers flip at the MapLibre
// boundary on macOS. On iOS each one is the plain MapLibre call.

#if os(iOS)
typealias MapGestureRecognizer = UIGestureRecognizer
typealias MapGestureDelegate = UIGestureRecognizerDelegate
let MAP_COLOR_CLASS = "UIColor"
#elseif os(macOS)
typealias MapGestureRecognizer = NSGestureRecognizer
typealias MapGestureDelegate = NSGestureRecognizerDelegate
let MAP_COLOR_CLASS = "NSColor"

private let PRIMARY_BUTTON = 0x1
private let SECONDARY_BUTTON = 0x2
private let MAP_PRESS_SECONDS = 0.5
#endif

extension MLNMapView {
    func screenPoint(_ coordinate: CLLocationCoordinate2D) -> CGPoint {
        #if os(iOS)
        return convert(coordinate, toPointTo: self)
        #elseif os(macOS)
        return flippedPoint(convert(coordinate, toPointTo: self))
        #endif
    }

    func screenCoordinate(_ point: CGPoint) -> CLLocationCoordinate2D {
        #if os(iOS)
        return convert(point, toCoordinateFrom: self)
        #elseif os(macOS)
        return convert(flippedPoint(point), toCoordinateFrom: self)
        #endif
    }

    func screenLocation(_ gesture: MapGestureRecognizer) -> CGPoint {
        #if os(iOS)
        return gesture.location(in: self)
        #elseif os(macOS)
        return flippedPoint(gesture.location(in: self))
        #endif
    }

    func screenFeatures(_ rect: CGRect, _ layers: Set<String>) -> [MLNFeature] {
        #if os(iOS)
        return visibleFeatures(in: rect, styleLayerIdentifiers: layers)
        #elseif os(macOS)
        // The macOS SDK names the plain rect query visibleFeatures(at:styleLayerIdentifiers:); the predicate form is unambiguous.
        return visibleFeatures(
            in: CGRect(x: rect.minX, y: bounds.height - rect.maxY, width: rect.width, height: rect.height),
            styleLayerIdentifiers: layers,
            predicate: nil
        )
        #endif
    }

    // Device pixels per view point, for the pixel-sized hit and slop radii.
    var screenScale: CGFloat {
        #if os(iOS)
        return contentScaleFactor
        #elseif os(macOS)
        return window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1
        #endif
    }

    #if os(macOS)
    func screenLocation(_ event: NSEvent) -> CGPoint {
        flippedPoint(convert(event.locationInWindow, from: nil))
    }

    private func flippedPoint(_ point: CGPoint) -> CGPoint {
        isFlipped ? point : CGPoint(x: point.x, y: bounds.height - point.y)
    }
    #endif
}

#if os(macOS)
extension MLNMapView {
    // iOS's setCenter(_:zoomLevel:animated:), which the AppKit SDK lacks. A camera's altitude for a zoom level scales
    // with 2^-zoom and the cosine of its latitude, so the animated form moves both in one camera change.
    func setCenter(_ coordinate: CLLocationCoordinate2D, zoomLevel: Double, animated: Bool) {
        guard animated else {
            self.zoomLevel = zoomLevel
            centerCoordinate = coordinate
            return
        }
        let target = camera
        let fromCos = max(cos(target.centerCoordinate.latitude * .pi / 180), 1e-9)
        target.altitude *= pow(2, self.zoomLevel - zoomLevel) * cos(coordinate.latitude * .pi / 180) / fromCos
        target.centerCoordinate = coordinate
        setCamera(target, animated: true)
    }
}

// A recognizer that owns the press it began (a handle drag), so the map's clicks and holds yield to it.
protocol MapExclusiveGesture: AnyObject {}

// The map's added clicks and holds run alongside MLNMapView's own (pan, clicks, magnify), as the iOS ones do.
final class MapSimultaneousGestures: NSObject, NSGestureRecognizerDelegate {
    static let shared = MapSimultaneousGestures()

    func gestureRecognizer(_ gestureRecognizer: NSGestureRecognizer, shouldRecognizeSimultaneouslyWith other: NSGestureRecognizer) -> Bool {
        !(other is MapExclusiveGesture)
    }
}

// A primary click that waits for MLNMapView's double-click zoom to fail, as the iOS taps require(toFail:) it.
final class MapClickRecognizer: NSClickGestureRecognizer {
    override init(target: Any?, action: Selector?) {
        super.init(target: target, action: action)
        buttonMask = PRIMARY_BUTTON
        delaysPrimaryMouseButtonEvents = false
        delegate = MapSimultaneousGestures.shared
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
    }

    override func shouldRequireFailure(of otherGestureRecognizer: NSGestureRecognizer) -> Bool {
        guard let click = otherGestureRecognizer as? NSClickGestureRecognizer else { return false }
        return click.numberOfClicksRequired == 2 && click.buttonMask & PRIMARY_BUTTON != 0
    }
}

// A secondary (right / two-finger) click, which stands in for the iOS long press.
func secondaryClickRecognizer(_ target: Any, _ action: Selector) -> NSClickGestureRecognizer {
    let click = NSClickGestureRecognizer(target: target, action: action)
    click.buttonMask = SECONDARY_BUTTON
    click.delaysSecondaryMouseButtonEvents = false
    click.delegate = MapSimultaneousGestures.shared
    return click
}

// Click-and-hold, the mouse form of the iOS long press.
func mapPressRecognizer(_ target: Any, _ action: Selector) -> NSPressGestureRecognizer {
    let press = NSPressGestureRecognizer(target: target, action: action)
    press.buttonMask = PRIMARY_BUTTON
    press.minimumPressDuration = MAP_PRESS_SECONDS
    press.delaysPrimaryMouseButtonEvents = false
    press.delegate = MapSimultaneousGestures.shared
    return press
}
#endif
