import SwiftUI

let OBSTACLE_VIEW = "view.obstacle"
private let SEGMENTS = 16
private let LEVEL_METRES = 10.0
private let SEGMENT_GAP_RAD = 0.03
private let TEXT_STEP_METRES = 2.0
private let MAP_TEXT: CGFloat = 22
private let VIDEO_TEXT: CGFloat = 10
let TRUE_SCALE_PROBE: CGFloat = 100
let TRUE_SCALE_BELOW_METRES = 4.0

struct ObstacleOverlay: Equatable {
    var ranges: [Double]
    var texts: [String]
    var increment: Double
    var offset: Double
    var maxMetres: Double
}

func obstacleOverlay(_ view: JSON?) -> ObstacleOverlay? {
    guard let drawn = view?["overlay"], drawn.object != nil, let ranges = drawn["ranges"].arrayOrNil else { return nil }
    let texts = drawn["texts"].arrayOrNil
    let increment = view?["ringIncrement"].double(.nan) ?? .nan
    let maxMetres = drawn["maxMetres"].double(.nan)
    guard increment.isFinite, increment > 0, maxMetres.isFinite, maxMetres > 0, !ranges.isEmpty else { return nil }
    let offset = view?["ringOffset"].double(0) ?? 0
    return ObstacleOverlay(
        ranges: ranges.map { $0.double(.nan) },
        texts: ranges.indices.map { index in texts.map { $0.indices.contains(index) ? $0[index].string : "" } ?? "" },
        increment: increment,
        offset: offset.isFinite ? offset : 0,
        maxMetres: maxMetres
    )
}

private func floorMod(_ value: Double, _ divisor: Double) -> Double {
    let remainder = value.truncatingRemainder(dividingBy: divisor)
    return remainder < 0 ? remainder + divisor : remainder
}

private func floorMod(_ value: Int, _ divisor: Int) -> Int { ((value % divisor) + divisor) % divisor }

func rangeIndex(_ degrees: Double, _ overlay: ObstacleOverlay, _ heading: Double) -> Int {
    let count = overlay.ranges.count
    let wrapped = floorMod(degrees - overlay.offset - heading, 360)
    guard count > 0, let step = Int(exactly: ceil(wrapped / overlay.increment)) else { return 0 }
    return floorMod(step, count)
}

struct GradientStop: Equatable {
    var at: CGFloat
    var colour: Color
}

private let MAP_STOPS = [
    GradientStop(at: 0, colour: Color(.sRGB, red: 1, green: 0, blue: 0, opacity: 1)),
    GradientStop(at: 0.1, colour: Color(.sRGB, red: 1, green: 0, blue: 0, opacity: 0.7)),
    GradientStop(at: 0.5, colour: Color(.sRGB, red: 1, green: 0.64, blue: 0, opacity: 0.7)),
    GradientStop(at: 0.65, colour: Color(.sRGB, red: 1, green: 0.64, blue: 0, opacity: 0.3)),
    GradientStop(at: 0.95, colour: Color(.sRGB, red: 0, green: 1, blue: 0, opacity: 0.3)),
    GradientStop(at: 1, colour: Color(.sRGB, red: 0, green: 1, blue: 0, opacity: 0)),
]

private let VIDEO_STOPS = [
    GradientStop(at: 0, colour: Color(.sRGB, red: 1, green: 0, blue: 0, opacity: 0.9)),
    GradientStop(at: 0.1, colour: Color(.sRGB, red: 1, green: 0, blue: 0, opacity: 0.3)),
    GradientStop(at: 0.5, colour: Color(.sRGB, red: 1, green: 0.64, blue: 0, opacity: 0.3)),
    GradientStop(at: 0.65, colour: Color(.sRGB, red: 1, green: 0.64, blue: 0, opacity: 0.2)),
    GradientStop(at: 0.95, colour: Color(.sRGB, red: 0, green: 1, blue: 0, opacity: 0.1)),
    GradientStop(at: 1, colour: Color(.sRGB, red: 0, green: 1, blue: 0, opacity: 0)),
]

func twoCircleStops(_ from: CGFloat, _ to: CGFloat, _ stops: [GradientStop]) -> [GradientStop] {
    stops.map { GradientStop(at: (from + $0.at * (to - from)) / to, colour: $0.colour) }
}

private func radial(_ centre: CGPoint, _ from: CGFloat, _ to: CGFloat, _ stops: [GradientStop]) -> GraphicsContext.Shading {
    .radialGradient(
        Gradient(stops: twoCircleStops(from, to, stops).map { Gradient.Stop(color: $0.colour, location: $0.at) }),
        center: centre,
        startRadius: 0,
        endRadius: to
    )
}

