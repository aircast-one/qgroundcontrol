import SwiftUI

private let TAPE_SPAN_DEGREES = 120.0
private let TICK_STEP = 10
private let LABEL_STEP = 30
private let READOUT_CLEARANCE_DEGREES = 15.0
private let EDGE_MARGIN_DEGREES = 5.0
private let EDGE_FADE = 0.6
private let EDGE_REACH = TAPE_SPAN_DEGREES / 2 - EDGE_MARGIN_DEGREES
private let TAPE_WIDTH: CGFloat = 320
private let TAPE_HEIGHT: CGFloat = 40
private let POINTER: CGFloat = 6
private let POINTER_WING: CGFloat = 5
private let LABEL_TICK: CGFloat = 10
private let MINOR_TICK: CGFloat = 5
private let TICK_WIDTH: CGFloat = 1.5
private let GAP: CGFloat = 2
private let HOME_RADIUS: CGFloat = 6
private let TAPE_BACKDROP = Color.black.opacity(0.35)
private let CARDINALS = [0: "N", 90: "E", 180: "S", 270: "W"]

private func floorMod(_ value: Int, _ divisor: Int) -> Int { ((value % divisor) + divisor) % divisor }

func relativeBearing(_ bearing: Double, _ heading: Double) -> Double {
    ((bearing - heading).truncatingRemainder(dividingBy: 360) + 540).truncatingRemainder(dividingBy: 360) - 180
}

func tapeX(_ relative: Double, _ width: CGFloat) -> CGFloat { width / 2 + CGFloat(relative / TAPE_SPAN_DEGREES) * width }

func tapeLabel(_ degrees: Int) -> String {
    let wrapped = floorMod(degrees, 360)
    return CARDINALS[wrapped] ?? String(wrapped)
}

func tapeTicks(_ heading: Double) -> [Int] {
    let half = TAPE_SPAN_DEGREES / 2
    let step = Double(TICK_STEP)
    let first = saturatingInt(((heading - half) / step).rounded(.up) * step)
    let last = saturatingInt(((heading + half) / step).rounded(.down) * step)
    guard first <= last else { return [] }
    return Array(stride(from: first, through: last, by: TICK_STEP))
}

func tapeLabelShown(_ relative: Double) -> Bool {
    abs(relative) > READOUT_CLEARANCE_DEGREES && abs(relative) < EDGE_REACH
}

func headingReadout(_ heading: Double) -> String { "\(floorMod(saturatingInt(heading.rounded()), 360))°" }

private func clampedTapeX(_ bearing: Double, _ heading: Double, _ width: CGFloat) -> CGFloat {
    tapeX(min(max(relativeBearing(bearing, heading), -EDGE_REACH), EDGE_REACH), width)
}

private func drawPointer(_ context: GraphicsContext, _ x: CGFloat, _ colour: Color) {
    let wedge = Path { path in
        path.move(to: CGPoint(x: x - POINTER_WING, y: 0))
        path.addLine(to: CGPoint(x: x + POINTER_WING, y: 0))
        path.addLine(to: CGPoint(x: x, y: POINTER))
        path.closeSubpath()
    }
    context.fill(wedge, with: .color(colour))
}

struct HeadingTape: View {
    @QgcPath(ATTITUDE_PATH) private var view
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        if let reading = attitude(view) {
            let heading = headingReadout(reading.heading)
            let home = theme.aircast.success
            let onHome = theme.aircast.onSuccess
            let mission = theme.aircast.mission
            Canvas { context, size in
                let line = POINTER + GAP
                let labelTop = line + LABEL_TICK + GAP
                tapeTicks(reading.heading).forEach { degrees in
                    let relative = relativeBearing(Double(degrees), reading.heading)
                    let x = tapeX(relative, size.width)
                    let shade = Color.white.opacity(1 - abs(relative) / (TAPE_SPAN_DEGREES / 2) * EDGE_FADE)
                    let labelled = degrees % LABEL_STEP == 0
                    let tick = Path { path in
                        path.move(to: CGPoint(x: x, y: line))
                        path.addLine(to: CGPoint(x: x, y: line + (labelled ? LABEL_TICK : MINOR_TICK)))
                    }
                    context.stroke(tick, with: .color(shade), lineWidth: TICK_WIDTH)
                    if labelled && tapeLabelShown(relative) {
                        context.draw(
                            Text(tapeLabel(degrees)).font(.system(size: 11, weight: .semibold)).foregroundStyle(shade),
                            at: CGPoint(x: x, y: labelTop),
                            anchor: .top
                        )
                    }
                }
                if let next = reading.headingToNextWaypoint {
                    drawPointer(context, clampedTapeX(next, reading.heading, size.width), mission)
                }
                if let bearing = reading.homeBearing ?? reading.headingToHome {
                    let at = CGPoint(x: clampedTapeX(bearing, reading.heading, size.width), y: line)
                    context.fill(Path(ellipseIn: CGRect(x: at.x - HOME_RADIUS, y: at.y - HOME_RADIUS, width: HOME_RADIUS * 2, height: HOME_RADIUS * 2)), with: .color(home))
                    context.draw(Text("H").font(.system(size: 9, weight: .bold)).foregroundStyle(onHome), at: at)
                }
                drawPointer(context, size.width / 2, .white)
                context.draw(
                    Text(heading).font(.system(size: 15, weight: .bold).monospacedDigit()).foregroundStyle(.white),
                    at: CGPoint(x: size.width / 2, y: labelTop - GAP),
                    anchor: .top
                )
            }
            .osdShadow()
            .frame(width: TAPE_WIDTH, height: TAPE_HEIGHT)
            .background(osdBackdrop(TAPE_BACKDROP, flyOsd), in: RoundedRectangle(cornerRadius: Corner.small))
            .allowsHitTesting(false)
            .accessibilityElement()
            .accessibilityLabel("Heading \(heading)")
        }
    }
}
