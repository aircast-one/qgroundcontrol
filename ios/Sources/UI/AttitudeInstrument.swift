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
}

private func optAngle(_ json: JSON, _ key: String) -> Double? {
    json[key].isNull ? nil : json[key].double.flatMap { $0.isNaN ? nil : $0 }
}

let NO_VEHICLE_ATTITUDE = Attitude(roll: 0, pitch: 0, heading: 0, headingText: "", courseOverGround: nil, headingToHome: nil, headingToNextWaypoint: nil, noseUp: false)

func attitude(_ view: JSON?) -> Attitude? {
    guard let view, view["available"].bool else { return nil }
    return Attitude(
        roll: optAngle(view, "roll") ?? 0,
        pitch: optAngle(view, "pitch") ?? 0,
        heading: optAngle(view, "heading") ?? 0,
        headingText: view["headingText"].string,
        courseOverGround: optAngle(view, "courseOverGround"),
        headingToHome: optAngle(view, "headingToHome"),
        headingToNextWaypoint: optAngle(view, "headingToNextWaypoint"),
        noseUp: view["noseUp"].bool
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

struct OsdCompassDial: View {
    let size: CGFloat
    @QgcPath(ATTITUDE_PATH) private var view
    @Environment(\.theme) private var theme

    var body: some View {
        let reading = attitude(view) ?? NO_VEHICLE_ATTITUDE
        let home = theme.aircast.success
        let onHome = theme.aircast.onSuccess
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
                        path.move(to: pointOnRing(center, radius - 2, Double(degrees)))
                        path.addLine(to: pointOnRing(center, radius - (degrees % 90 == 0 ? 8 : 5), Double(degrees)))
                    }
                },
                with: .color(ring),
                lineWidth: 1.5
            )
            context.draw(
                Text("N").font(.system(size: 11, weight: .bold)).foregroundStyle(OSD_NORTH),
                at: pointOnRing(center, radius - 15, 0)
            )
            if let bearing = reading.headingToHome {
                let at = pointOnRing(center, radius - 8, bearing)
                context.fill(Path(ellipseIn: CGRect(x: at.x - 7, y: at.y - 7, width: 14, height: 14)), with: .color(home))
                context.draw(Text("H").font(.system(size: 9, weight: .bold)).foregroundStyle(onHome), at: at)
            }
            context.drawLayer { nose in
                nose.translateBy(x: center.x, y: center.y)
                nose.rotate(by: .degrees(reading.heading))
                nose.translateBy(x: -center.x, y: -center.y)
                let chevron = Path { path in
                    path.move(to: CGPoint(x: center.x, y: center.y - radius * OSD_CHEVRON_TIP))
                    path.addLine(to: CGPoint(x: center.x + radius * OSD_CHEVRON_WING, y: center.y + radius * OSD_CHEVRON_WING))
                    path.addLine(to: CGPoint(x: center.x, y: center.y + radius * OSD_CHEVRON_NOTCH))
                    path.addLine(to: CGPoint(x: center.x - radius * OSD_CHEVRON_WING, y: center.y + radius * OSD_CHEVRON_WING))
                    path.closeSubpath()
                }
                nose.fill(chevron, with: .color(.white))
                nose.stroke(chevron, with: .color(.black.opacity(OSD_DIAL_SCRIM)), lineWidth: 1)
            }
            if !reading.headingText.isBlank {
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