struct OverlayPoint: Equatable {
    var outer: CGPoint
    var inner: CGPoint
    var range: Double
    var index: Int
}

struct MapOverlayShape: Equatable {
    var gradientFrom: CGFloat
    var gradientTo: CGFloat
    var points: [OverlayPoint]
}

func mapOverlayShape(
    _ overlay: ObstacleOverlay,
    _ centre: CGPoint,
    _ heightPx: CGFloat,
    _ metresInProbe: Double,
    _ probePx: CGFloat,
    _ heading: Double,
    _ mapBearing: Double
) -> MapOverlayShape {
    let maxRadius = 0.9 * heightPx / 2
    let minRadius = maxRadius * 0.2
    let pixelsPerMetre = Double(probePx) / metresInProbe
    let trueScale = metresInProbe < TRUE_SCALE_BELOW_METRES
    let gradientFrom = trueScale ? 0 : minRadius
    let gradientTo = trueScale ? CGFloat(overlay.maxMetres * pixelsPerMetre) : maxRadius
    let metresToPixels = trueScale ? pixelsPerMetre : Double(maxRadius - minRadius) / overlay.maxMetres
    let height = Double(minRadius / 8)
    let points = overlay.ranges.indices.map { i in
        let degrees = Double(i) * overlay.increment
        let rad = (degrees - mapBearing) * .pi / 180
        let index = rangeIndex(degrees, overlay, heading)
        let metres = overlay.ranges[index]
        let pixels = Double(gradientFrom) + metres * metresToPixels
        return OverlayPoint(
            outer: CGPoint(x: centre.x + pixels * sin(rad), y: centre.y - pixels * cos(rad)),
            inner: CGPoint(x: centre.x + (pixels - height) * sin(rad), y: centre.y - (pixels - height) * cos(rad)),
            range: metres,
            index: index
        )
    }
    return MapOverlayShape(gradientFrom: gradientFrom, gradientTo: gradientTo, points: points)
}

func mapOverlayLabels(_ overlay: ObstacleOverlay, _ points: [OverlayPoint]) -> [OverlayPoint] {
    points.indices.filter { $0 % 3 == 0 }
        .map { i in points[(i + 1) % points.count].range < points[i].range ? points[(i + 1) % points.count] : points[i] }
        .reduce(([OverlayPoint](), -1.0)) { shown, point in
            point.range < overlay.maxMetres && abs(point.range - shown.1) > TEXT_STEP_METRES ? (shown.0 + [point], point.range) : shown
        }
        .0
}

struct VideoSegment: Equatable {
    var from: CGFloat
    var to: CGFloat
    var radFrom: Double
    var radTo: Double
    var label: String?
}

func videoOverlaySegments(_ overlay: ObstacleOverlay, _ heightPx: CGFloat, _ showText: Bool) -> ((CGFloat, CGFloat), [VideoSegment])? {
    let maxRadius = 0.9 * heightPx / 2
    let segmentHeight = maxRadius * 0.2 / 8
    let levels = overlay.maxMetres / LEVEL_METRES
    let gradientFrom = maxRadius - segmentHeight * CGFloat(levels) * 2
    guard maxRadius > 0, gradientFrom >= 0 else { return nil }
    let count = overlay.ranges.count
    let step = 360.0 / Double(SEGMENTS)
    let segments = (0..<SEGMENTS).flatMap { s -> [VideoSegment] in
        let degrees = Double(s) * step
        let first = rangeIndex(degrees, overlay, 0)
        let next = rangeIndex(degrees + step, overlay, 0)
        let end = first < next ? next : count + next
        let nearest = (first..<max(first, end)).map { $0 % count }
            .filter { overlay.ranges[$0] < overlay.maxMetres }
            .min { overlay.ranges[$0] < overlay.ranges[$1] }
        let rangeMin = nearest.map { overlay.ranges[$0] } ?? overlay.maxMetres
        let radFrom = degrees * .pi / 180
        let radTo = radFrom + step * .pi / 180 - SEGMENT_GAP_RAD
        let drawn = (0..<Int(ceil(levels))).map { ii -> (VideoSegment, Bool) in
            let from = maxRadius - CGFloat(ii) * segmentHeight * 2
            let rangeInLevel = overlay.maxMetres - Double(ii + 1) * LEVEL_METRES
            let reached = rangeMin > rangeInLevel || Double(ii) >= levels - 1
            let range = reached ? rangeMin : overlay.maxMetres
            let label = nearest.flatMap { showText && reached && range < overlay.maxMetres ? overlay.texts[$0] : nil }
            return (VideoSegment(from: from, to: from - segmentHeight, radFrom: radFrom, radTo: radTo, label: label), range == rangeMin)
        }
        let upTo = drawn.firstIndex { $0.1 }.map { $0 + 1 } ?? drawn.count
        return drawn.prefix(upTo).map(\.0)
    }
    let upToPositive = segments.firstIndex { $0.to < 0 } ?? segments.count
    return ((gradientFrom, maxRadius), Array(segments.prefix(upToPositive)))
}

