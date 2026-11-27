import SwiftUI

let ATTITUDE_PATH = "view.attitude"
private let HORIZON_FRACTION = 0.72
private let PITCH_SPAN_DEGREES = 45.0

struct Attitude: Equatable {
    let roll: Double
    let pitch: Double
    let heading: Double
    let headingText: String
    let courseOverGround: Double?
    let headingToHome: Double?
    let headingToNextWaypoint: Double?
    let noseUp: Bool
    var homeBearing: Double? = nil
    var pilotBearing: Double? = nil
}

let NO_VEHICLE_ATTITUDE = Attitude(roll: 0, pitch: 0, heading: 0, headingText: "", courseOverGround: nil, headingToHome: nil, headingToNextWaypoint: nil, noseUp: false)

func attitude(_ view: JSON?) -> Attitude? {
    guard let view, view["available"].bool else { return nil }
    return Attitude(
        roll: view["roll"].nonNanDouble ?? 0,
        pitch: view["pitch"].nonNanDouble ?? 0,
        heading: view["heading"].nonNanDouble ?? 0,
        headingText: view["headingText"].string,
        courseOverGround: view["courseOverGround"].nonNanDouble,
        headingToHome: view["headingToHome"].nonNanDouble,
        headingToNextWaypoint: view["headingToNextWaypoint"].nonNanDouble,
        noseUp: view["noseUp"].bool,
        homeBearing: view["homeBearing"].nonNanDouble,
        pilotBearing: view["pilotBearing"].nonNanDouble
    )
}

func pitchOffset(_ pitch: Double, _ horizonRadius: Double) -> Double {
    pitch * (horizonRadius * 2) / PITCH_SPAN_DEGREES
}

private func pointOnRing(_ center: CGPoint, _ radius: CGFloat, _ degrees: Double) -> CGPoint {
    let radians = degrees * .pi / 180
    return CGPoint(x: center.x + radius * sin(radians), y: center.y - radius * cos(radians))
}

private let OSD_DIAL_SCRIM = 0.4
private let OSD_DIAL_RING_ALPHA = 0.7
private let OSD_DIAL_HORIZON_ALPHA = 0.45
private let OSD_CHEVRON_TIP: CGFloat = 0.42
private let OSD_CHEVRON_WING: CGFloat = 0.3
private let OSD_CHEVRON_NOTCH: CGFloat = 0.14
private let OSD_NORTH = Color(hex: 0xFF4D4D)
private let ROOMY_DIAL: CGFloat = 96

struct OsdCompassDial: View {
    let size: CGFloat
    @QgcPath(ATTITUDE_PATH) private var view
    @Environment(\.theme) private var theme

