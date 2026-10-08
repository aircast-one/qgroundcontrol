import SwiftUI

private let OBSTACLE_PATH = "view.obstacle"
private let ARC_SIZE: CGFloat = 48
private let SWEEP_PADDING = 0.85

func arcSweep(_ increment: Double) -> Double { increment * SWEEP_PADDING }

let NEAREST_VISIBLE_FRACTION = 0.12

func arcRadiusFraction(_ metres: Double, _ ceiling: Double) -> Double {
    ceiling <= 0 ? 0 : max(min(max(metres / ceiling, 0), 1).squareRoot(), NEAREST_VISIBLE_FRACTION)
}

func sampleIsClose(_ metres: Double, _ floorMetres: Double) -> Bool { metres < floorMetres * 2 }

enum ArcTone { case Alarm, Calm, Stale }

func arcTone(_ near: Bool, _ stale: Bool) -> ArcTone {
    stale ? .Stale : near ? .Alarm : .Calm
}

struct ObstacleArc: View {
    @QgcPath(OBSTACLE_PATH) private var view
    @Environment(\.theme) private var theme

    var body: some View {
        if let ring = obstacleRing(view) {
            let alarm = theme.colors.error
            let calm = theme.colors.onSurfaceVariant
            let faded = calm.opacity(0.45)
            VStack(spacing: 2) {
                Canvas { context, canvas in
                    let center = CGPoint(x: canvas.width / 2, y: canvas.height / 2)
                    let full = min(canvas.width, canvas.height) / 2
                    context.stroke(Path(ellipseIn: CGRect(x: center.x - full, y: center.y - full, width: full * 2, height: full * 2)), with: .color(calm.opacity(0.25)), lineWidth: 2)
                    context.fill(Path(ellipseIn: CGRect(x: center.x - 3, y: center.y - 3, width: 6, height: 6)), with: .color(calm.opacity(0.6)))
                    ring.samples.forEach { sample in
                        let reach = full * arcRadiusFraction(sample.metres, ring.maxMetres)
                        let near = sampleIsClose(sample.metres, ring.floorMetres)
                        let start = sample.bearingDegrees - 90 - arcSweep(ring.incrementDegrees) / 2
                        let tone: Color = switch arcTone(near, ring.stale) {
                        case .Alarm: alarm
                        case .Calm: calm
                        case .Stale: faded
                        }
                        context.stroke(
                            Path { $0.addArc(center: center, radius: reach, startAngle: .degrees(start), endAngle: .degrees(start + arcSweep(ring.incrementDegrees)), clockwise: false) },
                            with: .color(tone),
                            lineWidth: near ? 10 : 6
                        )
                    }
                }
                .frame(width: ARC_SIZE, height: ARC_SIZE)
                if ring.stale {
                    Text("Last seen").font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .background(theme.colors.surface.opacity(0.80), in: RoundedRectangle(cornerRadius: Corner.small))
        }
    }
}
