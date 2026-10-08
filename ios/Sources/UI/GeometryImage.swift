import SwiftUI

private let AXIS_INDICATOR_SIZE: CGFloat = 15
private let IMAGE_MARGIN: CGFloat = 5
private let FRAME_WIDTH: CGFloat = 6
private let COAX_DISTANCE = 0.03
private let SPIN_ARC_DEGREES: CGFloat = 50
private let IMAGE_ASPECT: CGFloat = 2
private let CLOCKWISE = Color(.sRGB, red: 21 / 255, green: 158 / 255, blue: 31 / 255, opacity: 200 / 255)
private let COUNTER_CLOCKWISE = Color(.sRGB, red: 78 / 255, green: 195 / 255, blue: 232 / 255, opacity: 200 / 255)
private let FRAME_ARROW = Color(.sRGB, red: 255 / 255, green: 68 / 255, blue: 43 / 255, opacity: 200 / 255)
private let FRAME = Color(.sRGB, red: 150 / 255, green: 150 / 255, blue: 150 / 255, opacity: 1)

struct GeometryMotor: Equatable {
    let index: Int
    let label: Int
    let x: Double
    let y: Double
    let counterClockwise: Bool
}

struct DrawnMotor: Equatable {
    let motor: GeometryMotor
    let center: CGPoint
    let textCenter: CGPoint
    let coax: Bool
}

struct GeometryLayout: Equatable {
    let origin: CGPoint
    let rotorDiameter: CGFloat
    let fontSize: CGFloat
    let extraYMargin: CGFloat
    let motors: [DrawnMotor]
}

func geometryMotors(_ geometry: JSON?) -> [GeometryMotor] {
    (geometry?["motors"].array ?? []).filter { $0.object != nil }.map {
        GeometryMotor(index: $0["index"].int(0), label: $0["label"].int(0), x: $0["x"].double(.nan), y: $0["y"].double(.nan), counterClockwise: $0["counterClockwise"].bool)
    }
}

func geometryLayout(_ motors: [GeometryMotor], _ width: CGFloat, _ height: CGFloat) -> GeometryLayout? {
    guard motors.count > 1,
          let minX = motors.map(\.x).min(), let maxX = motors.map(\.x).max(),
          let minY = motors.map(\.y).min(), let maxY = motors.map(\.y).max(),
          !(maxX - minX < 0.0001 || maxY - minY < 0.0001) else { return nil }
    let coax = motors.enumerated().map { at, motor in
        motors.prefix(at).contains { hypot($0.x - motor.x, $0.y - motor.y) < COAX_DISTANCE }
    }
    let squeezed = width < height + AXIS_INDICATOR_SIZE * 4
    let usableWidth = width - 2 * IMAGE_MARGIN - (squeezed ? AXIS_INDICATOR_SIZE : 0)
    let usableHeight = height - 2 * IMAGE_MARGIN - (squeezed ? AXIS_INDICATOR_SIZE : 0)
    let extraOffsetX = squeezed ? AXIS_INDICATOR_SIZE : 0
    let rotorDiameter = min(usableWidth, usableHeight) * (motors.count <= 6 ? 0.31 : 0.25)
    let fontSize = rotorDiameter * 0.4
    let extraYMargin = coax.contains(true) ? fontSize * 1.1 : 0
    let scaleX = (usableWidth - rotorDiameter) / CGFloat(maxY - minY)
    let scaleY = (usableHeight - extraYMargin - rotorDiameter) / CGFloat(maxX - minX)
    let scale = min(scaleX, scaleY)
    let offsetX = IMAGE_MARGIN + extraOffsetX + usableWidth / 2 - CGFloat((maxY + minY) / 2) * scale
    let offsetY = IMAGE_MARGIN + (usableHeight - extraYMargin) / 2 + CGFloat((maxX + minX) / 2) * scale
    let drawn = zip(motors, coax).map { motor, isCoax in
        let base = CGPoint(x: offsetX + CGFloat(motor.y) * scale, y: offsetY - CGFloat(motor.x) * scale)
        let center = isCoax ? CGPoint(x: base.x, y: base.y + extraYMargin) : base
        let textCenter = isCoax ? CGPoint(x: center.x, y: center.y + rotorDiameter / 2 - extraYMargin / 2) : center
        return DrawnMotor(motor: motor, center: center, textCenter: textCenter, coax: isCoax)
    }
    return GeometryLayout(origin: CGPoint(x: offsetX, y: offsetY), rotorDiameter: rotorDiameter, fontSize: fontSize, extraYMargin: extraYMargin, motors: drawn)
}

struct GeometryImage: View {
    let motors: [GeometryMotor]
    var highlighted: Set<Int> = []
    var onMotor: (Int) -> Void = { _ in }
    @Environment(\.theme) private var theme

