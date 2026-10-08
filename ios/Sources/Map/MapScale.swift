import Foundation

private let EQUATOR_METRES = 40_075_016.686
private let TILE_PIXELS = 512.0

struct MapScale: Equatable {
    var text: String
    var pixels: Double
}

func metresPerPixel(_ latitude: Double, _ zoom: Double) -> Double {
    EQUATOR_METRES * cos(latitude * .pi / 180) / (TILE_PIXELS * pow(2.0, zoom))
}

func metresAcross(_ latitude: Double, _ zoom: Double, _ maxPixels: Double) -> Double? {
    let perPixel = metresPerPixel(latitude, zoom)
    guard perPixel.isFinite, perPixel > 0, maxPixels > 0 else { return nil }
    return perPixel * maxPixels
}

func mapScaleBar(_ view: JSON?, _ maxPixels: Double) -> MapScale? {
    guard let view, view["available"].bool else { return nil }
    let fraction = view["fraction"].double(.nan)
    guard fraction.isFinite, fraction > 0 else { return nil }
    let text = view["text"].string
    guard !text.isBlank else { return nil }
    return MapScale(text: text, pixels: fraction * maxPixels)
}
