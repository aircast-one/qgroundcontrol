import MapLibre

let SYNTHETIC_VIEW_PATH = "view.syntheticView"
private let BEAM_SOURCE = "aircast-camera-beam"
private let BEAM_FILL_LAYER = "aircast-camera-beam-fill"
private let BEAM_LINE_LAYER = "aircast-camera-beam-line"
private let BEAM_COLOUR = "#FFE9A6"
private let BEAM_FILL_OPACITY = 0.28
private let BEAM_LINE_OPACITY = 0.6
private let BEAM_LINE_WIDTH = 1.5
private let POLYGON_POINTS = 4

func cameraBeam(_ view: JSON?) -> [TrackPoint] {
    guard let view, view["available"].bool else { return [] }
    return view["beam"].array.map { TrackPoint(latitude: $0["latitude"].double ?? .nan, longitude: $0["longitude"].double ?? .nan) }
}

func beamFeatures(_ beam: [TrackPoint]) -> FeatureCollection {
    let drawable = beam.count >= POLYGON_POINTS && beam.allSatisfy { isPlottable($0.latitude, $0.longitude) }
    return featureCollection(drawable ? [polygonFeature(beam)] : [])
}

func installCameraBeamLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: BEAM_SOURCE) == nil else { return }
    let source = geoJsonSource(BEAM_SOURCE)
    style.addSource(source)
    let fill = MLNFillStyleLayer(identifier: BEAM_FILL_LAYER, source: source)
    fill.fillColor = styleConstant(mapColour(BEAM_COLOUR))
    fill.fillOpacity = styleConstant(BEAM_FILL_OPACITY)
    style.addLayer(fill)
    let line = MLNLineStyleLayer(identifier: BEAM_LINE_LAYER, source: source)
    line.lineColor = styleConstant(mapColour(BEAM_COLOUR))
    line.lineOpacity = styleConstant(BEAM_LINE_OPACITY)
    line.lineWidth = styleConstant(BEAM_LINE_WIDTH)
    style.addLayer(line)
}

func renderCameraBeam(_ style: MLNStyle, _ beam: [TrackPoint]) {
    style.setGeoJson(BEAM_SOURCE, beamFeatures(beam))
}
