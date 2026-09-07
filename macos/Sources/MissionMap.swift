import AppKit
import MapKit
import SwiftUI

final class MissionAnnotation: NSObject, MKAnnotation {
    let coordinate: CLLocationCoordinate2D
    let title: String?
    let subtitle: String?
    let sequence: Int
    let isCurrent: Bool

    init(item: MissionItem, latitude: Double, longitude: Double) {
        coordinate = CLLocationCoordinate2D(latitude: latitude, longitude: longitude)
        title = "\(item.sequence). \(item.command)"
        subtitle = item.altitudeText == "—" ? nil : item.altitudeText
        sequence = item.sequence
        isCurrent = item.isCurrent
    }
}

final class VehicleAnnotation: NSObject, MKAnnotation {
    let coordinate: CLLocationCoordinate2D
    let title: String? = "Vehicle"

    init(latitude: Double, longitude: Double) {
        coordinate = CLLocationCoordinate2D(latitude: latitude, longitude: longitude)
    }
}

struct MissionMap: NSViewRepresentable {
    let items: [MissionItem]
    let vehicle: (latitude: Double, longitude: Double)?

    func makeNSView(context: Context) -> MKMapView {
        let map = MKMapView()
        map.delegate = context.coordinator

        // Draw QGC's own cached imagery when there is any; Apple's tiles are the
        // fallback, and are useless without a network connection.
        let mapType = CachedTileOverlay.currentMapType()
        if !mapType.isEmpty {
            context.coordinator.overlay = CachedTileOverlay(mapType: mapType)
            map.addOverlay(context.coordinator.overlay!, level: .aboveLabels)
        }
        map.showsCompass = true
        map.showsScale = true
        map.isPitchEnabled = false
        return map
    }

    func updateNSView(_ map: MKMapView, context: Context) {
        map.removeAnnotations(map.annotations)
        // Keep the tile overlay: it is not derived from the mission and rebuilding it
        // every update would discard every tile the map has drawn.
        map.overlays.filter { !($0 is CachedTileOverlay) }.forEach(map.removeOverlay)

        let placed = items.compactMap { item -> MissionAnnotation? in
            guard let latitude = item.latitude, let longitude = item.longitude else { return nil }
            return MissionAnnotation(item: item, latitude: latitude, longitude: longitude)
        }
        map.addAnnotations(placed)

        if let vehicle {
            map.addAnnotation(VehicleAnnotation(latitude: vehicle.latitude, longitude: vehicle.longitude))
        }

        // The route is what makes a list of waypoints a mission: order matters, and a
        // leg that doubles back is obvious on a line and invisible in a table.
        if placed.count > 1 {
            var coordinates = placed.map(\.coordinate)
            map.addOverlay(MKPolyline(coordinates: &coordinates, count: coordinates.count))
        }

        // Frame the mission once. Re-framing on every update would fight the operator
        // the moment they pan.
        MissionMap.lastRender = [
            "items": items.count,
            "placed": placed.count,
            "annotations": map.annotations.count,
            "overlays": map.overlays.count,
            "framed": context.coordinator.hasFramed,
            "centre": ["lat": map.centerCoordinate.latitude, "lon": map.centerCoordinate.longitude],
            "spanLat": map.region.span.latitudeDelta,
            "tileOverlay": context.coordinator.overlay != nil,
            "mapType": CachedTileOverlay.currentMapType(),
            "size": ["w": Double(map.bounds.width), "h": Double(map.bounds.height)],
        ]

        if !context.coordinator.hasFramed, !placed.isEmpty {
            context.coordinator.hasFramed = true
            map.showAnnotations(placed, animated: false)
            // map.camera returns a copy, so mutating its altitude does nothing. Widen
            // the region instead: showAnnotations frames the pins edge to edge, which
            // puts the outermost waypoints under the map's own chrome.
            var region = map.region
            region.span.latitudeDelta *= 1.4
            region.span.longitudeDelta *= 1.4
            map.setRegion(region, animated: false)
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    // What the map actually ended up holding. MapKit draws through Metal, and a locked
    // screen gives its layer a zero-sized drawable, so the map renders nothing even
    // though AppKit and SwiftUI views still capture normally. Without this the two
    // cases -- "never given any data" and "given data, cannot draw" -- look identical.
    static var lastRender: [String: Any] = [:]

    final class Coordinator: NSObject, MKMapViewDelegate {
        var hasFramed = false
        var overlay: CachedTileOverlay?
        var tilesServed = 0
        var tilesMissing = 0

        func mapView(_ mapView: MKMapView, rendererFor overlay: MKOverlay) -> MKOverlayRenderer {
            if let tiles = overlay as? CachedTileOverlay {
                return MKTileOverlayRenderer(tileOverlay: tiles)
            }
            guard let line = overlay as? MKPolyline else { return MKOverlayRenderer(overlay: overlay) }
            let renderer = MKPolylineRenderer(polyline: line)
            renderer.strokeColor = .controlAccentColor
            renderer.lineWidth = 3
            return renderer
        }

        func mapView(_ mapView: MKMapView, viewFor annotation: MKAnnotation) -> MKAnnotationView? {
            if let vehicle = annotation as? VehicleAnnotation {
                let view = mapView.dequeueReusableAnnotationView(withIdentifier: "vehicle")
                    ?? MKAnnotationView(annotation: vehicle, reuseIdentifier: "vehicle")
                view.annotation = vehicle
                view.image = Coordinator.vehicleImage
                view.canShowCallout = true
                return view
            }

            guard let item = annotation as? MissionAnnotation else { return nil }
            let view = mapView.dequeueReusableAnnotationView(withIdentifier: "item") as? MKMarkerAnnotationView
                ?? MKMarkerAnnotationView(annotation: item, reuseIdentifier: "item")
            view.annotation = item
            view.canShowCallout = true
            view.glyphText = String(item.sequence)
            view.markerTintColor = item.isCurrent ? .systemGreen : .controlAccentColor
            return view
        }

        private static let vehicleImage: NSImage = {
            let size = NSSize(width: 14, height: 14)
            let image = NSImage(size: size)
            image.lockFocus()
            NSColor.systemRed.setFill()
            NSBezierPath(ovalIn: NSRect(origin: .zero, size: size)).fill()
            NSColor.white.setStroke()
            let ring = NSBezierPath(ovalIn: NSRect(x: 1, y: 1, width: size.width - 2, height: size.height - 2))
            ring.lineWidth = 2
            ring.stroke()
            image.unlockFocus()
            return image
        }()
    }
}
