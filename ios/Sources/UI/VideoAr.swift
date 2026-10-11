import SwiftUI

let VIDEO_AR_VIEW = "view.videoAr"
private let ROUTE_COLOUR = Color(red: 1, green: 0xE9 / 255, blue: 0xA6 / 255)
private let HOME_COLOUR = Color(red: 0x4C / 255, green: 0xAF / 255, blue: 0x50 / 255)
private let EDGE_COLOUR = Color.black.opacity(0.6)
private let ROUTE_WIDTH: CGFloat = 3
private let EDGE_WIDTH: CGFloat = 2
private let MARK_RADIUS: CGFloat = 7
private let LABEL_GAP: CGFloat = 4
private let LABEL_SHADOW: CGFloat = 2

struct ArSpot: Equatable {
    let x: Double
    let up: Double
}

struct ArMark: Equatable {
    let at: ArSpot
    let label: String
    let text: String
}

struct ArLeg: Equatable {
    let from: ArSpot
    let to: ArSpot
}

struct ArReading: Equatable {
    let home: ArMark?
    let waypoints: [ArMark]
    let route: [ArLeg]
}

private func spot(_ json: JSON) -> ArSpot? {
    guard let x = json["x"].double, let up = json["up"].double else { return nil }
    return ArSpot(x: x, up: up)
}

private func mark(_ json: JSON) -> ArMark? {
    spot(json).map { ArMark(at: $0, label: json["label"].string, text: json["text"].string) }
}

func arReading(_ view: JSON?) -> ArReading? {
    guard let view, view["available"].bool else { return nil }
    return ArReading(
        home: mark(view["home"]),
        waypoints: view["waypoints"].array.compactMap(mark),
        route: view["route"].array.compactMap { leg in spot(leg["from"]).flatMap { from in spot(leg["to"]).map { ArLeg(from: from, to: $0) } } }
    )
}

func arPoint(_ spot: ArSpot, _ size: CGSize) -> CGPoint {
    CGPoint(x: spot.x * size.width, y: size.height / 2 - spot.up * size.width)
}

struct VideoArOverlay: View {
    @QgcPath(VIDEO_AR_VIEW) private var json

    var body: some View {
        if let reading = arReading(json) {
            Canvas { context, size in
                reading.route.forEach { leg in
                    let line = Path { path in
                        path.move(to: arPoint(leg.from, size))
                        path.addLine(to: arPoint(leg.to, size))
                    }
                    context.stroke(line, with: .color(EDGE_COLOUR), style: StrokeStyle(lineWidth: ROUTE_WIDTH + EDGE_WIDTH * 2, lineCap: .round))
                    context.stroke(line, with: .color(ROUTE_COLOUR), style: StrokeStyle(lineWidth: ROUTE_WIDTH, lineCap: .round))
                }
                reading.waypoints.forEach { marked(context, size, $0, .white) }
                reading.home.map { marked(context, size, $0, HOME_COLOUR) }
            }
            .allowsHitTesting(false)
            .clipped()
        }
    }

    private func marked(_ context: GraphicsContext, _ size: CGSize, _ mark: ArMark, _ fill: Color) {
        let at = arPoint(mark.at, size)
        let ring = MARK_RADIUS + EDGE_WIDTH
        context.fill(Path(ellipseIn: CGRect(x: at.x - ring, y: at.y - ring, width: ring * 2, height: ring * 2)), with: .color(EDGE_COLOUR))
        context.fill(Path(ellipseIn: CGRect(x: at.x - MARK_RADIUS, y: at.y - MARK_RADIUS, width: MARK_RADIUS * 2, height: MARK_RADIUS * 2)), with: .color(fill))
        let caption = [mark.label, mark.text].filter { !$0.isEmpty }.joined(separator: " · ")
        var shadowed = context
        shadowed.addFilter(.shadow(color: .black, radius: LABEL_SHADOW))
        shadowed.draw(Text(caption).font(TypeScale.labelMedium.font).foregroundColor(.white), at: CGPoint(x: at.x, y: at.y - MARK_RADIUS - LABEL_GAP), anchor: .bottom)
    }
}
