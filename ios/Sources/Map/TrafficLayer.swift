import MapLibre
import UIKit

let TRAFFIC_SOURCE = "traffic"
let TRAFFIC_LAYER = "traffic-aircraft"
private let ALERT_IMAGE = "traffic-alert"
private let AWARENESS_IMAGE = "traffic-awareness"
private let HEADING = "heading"
private let ALERT = "alert"
private let LABEL = "label"
private let AIRCRAFT_PX: CGFloat = 44

struct TrafficMark: Equatable {
    var latitude: Double
    var longitude: Double
    var heading: Double?
    var alert: Bool
    var label: String
}

private func number(_ json: JSON, _ key: String) -> Double? {
    json[key].isNull ? nil : json[key].double.flatMap { $0.isNaN ? nil : $0 }
}

func trafficLabel(_ altitude: Double?, _ unit: String, _ callsign: String) -> String {
    altitude.map {
        (String(format: "%.0f", locale: Locale(identifier: "en_US_POSIX"), $0) + " \(unit)\n\(callsign)")
            .replacingOccurrences(of: "\\s+$", with: "", options: .regularExpression)
    } ?? ""
}

func trafficMarks(_ view: JSON?) -> [TrafficMark] {
    guard let contacts = view?["contacts"].arrayOrNil else { return [] }
    let unit = view?["units"]["altitude"].string ?? ""
    return contacts.filter { $0.object != nil }.compactMap { contact in
        guard let latitude = number(contact, "latitude"), let longitude = number(contact, "longitude"), isPlottable(latitude, longitude) else { return nil }
        return TrafficMark(
            latitude: latitude,
            longitude: longitude,
            heading: number(contact, "headingDegrees"),
            alert: contact["alert"].bool,
            label: trafficLabel(number(contact, "altitude"), unit, contact["callsign"].string)
        )
    }
}

enum TrafficBridge {
    static func read() -> [TrafficMark] { trafficMarks(Qgc.get(TRAFFIC_VIEW)) }
}

func trafficFeatures(_ marks: [TrafficMark]) -> FeatureCollection {
    featureCollection(marks.map { mark in
        pointFeature(TrackPoint(latitude: mark.latitude, longitude: mark.longitude), attributes: [HEADING: mark.heading ?? 0, ALERT: mark.alert, LABEL: mark.label])
    })
}

private func aircraft(_ fill: UIColor) -> UIImage {
    mapIcon(AIRCRAFT_PX) { context in
        let s = AIRCRAFT_PX
        let path = CGMutablePath()
        path.addLines(between: [
            CGPoint(x: s * 0.5, y: s * 0.06), CGPoint(x: s * 0.58, y: s * 0.42), CGPoint(x: s * 0.94, y: s * 0.58),
            CGPoint(x: s * 0.58, y: s * 0.6), CGPoint(x: s * 0.56, y: s * 0.82), CGPoint(x: s * 0.7, y: s * 0.92),
            CGPoint(x: s * 0.3, y: s * 0.92), CGPoint(x: s * 0.44, y: s * 0.82), CGPoint(x: s * 0.42, y: s * 0.6),
            CGPoint(x: s * 0.06, y: s * 0.58), CGPoint(x: s * 0.42, y: s * 0.42),
        ])
        path.closeSubpath()
        context.addPath(path)
        context.setStrokeColor(UIColor.black.cgColor)
        context.setLineWidth(4)
        context.setLineJoin(.round)
        context.strokePath()
        context.addPath(path)
        context.setFillColor(fill.cgColor)
        context.fillPath()
    }
}

func installTrafficLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: TRAFFIC_SOURCE) == nil else { return }
    style.setImage(aircraft(mapColour("#FF5252")), forName: ALERT_IMAGE)
    style.setImage(aircraft(.white), forName: AWARENESS_IMAGE)
    let source = geoJsonSource(TRAFFIC_SOURCE)
    style.addSource(source)
    let layer = MLNSymbolStyleLayer(identifier: TRAFFIC_LAYER, source: source)
    layer.iconImageName = styleExpression(["case", ["get", ALERT], ALERT_IMAGE, AWARENESS_IMAGE])
    layer.iconRotation = styleGet(HEADING)
    layer.iconRotationAlignment = styleConstant("map")
    layer.iconAllowsOverlap = styleConstant(true)
    layer.iconIgnoresPlacement = styleConstant(true)
    layer.text = styleGet(LABEL)
    layer.textFontSize = styleConstant(11)
    layer.textColor = styleConstant(UIColor.white)
    layer.textHaloColor = styleConstant(UIColor.black)
    layer.textHaloWidth = styleConstant(1.5)
    layer.textAnchor = styleConstant("top")
    layer.textOffset = styleOffset(0, 1.6)
    layer.textAllowsOverlap = styleConstant(false)
    layer.textOptional = styleConstant(true)
    if let label = style.layer(withIdentifier: VEHICLE_LABEL_LAYER) { style.insertLayer(layer, below: label) } else { style.addLayer(layer) }
}
