import MapLibre
import UIKit

let GIMBAL_AZIMUTH_VIEW = "view.gimbalAzimuth"
private let GIMBAL_SOURCE = "aircast-gimbal-azimuth"
let GIMBAL_LAYER = "aircast-gimbal-azimuth-layer"
private let GIMBAL_IMAGE = "aircast-gimbal-wedge"
private let YAW_PROPERTY = "yaw"
private let OPACITY_PROPERTY = "opacity"
private let ACTIVE_OPACITY = 1.0
private let OTHER_OPACITY = 0.4
private let WEDGE_SIZE_PX: CGFloat = 48

struct GimbalAzimuth: Equatable {
    var yaw: Double
    var active: Bool

    static func == (lhs: GimbalAzimuth, rhs: GimbalAzimuth) -> Bool {
        (lhs.yaw == rhs.yaw || (lhs.yaw.isNaN && rhs.yaw.isNaN)) && lhs.active == rhs.active
    }
}

func gimbalAzimuths(_ view: JSON?) -> [GimbalAzimuth] {
    (view?["gimbals"].arrayOrNil ?? [])
        .filter { $0.object != nil }
        .map { GimbalAzimuth(yaw: $0["yaw"].double(.nan), active: $0["active"].bool) }
}

enum GimbalBridge {
    static func read() -> [GimbalAzimuth] { gimbalAzimuths(Qgc.get(GIMBAL_AZIMUTH_VIEW)) }
}

func gimbalFeatures(_ latitude: Double, _ longitude: Double, _ gimbals: [GimbalAzimuth]) -> FeatureCollection {
    featureCollection(
        gimbals.filter { isPlottable(latitude, longitude) && !$0.yaw.isNaN }.map { gimbal in
            pointFeature(
                TrackPoint(latitude: latitude, longitude: longitude),
                attributes: [YAW_PROPERTY: gimbal.yaw, OPACITY_PROPERTY: gimbal.active ? ACTIVE_OPACITY : OTHER_OPACITY]
            )
        }
    )
}

private func wedge() -> UIImage {
    let size = WEDGE_SIZE_PX
    return mapIcon(size) { context in
        let path = CGMutablePath()
        path.move(to: CGPoint(x: size / 2, y: size))
        path.addLine(to: CGPoint(x: 0, y: 0))
        path.addLine(to: CGPoint(x: size / 2, y: size * 0.25))
        path.addLine(to: CGPoint(x: size, y: 0))
        path.closeSubpath()
        context.addPath(path)
        context.clip()
        let colours = [UIColor(white: 1, alpha: 0).cgColor, UIColor(white: 1, alpha: 128.0 / 255).cgColor] as CFArray
        guard let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colours, locations: [0, 1]) else { return }
        context.drawLinearGradient(gradient, start: .zero, end: CGPoint(x: 0, y: size), options: [])
    }
}

func installGimbalLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: GIMBAL_SOURCE) == nil else { return }
    let source = geoJsonSource(GIMBAL_SOURCE)
    style.addSource(source)
    style.setImage(wedge(), forName: GIMBAL_IMAGE)
    let layer = MLNSymbolStyleLayer(identifier: GIMBAL_LAYER, source: source)
    layer.iconImageName = styleConstant(GIMBAL_IMAGE)
    layer.iconAnchor = styleConstant("bottom")
    layer.iconRotation = styleGet(YAW_PROPERTY)
    layer.iconRotationAlignment = styleConstant("map")
    layer.iconOpacity = styleGet(OPACITY_PROPERTY)
    layer.iconAllowsOverlap = styleConstant(true)
    layer.iconIgnoresPlacement = styleConstant(true)
    style.addLayer(layer)
}

func renderGimbals(_ style: MLNStyle, _ latitude: Double, _ longitude: Double, _ gimbals: [GimbalAzimuth]) {
    style.setGeoJson(GIMBAL_SOURCE, gimbalFeatures(latitude, longitude, gimbals))
}