    var body: some View {
        let ink = theme.colors.onSurface
        GeometryReader { geo in
            Canvas { context, size in
                guard let layout = geometryLayout(motors, size.width, size.height) else { return }
                layout.motors.filter { !$0.coax }.forEach { drawn in
                    context.stroke(Path { $0.move(to: layout.origin); $0.addLine(to: drawn.center) }, with: .color(FRAME), lineWidth: FRAME_WIDTH)
                }
                let centerSize = layout.rotorDiameter * 0.8
                context.fill(
                    Path(roundedRect: CGRect(x: layout.origin.x - centerSize / 2, y: layout.origin.y - centerSize / 2, width: centerSize, height: centerSize), cornerRadius: FRAME_WIDTH),
                    with: .color(FRAME)
                )
                let arrowWidth = layout.rotorDiameter / 4
                let arrowHeight = layout.rotorDiameter / 2
                context.fill(Path { path in
                    path.move(to: CGPoint(x: layout.origin.x - arrowWidth / 2, y: layout.origin.y + arrowHeight / 2))
                    path.addLine(to: CGPoint(x: layout.origin.x, y: layout.origin.y - arrowHeight / 2))
                    path.addLine(to: CGPoint(x: layout.origin.x + arrowWidth / 2, y: layout.origin.y + arrowHeight / 2))
                    path.closeSubpath()
                }, with: .color(FRAME_ARROW))
                drawAxisIndicator(context, CGPoint(x: AXIS_INDICATOR_SIZE / 2, y: size.height - AXIS_INDICATOR_SIZE / 2), ink)
                (layout.motors.filter(\.coax) + layout.motors.filter { !$0.coax }).forEach {
                    drawMotor(context, $0, layout, ink, highlighted.contains($0.motor.index))
                }
            }
            .contentShape(Rectangle())
            .onTapGesture { at in
                guard let layout = geometryLayout(motors, geo.size.width, geo.size.height) else { return }
                if let motor = motorAt(layout, at, highlighted) { onMotor(motor) }
            }
        }
        .aspectRatio(IMAGE_ASPECT, contentMode: .fit)
        .frame(maxWidth: .infinity)
    }
}

private func drawMotor(_ context: GraphicsContext, _ drawn: DrawnMotor, _ layout: GeometryLayout, _ ink: Color, _ highlight: Bool) {
    let fill = drawn.motor.counterClockwise ? COUNTER_CLOCKWISE : CLOCKWISE
    let arrowColor = Color(uiColor: UIColor(fill).withAlphaComponent(1))
    let radius = layout.rotorDiameter / 2
    context.fill(circle(drawn.center, radius), with: .color(fill))
    if highlight { context.fill(circle(drawn.textCenter, drawn.coax ? layout.fontSize / 2 : radius), with: .color(FRAME_ARROW)) }
    context.draw(Text("\(drawn.motor.label)").font(.system(size: layout.fontSize)).foregroundColor(ink), at: drawn.textCenter)
    let offsets: [CGFloat] = drawn.coax ? [30, 150] : [0, 180]
    let ySign: CGFloat = drawn.motor.counterClockwise ? 1 : -1
    offsets.forEach { offset in
        let turn = drawn.motor.counterClockwise ? -SPIN_ARC_DEGREES / 2 + offset : SPIN_ARC_DEGREES / 2 + offset
        var turned = context
        turned.translateBy(x: drawn.center.x, y: drawn.center.y)
        turned.rotate(by: .degrees(turn))
        turned.translateBy(x: -drawn.center.x, y: -drawn.center.y)
        let sweep = ySign * SPIN_ARC_DEGREES
        turned.stroke(Path { path in
            path.addArc(center: drawn.center, radius: radius, startAngle: .degrees(0), endAngle: .degrees(sweep), clockwise: sweep < 0)
        }, with: .color(arrowColor), lineWidth: 2.5)
        let head = FRAME_WIDTH * 1.25
        turned.fill(Path { path in
            path.move(to: CGPoint(x: drawn.center.x + radius - FRAME_WIDTH / 2, y: drawn.center.y + ySign * head / 2))
            path.addLine(to: CGPoint(x: drawn.center.x + radius, y: drawn.center.y - ySign * head / 2))
            path.addLine(to: CGPoint(x: drawn.center.x + radius + FRAME_WIDTH / 2, y: drawn.center.y + ySign * head / 2))
            path.closeSubpath()
        }, with: .color(arrowColor))
    }
}

private func drawAxisIndicator(_ context: GraphicsContext, _ origin: CGPoint, _ ink: Color) {
    let length = AXIS_INDICATOR_SIZE * 2
    let up = CGPoint(x: origin.x, y: origin.y - length)
    let right = CGPoint(x: origin.x + length, y: origin.y)
    context.stroke(Path { $0.move(to: origin); $0.addLine(to: up) }, with: .color(ink), lineWidth: 1.5)
    context.stroke(Path { $0.move(to: origin); $0.addLine(to: right) }, with: .color(ink), lineWidth: 1.5)
    context.draw(Text("x").font(.system(size: AXIS_INDICATOR_SIZE)).foregroundColor(ink), at: up, anchor: .bottom)
    context.draw(Text(" y").font(.system(size: AXIS_INDICATOR_SIZE)).foregroundColor(ink), at: right, anchor: .leading)
}

private func circle(_ center: CGPoint, _ radius: CGFloat) -> Path {
    Path(ellipseIn: CGRect(x: center.x - radius, y: center.y - radius, width: radius * 2, height: radius * 2))
}

func motorAt(_ layout: GeometryLayout, _ point: CGPoint, _ highlighted: Set<Int>) -> Int? {
    let hits = layout.motors.filter { highlighted.contains($0.motor.index) }.filter { drawn in
        hypot(drawn.textCenter.x - point.x, drawn.textCenter.y - point.y) < (drawn.coax ? layout.fontSize / 2 : layout.rotorDiameter / 2)
    }
    return hits.count == 1 ? hits[0].motor.index : nil
}
