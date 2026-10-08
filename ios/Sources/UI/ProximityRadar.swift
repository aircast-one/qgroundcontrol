import SwiftUI

let PROXIMITY_VIEW = "view.proximityRadar"
private let SECTOR_SWEEP_DEG = 45.0
private let SECTOR_GAP_DEG = 360.0 / 100.0
private let ARC_START_OFFSET_DEG = -90.0 - 22.5
private let RADAR_COLOUR = Color(.sRGB, red: 1, green: 0, blue: 0, opacity: 0.5)

struct RadarSector: Equatable {
    let bearing: Int
    let meters: Double?
    let text: String
}

struct ProximityRadar: Equatable {
    let range: Double
    let sectors: [RadarSector]
}

func proximityRadar(_ view: JSON?) -> ProximityRadar? {
    guard let view, view["shown"].bool else { return nil }
    return ProximityRadar(
        range: view["rangeMeters"].double(6.0),
        sectors: view["sectors"].array.filter { $0.object != nil }.map { sector in
            RadarSector(bearing: sector["bearing"].int(0), meters: sector["meters"].isNull ? nil : sector["meters"].double(.nan), text: sector["text"].string)
        }
    )
}

struct ProximityRadarOverlay: View {
    @QgcPath(PROXIMITY_VIEW) private var json

    var body: some View {
        if let radar = proximityRadar(json) {
            GeometryReader { geometry in
                let width = geometry.size.width
                let height = geometry.size.height
                let shortest = min(width, height)
                let ratio = (shortest / 2) / radar.range
                let scaleX = width / shortest
                let scaleY = height / shortest
                let placed = radar.sectors.compactMap { sector in sector.meters.map { (sector, $0) } }
                ZStack {
                    Canvas { context, canvas in
                        let center = CGPoint(x: canvas.width / 2, y: canvas.height / 2)
                        placed.forEach { sector, meters in
                            let start = ARC_START_OFFSET_DEG + Double(sector.bearing) + SECTOR_GAP_DEG
                            let arc = Path { $0.addArc(center: .zero, radius: meters * ratio, startAngle: .degrees(start), endAngle: .degrees(start + SECTOR_SWEEP_DEG - 2 * SECTOR_GAP_DEG), clockwise: false) }
                                .applying(CGAffineTransform(scaleX: scaleX, y: scaleY))
                                .offsetBy(dx: center.x, dy: center.y)
                            context.stroke(arc, with: .color(RADAR_COLOUR), lineWidth: width / 100)
                        }
                    }
                    ForEach(Array(placed.enumerated()), id: \.offset) { _, entry in
                        let (sector, meters) = entry
                        let angle = -Double.pi / 2 + Double.pi / 180 * Double(sector.bearing)
                        Text(sector.text)
                            .font(.labelSmall)
                            .fontWeight(.bold)
                            .foregroundStyle(.white)
                            .position(x: width / 2 + cos(angle) * meters * ratio * scaleX, y: height / 2 + sin(angle) * meters * ratio * scaleY)
                    }
                }
            }
            .allowsHitTesting(false)
        }
    }
}
