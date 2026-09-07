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
        map.overlays.filter { !($0 is CachedTileOverlay) }.forEach(map.removeOverlay)

        let placed = items.compactMap { item -> MissionAnnotation? in
            guard let latitude = item.latitude, let longitude = item.longitude else { return nil }
            return MissionAnnotation(item: item, latitude: latitude, longitude: longitude)
        }
        map.addAnnotations(placed)

        if let vehicle {
            map.addAnnotation(VehicleAnnotation(latitude: vehicle.latitude, longitude: vehicle.longitude))
        }

        if placed.count > 1 {
            var coordinates = placed.map(\.coordinate)
            map.addOverlay(MKPolyline(coordinates: &coordinates, count: coordinates.count))
        }

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
            let region = MissionMap.region(enclosing: placed.map(\.coordinate))
            guard map.bounds.width > 0 else {
                DispatchQueue.main.async { [weak map] in
                    guard let map, !context.coordinator.hasFramed, map.bounds.width > 0 else { return }
                    context.coordinator.hasFramed = true
                    map.setRegion(region, animated: false)
                }
                return
            }
            context.coordinator.hasFramed = true
            map.setRegion(region, animated: false)
            MissionMap.lastRender["framedSpanLat"] = map.region.span.latitudeDelta
            MissionMap.lastRender["framedCentre"] =
                ["lat": map.centerCoordinate.latitude, "lon": map.centerCoordinate.longitude]
        }
    }

    static func region(enclosing coordinates: [CLLocationCoordinate2D]) -> MKCoordinateRegion {
        let frame = MapFrame(latitudes: coordinates.map(\.latitude),
                             longitudes: coordinates.map(\.longitude))
        return MKCoordinateRegion(
            center: CLLocationCoordinate2D(latitude: frame.centreLatitude,
                                           longitude: frame.centreLongitude),
            span: MKCoordinateSpan(latitudeDelta: frame.latitudeDelta,
                                   longitudeDelta: frame.longitudeDelta))
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

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