    var body: some View {
        let reading = attitude(view) ?? NO_VEHICLE_ATTITUDE
        let home = theme.aircast.success
        let onHome = theme.aircast.onSuccess
        let pilot = theme.colors.primary
        let roomy = size >= ROOMY_DIAL
        Canvas { context, canvas in
            let radius = min(canvas.width, canvas.height) / 2
            let center = CGPoint(x: canvas.width / 2, y: canvas.height / 2)
            let disc = Path(ellipseIn: CGRect(x: center.x - radius, y: center.y - radius, width: radius * 2, height: radius * 2))
            let ring = Color.white.opacity(OSD_DIAL_RING_ALPHA)
            context.fill(disc, with: .color(.black.opacity(OSD_DIAL_SCRIM)))
            context.drawLayer { horizon in
                horizon.clip(to: disc)
                horizon.translateBy(x: center.x, y: center.y)
                horizon.rotate(by: .degrees(-reading.roll))
                horizon.translateBy(x: -center.x, y: -center.y + pitchOffset(reading.pitch, radius * HORIZON_FRACTION))
                horizon.stroke(
                    Path { $0.move(to: CGPoint(x: center.x - radius, y: center.y)); $0.addLine(to: CGPoint(x: center.x + radius, y: center.y)) },
                    with: .color(.white.opacity(OSD_DIAL_HORIZON_ALPHA)),
                    lineWidth: 1.5
                )
            }
            context.stroke(
                Path(ellipseIn: CGRect(x: center.x - radius + 1, y: center.y - radius + 1, width: (radius - 1) * 2, height: (radius - 1) * 2)),
                with: .color(ring),
                lineWidth: 1.5
            )
            context.stroke(
                Path { path in
                    stride(from: 0, to: 360, by: 30).forEach { degrees in
                        path.move(to: pointOnRing(center, radius - 2, Double(degrees) - reading.heading))
                        path.addLine(to: pointOnRing(center, radius - (degrees % 90 == 0 ? 8 : 5), Double(degrees) - reading.heading))
                    }
                },
                with: .color(ring),
                lineWidth: 1.5
            )
            if let bearing = reading.pilotBearing {
                let at = pointOnRing(center, radius - 8, bearing - reading.heading)
                context.fill(Path(ellipseIn: CGRect(x: at.x - 5, y: at.y - 5, width: 10, height: 10)), with: .color(.white))
                context.fill(Path(ellipseIn: CGRect(x: at.x - 3.5, y: at.y - 3.5, width: 7, height: 7)), with: .color(pilot))
            }
            context.draw(
                Text("N").font(.system(size: 11, weight: .bold)).foregroundStyle(OSD_NORTH),
                at: pointOnRing(center, radius - 15, -reading.heading)
            )
            if let bearing = reading.homeBearing ?? reading.headingToHome {
                let at = pointOnRing(center, radius - 8, bearing - reading.heading)
                context.fill(Path(ellipseIn: CGRect(x: at.x - 7, y: at.y - 7, width: 14, height: 14)), with: .color(home))
                context.draw(Text("H").font(.system(size: 9, weight: .bold)).foregroundStyle(onHome), at: at)
            }
            let chevron = Path { path in
                path.move(to: CGPoint(x: center.x, y: center.y - radius * OSD_CHEVRON_TIP))
                path.addLine(to: CGPoint(x: center.x + radius * OSD_CHEVRON_WING, y: center.y + radius * OSD_CHEVRON_WING))
                path.addLine(to: CGPoint(x: center.x, y: center.y + radius * OSD_CHEVRON_NOTCH))
                path.addLine(to: CGPoint(x: center.x - radius * OSD_CHEVRON_WING, y: center.y + radius * OSD_CHEVRON_WING))
                path.closeSubpath()
            }
            context.fill(chevron, with: .color(.white))
            context.stroke(chevron, with: .color(.black.opacity(OSD_DIAL_SCRIM)), lineWidth: 1)
            if roomy && !reading.headingText.isBlank {
                context.draw(
                    Text(reading.headingText).font(.system(size: 11, weight: .bold)).foregroundStyle(.white),
                    at: CGPoint(x: center.x, y: center.y + radius * 0.5),
                    anchor: .top
                )
            }
        }
        .frame(width: size, height: size)
        .accessibilityElement()
        .accessibilityLabel("Heading \(reading.headingText)")
    }
}

private let SKY = Color(hex: 0x3A78C4)
private let GROUND = Color(hex: 0x4F8A3C)
private let WINGS = Color(hex: 0xFF6B2C)
private let LADDER = [-20.0, -10.0, 10.0, 20.0]
private let ROLL_TICKS = [-60, -45, -30, -20, -10, 0, 10, 20, 30, 45, 60]
let ATTITUDE_GAUGE_SIZE: CGFloat = 104

func attitudeLabel(_ reading: Attitude) -> String {
    "Attitude, pitch \(Int(reading.pitch.rounded()))°, roll \(Int(reading.roll.rounded()))°"
}

struct OsdAttitudeDial: View {
    let size: CGFloat
    @QgcPath(ATTITUDE_PATH) private var view

