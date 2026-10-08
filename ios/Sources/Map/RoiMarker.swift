import MapLibre
import UIKit

let ROI_GUIDED_VIEW = "view.guidedActions"
private let ROI_SOURCE = "aircast-roi"
let ROI_LAYER = "aircast-roi-layer"
private let ROI_LABEL_LAYER = "aircast-roi-label"
private let ROI_COLOUR = "#FF9500"

func roiPoint(_ view: JSON?) -> TrackPoint? {
    guard let roi = view?["roi"], roi.object != nil else { return nil }
    let point = TrackPoint(latitude: roi["latitude"].double(.nan), longitude: roi["longitude"].double(.nan))
    return isPlottable(point.latitude, point.longitude) ? point : nil
}

enum RoiBridge {
    static func read() -> TrackPoint? { roiPoint(Qgc.get(ROI_GUIDED_VIEW)) }
}

func installRoiLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: ROI_SOURCE) == nil else { return }
    let source = geoJsonSource(ROI_SOURCE)
    style.addSource(source)
    let dot = MLNCircleStyleLayer(identifier: ROI_LAYER, source: source)
    dot.circleColor = styleConstant(mapColour(ROI_COLOUR))
    dot.circleRadius = styleConstant(10)
    dot.circleStrokeColor = styleConstant(UIColor.white)
    dot.circleStrokeWidth = styleConstant(2)
    style.addLayer(dot)
    let label = MLNSymbolStyleLayer(identifier: ROI_LABEL_LAYER, source: source)
    label.text = styleConstant("ROI here")
    label.textFontSize = styleConstant(12)
    label.textColor = styleConstant(UIColor.white)
    label.textOffset = styleOffset(0, 1.6)
    label.textAnchor = styleConstant("top")
    label.textAllowsOverlap = styleConstant(true)
    label.textIgnoresPlacement = styleConstant(true)
    style.addLayer(label)
}

func renderRoi(_ style: MLNStyle, _ roi: TrackPoint?) {
    style.setGeoJson(ROI_SOURCE, featureCollection([roi].compactMap { $0 }.map { pointFeature($0) }))
}