private func outlinedText(_ context: GraphicsContext, _ text: String, _ at: CGPoint, _ textSize: CGFloat, _ fill: Color, _ strokeWidth: CGFloat) {
    let font = Font.system(size: textSize, weight: .bold)
    let outline = context.resolve(Text(text).font(font).foregroundStyle(Color(.sRGB, red: 0, green: 0, blue: 0, opacity: 0.8)))
    let shift = strokeWidth / 2
    [CGPoint(x: -shift, y: 0), CGPoint(x: shift, y: 0), CGPoint(x: 0, y: -shift), CGPoint(x: 0, y: shift)].forEach { offset in
        context.draw(outline, at: CGPoint(x: at.x + offset.x, y: at.y + offset.y), anchor: .bottomLeading)
    }
    context.draw(Text(text).font(font).foregroundStyle(fill), at: at, anchor: .bottomLeading)
}

func drawMapObstacleOverlay(_ context: GraphicsContext, _ overlay: ObstacleOverlay, _ shape: MapOverlayShape, _ centre: CGPoint, _ showText: Bool) {
    let points = shape.points
    guard !points.isEmpty, shape.gradientTo > 0 else { return }
    let brush = radial(centre, shape.gradientFrom, shape.gradientTo, MAP_STOPS)
    points.indices.filter { $0 % 3 == 0 }.forEach { i in
        let a = points[i]
        let b = points[(i + 1) % points.count]
        let c = points[(i + 2) % points.count]
        let d = points[(i + 3) % points.count]
        let wedge = Path { path in
            path.move(to: a.inner)
            path.addLine(to: a.outer)
            path.addCurve(to: d.outer, control1: b.outer, control2: c.outer)
            path.addLine(to: d.inner)
            path.addCurve(to: b.inner, control1: d.inner, control2: c.inner)
            path.closeSubpath()
        }
        context.fill(wedge, with: brush)
    }
    if showText {
        mapOverlayLabels(overlay, points).forEach { point in
            outlinedText(context, overlay.texts[point.index], point.inner, MAP_TEXT, Color(.sRGB, red: 1, green: 1, blue: 1, opacity: 0.9), 2)
        }
    }
}

private func arc(_ centre: CGPoint, _ radius: CGFloat, _ from: Double, _ to: Double) -> [CGPoint] {
    (0...8).map { step in
        let angle = from + (to - from) * Double(step) / 8
        return CGPoint(x: centre.x + radius * cos(angle), y: centre.y + radius * sin(angle))
    }
}

func drawVideoObstacleOverlay(_ context: GraphicsContext, _ size: CGSize, _ overlay: ObstacleOverlay, _ showText: Bool) {
    guard let (radii, segments) = videoOverlaySegments(overlay, size.height, showText) else { return }
    let centre = CGPoint(x: size.width / 2, y: size.height / 2)
    let brush = radial(centre, radii.0, radii.1, VIDEO_STOPS)
    segments.forEach { segment in
        let radFrom = segment.radFrom - .pi / 2
        let radTo = segment.radTo - .pi / 2
        let top = CGPoint(x: centre.x + segment.from * cos(radFrom), y: centre.y + segment.from * sin(radFrom))
        let topEnd = CGPoint(x: centre.x + segment.from * cos(radTo), y: centre.y + segment.from * sin(radTo))
        let ring = Path { path in
            path.move(to: top)
            path.addLines(arc(centre, segment.from, radFrom, radTo) + arc(centre, segment.to, radTo, radFrom) + [top])
            path.closeSubpath()
        }
        var stretched = context
        stretched.translateBy(x: centre.x, y: 0)
        stretched.scaleBy(x: 2, y: 1)
        stretched.translateBy(x: -centre.x, y: 0)
        stretched.fill(ring, with: brush)
        if let label = segment.label {
            let middle = CGPoint(x: (top.x + topEnd.x) / 2, y: (top.y + topEnd.y) / 2)
            outlinedText(context, label, CGPoint(x: 2 * middle.x - centre.x, y: middle.y), VIDEO_TEXT * 2, Color(.sRGB, red: 1, green: 1, blue: 1, opacity: 0.8), 4)
        }
    }
}

struct ObstacleVideoOverlay: View {
    let showText: Bool
    @MapPath(OBSTACLE_VIEW) private var json

    var body: some View {
        if let overlay = obstacleOverlay(json) {
            Canvas { context, size in drawVideoObstacleOverlay(context, size, overlay, showText) }
                .allowsHitTesting(false)
        }
    }
}