    var body: some View {
        let reading = attitude(view) ?? NO_VEHICLE_ATTITUDE
        Canvas { context, canvas in
            let radius = min(canvas.width, canvas.height) / 2
            let center = CGPoint(x: canvas.width / 2, y: canvas.height / 2)
            let disc = Path(ellipseIn: CGRect(x: center.x - radius, y: center.y - radius, width: radius * 2, height: radius * 2))
            let step = pitchOffset(10, radius * HORIZON_FRACTION)
            let wing = radius * 0.42
            context.drawLayer { horizon in
                horizon.clip(to: disc)
                horizon.translateBy(x: center.x, y: center.y)
                horizon.rotate(by: .degrees(-reading.roll))
                horizon.translateBy(x: -center.x, y: -center.y + pitchOffset(reading.pitch, radius * HORIZON_FRACTION))
                horizon.fill(Path(CGRect(x: center.x - radius * 3, y: center.y - radius * 4, width: radius * 6, height: radius * 4)), with: .color(SKY))
                horizon.fill(Path(CGRect(x: center.x - radius * 3, y: center.y, width: radius * 6, height: radius * 4)), with: .color(GROUND))
                horizon.stroke(
                    Path { $0.move(to: CGPoint(x: center.x - radius * 3, y: center.y)); $0.addLine(to: CGPoint(x: center.x + radius * 3, y: center.y)) },
                    with: .color(.white),
                    lineWidth: 1.5
                )
                LADDER.forEach { degrees in
                    let y = center.y - degrees / 10 * step
                    horizon.stroke(
                        Path { $0.move(to: CGPoint(x: center.x - radius * 0.18, y: y)); $0.addLine(to: CGPoint(x: center.x + radius * 0.18, y: y)) },
                        with: .color(.white),
                        lineWidth: 1
                    )
                    horizon.draw(
                        Text("\(Int(abs(degrees)))").font(.system(size: 8)).foregroundStyle(.white),
                        at: CGPoint(x: center.x + radius * 0.24, y: y),
                        anchor: .leading
                    )
                }
            }
            context.stroke(
                Path { path in
                    ROLL_TICKS.forEach { degrees in
                        path.move(to: pointOnRing(center, radius - 1, Double(degrees)))
                        path.addLine(to: pointOnRing(center, radius - (degrees % 30 == 0 ? 7 : 4), Double(degrees)))
                    }
                },
                with: .color(.white),
                lineWidth: 1
            )
            context.stroke(
                Path(ellipseIn: CGRect(x: center.x - radius + 0.75, y: center.y - radius + 0.75, width: (radius - 0.75) * 2, height: (radius - 0.75) * 2)),
                with: .color(.white.opacity(OSD_DIAL_RING_ALPHA)),
                lineWidth: 1.5
            )
            let pointer = pointOnRing(center, radius - 3, -reading.roll)
            context.fill(Path(ellipseIn: CGRect(x: pointer.x - 2.5, y: pointer.y - 2.5, width: 5, height: 5)), with: .color(WINGS))
            context.stroke(
                Path { path in
                    path.move(to: CGPoint(x: center.x - wing, y: center.y))
                    path.addLine(to: CGPoint(x: center.x - wing * 0.35, y: center.y))
                    path.addLine(to: CGPoint(x: center.x, y: center.y + wing * 0.25))
                    path.addLine(to: CGPoint(x: center.x + wing * 0.35, y: center.y))
                    path.addLine(to: CGPoint(x: center.x + wing, y: center.y))
                },
                with: .color(WINGS),
                style: StrokeStyle(lineWidth: 3, lineJoin: .round)
            )
        }
        .frame(width: size, height: size)
        .accessibilityElement()
        .accessibilityLabel(attitudeLabel(reading))
    }
}

struct AttitudeGauges: View {
    var size: CGFloat = ATTITUDE_GAUGE_SIZE

    var body: some View {
        HStack(spacing: Space.s2) {
            OsdAttitudeDial(size: size)
            OsdCompassDial(size: size)
        }
    }
}

struct AttitudeChoice: View {
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.theme) private var theme

    var body: some View {
        let layout = flyScreen.layout
        let shown = !layout.hidden.contains(ATTITUDE_KEY)
        SectionHeader(text: "Instruments")
        HStack(spacing: Space.s4) {
            Image(shown ? .checkBox : .checkBoxOutline)
                .font(.title3)
                .foregroundStyle(shown ? theme.colors.primary : theme.colors.onSurfaceVariant)
            Text(ATTITUDE_NAME).font(.bodyLarge)
            Spacer()
        }
        .padding(.horizontal, Space.s4)
        .frame(minHeight: Metrics.row)
        .contentShape(Rectangle())
        .onTapGesture { layout.setHidden(ATTITUDE_KEY, shown) }
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(shown ? [.isButton, .isSelected] : .isButton)
    }
}
