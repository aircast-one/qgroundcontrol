import SwiftUI

let VIDEO_AR_VIEW = "view.videoAr"
private let ROUTE_COLOUR = Color(red: 1, green: 0xE9 / 255, blue: 0xA6 / 255)
private let HOME_COLOUR = Color(red: 0x4C / 255, green: 0xAF / 255, blue: 0x50 / 255)
private let AIM_COLOUR = Color(red: 1, green: 0x3B / 255, blue: 0x30 / 255)
private let IMPACT_COLOUR = Color(red: 1, green: 0xD6 / 255, blue: 0x0A / 255)
private let PIPPER_RADIUS: CGFloat = 14
private let PIPPER_WIDTH: CGFloat = 3
private let PIPPER_DOT: CGFloat = 2
private let LEAD_RADIUS: CGFloat = 9
private let LEAD_WIDTH: CGFloat = 2
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
    var aim: ArMark? = nil
    var impact: ArMark? = nil
    var lead: ArSpot? = nil
    var leadLine: ArLeg? = nil
}

private func spot(_ json: JSON) -> ArSpot? {
    guard let x = json["x"].double, let up = json["up"].double else { return nil }
    return ArSpot(x: x, up: up)
}

private func leg(_ json: JSON) -> ArLeg? {
    spot(json["from"]).flatMap { from in spot(json["to"]).map { ArLeg(from: from, to: $0) } }
}

private func mark(_ json: JSON) -> ArMark? {
    spot(json).map { ArMark(at: $0, label: json["label"].string, text: json["text"].string) }
}

func arReading(_ view: JSON?) -> ArReading? {
    guard let view, view["available"].bool else { return nil }
    return ArReading(
        home: mark(view["home"]),
        waypoints: view["waypoints"].array.compactMap(mark),
        route: view["route"].array.compactMap(leg),
        aim: mark(view["aim"]),
        impact: mark(view["impact"]),
        lead: spot(view["lead"]),
        leadLine: leg(view["leadLine"])
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
                reading.leadLine.map { leg in
                    let line = Path { path in
                        path.move(to: arPoint(leg.from, size))
                        path.addLine(to: arPoint(leg.to, size))
                    }
                    context.stroke(line, with: .color(EDGE_COLOUR), style: StrokeStyle(lineWidth: LEAD_WIDTH + EDGE_WIDTH, lineCap: .round))
                    context.stroke(line, with: .color(AIM_COLOUR), style: StrokeStyle(lineWidth: LEAD_WIDTH, lineCap: .round))
                }
                reading.lead.map { spot in
                    let ring = circle(arPoint(spot, size), LEAD_RADIUS)
                    context.stroke(ring, with: .color(EDGE_COLOUR), lineWidth: LEAD_WIDTH + EDGE_WIDTH)
                    context.stroke(ring, with: .color(AIM_COLOUR), lineWidth: LEAD_WIDTH)
                }
                reading.aim.map { marked(context, size, $0, AIM_COLOUR) }
                reading.impact.map { marked(context, size, $0, IMPACT_COLOUR, ring: true) }
            }
            .allowsHitTesting(false)
            .clipped()
        }
    }

    private func circle(_ at: CGPoint, _ radius: CGFloat) -> Path {
        Path(ellipseIn: CGRect(x: at.x - radius, y: at.y - radius, width: radius * 2, height: radius * 2))
    }

    private func marked(_ context: GraphicsContext, _ size: CGSize, _ mark: ArMark, _ fill: Color, ring: Bool = false) {
        let at = arPoint(mark.at, size)
        let radius = ring ? PIPPER_RADIUS : MARK_RADIUS
        if ring {
            context.stroke(circle(at, radius), with: .color(EDGE_COLOUR), lineWidth: PIPPER_WIDTH + EDGE_WIDTH * 2)
            context.stroke(circle(at, radius), with: .color(fill), lineWidth: PIPPER_WIDTH)
            context.fill(circle(at, PIPPER_DOT), with: .color(fill))
        } else {
            context.fill(circle(at, radius + EDGE_WIDTH), with: .color(EDGE_COLOUR))
            context.fill(circle(at, radius), with: .color(fill))
        }
        let caption = [mark.label, mark.text].filter { !$0.isEmpty }.joined(separator: " · ")
        var shadowed = context
        shadowed.addFilter(.shadow(color: .black, radius: LABEL_SHADOW))
        shadowed.draw(Text(caption).font(TypeScale.labelMedium.font).foregroundColor(.white), at: CGPoint(x: at.x, y: at.y - radius - LABEL_GAP), anchor: .bottom)
    }
}
