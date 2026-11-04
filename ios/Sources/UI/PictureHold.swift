import SwiftUI

let GROUND_AT = "videoAr.groundAt"
let VIDEO_AR_OUTLINE = "videoAr.outline"
let PAUSE_VEHICLE_PATH = "vehicle.pauseVehicle"

let PICTURE_HOLD_HINT = "Press and hold a spot to look at it or fly there"
let BELOW_HORIZON = "Press and hold on a spot below the horizon"
let NO_GIMBAL = "The vehicle reports no gimbal, so a spot in the picture can't be placed on the map"

private let NOT_GROUND = "notGround"
private let HOLD_RING_DELAY_MS = 150
private let HOLD_RING_MIN_MS = 100
private let OUTLINE_SEGMENTS = 36
private let OUTLINE_POLL_MS = 200
private let CHIP_SCRIM_ALPHA = 0.6
private let OUTLINE_COLOUR = Color(red: 0, green: 0xE5 / 255, blue: 1)
private let EDGE_COLOUR = Color.black.opacity(0.6)
private let OUTLINE_WIDTH: CGFloat = 3
private let EDGE_WIDTH: CGFloat = 2
private let RING_SIZE: CGFloat = 48
private let RING_STROKE: CGFloat = 4
private let MARK_RADIUS: CGFloat = 13
private let RETICLE_SIZE: CGFloat = 64
private let RETICLE_STROKE: CGFloat = 2
private let RETICLE_DOT: CGFloat = 3
private let CHIP_GAP: CGFloat = 8

struct PicturePress: Equatable {
    let spot: CGPoint
    let point: MapPoint?
    let actions: [String]
    let warning: String?
    let reason: String
}

func picturePress(_ spot: CGPoint, _ answer: JSON?, hasGimbal: Bool) -> PicturePress {
    let placed = answer.flatMap { json -> MapPoint? in
        guard hasGimbal, json["ok"].bool, let latitude = json["latitude"].double, let longitude = json["longitude"].double,
              latitude.isFinite, longitude.isFinite else { return nil }
        return MapPoint(latitude: latitude, longitude: longitude)
    }
    let reason = !hasGimbal ? NO_GIMBAL : answer?["refusal"].string == NOT_GROUND ? BELOW_HORIZON : refusal(answer) ?? ""
    return PicturePress(
        spot: spot,
        point: placed,
        actions: answer?["actions"].array.compactMap { $0.string.nonBlank } ?? [],
        warning: answer?["warning"].string.nonBlank,
        reason: reason
    )
}

func groundAt(_ at: CGPoint, _ size: CGSize, hasGimbal: Bool) -> PicturePress {
    guard size.width > 0, size.height > 0 else { return picturePress(.zero, nil, hasGimbal: hasGimbal) }
    let spot = CGPoint(x: at.x / size.width, y: at.y / size.height)
    guard hasGimbal else { return picturePress(spot, nil, hasGimbal: false) }
    let (pan, tilt) = screenFraction(at.x, at.y, size.width, size.height)
    return picturePress(spot, Qgc.call(GROUND_AT, Double(pan), Double(tilt), Double(size.width / size.height), false), hasGimbal: true)
}

struct HoldRing: View {
    let at: CGPoint?
    @State private var progress: CGFloat = 0

    var body: some View {
        ZStack {
            Color.clear
            if let pressed = at {
                Circle()
                    .trim(from: 0, to: progress)
                    .stroke(Color.white, style: StrokeStyle(lineWidth: RING_STROKE, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                    .frame(width: RING_SIZE - RING_STROKE * 2, height: RING_SIZE - RING_STROKE * 2)
                    .position(pressed)
                    .opacity(progress == 0 ? 0 : 1)
            }
        }
        .allowsHitTesting(false)
        .task(id: at) {
            progress = 0
            guard at != nil else { return }
            try? await Task.sleep(for: .milliseconds(HOLD_RING_DELAY_MS))
            guard !Task.isCancelled else { return }
            let seconds = Double(max(LONG_PRESS_MS - HOLD_RING_DELAY_MS, HOLD_RING_MIN_MS)) / 1000
            withAnimation(.linear(duration: seconds)) { progress = 1 }
        }
    }
}

struct PictureMark: View {
    let spot: CGPoint

    var body: some View {
        Canvas { context, size in
            let at = CGPoint(x: spot.x * size.width, y: spot.y * size.height)
            context.stroke(circle(at, MARK_RADIUS), with: .color(EDGE_COLOUR), lineWidth: EDGE_WIDTH * 2)
            context.stroke(circle(at, MARK_RADIUS - 1), with: .color(.white), lineWidth: EDGE_WIDTH)
            context.fill(circle(at, EDGE_WIDTH), with: .color(.white))
        }
        .allowsHitTesting(false)
    }
}

private func circle(_ at: CGPoint, _ radius: CGFloat) -> Path {
    Path(ellipseIn: CGRect(x: at.x - radius, y: at.y - radius, width: radius * 2, height: radius * 2))
}

struct OrbitOutline: View {
    @Environment(FlyMapEdits.self) private var mapEdits: FlyMapEdits?
    @QgcPath(VIDEO_AR_VIEW) private var videoArJson
    @State private var polled: [ArSpot?] = []

    var body: some View {
        let pending = mapEdits?.orbit
        let legs = outlineLegs(pending != nil ? polled : orbitOutline(videoArJson))
        ZStack {
            Color.clear
            if !legs.isEmpty {
                Canvas { context, size in
                    let line = Path { path in
                        legs.forEach { leg in
                            path.move(to: arPoint(leg.from, size))
                            path.addLine(to: arPoint(leg.to, size))
                        }
                    }
                    context.stroke(line, with: .color(EDGE_COLOUR), style: StrokeStyle(lineWidth: OUTLINE_WIDTH + EDGE_WIDTH * 2, lineCap: .round, lineJoin: .round))
                    context.stroke(line, with: .color(OUTLINE_COLOUR), style: StrokeStyle(lineWidth: OUTLINE_WIDTH, lineCap: .round, lineJoin: .round))
                }
            }
        }
        .allowsHitTesting(false)
        .task(id: pending) {
            guard let orbit = pending else { return }
            let latitude = orbit.centre.latitude
            let longitude = orbit.centre.longitude
            let radius = orbit.radiusMetres
            while !Task.isCancelled {
                polled = await offMain { outlineSpots(Qgc.call(VIDEO_AR_OUTLINE, latitude, longitude, radius, OUTLINE_SEGMENTS)) }
                try? await Task.sleep(for: .milliseconds(OUTLINE_POLL_MS))
            }
        }
    }
}

enum Pointing: Equatable {
    case Looking, Orbiting

    var text: String {
        switch self {
        case .Looking: "Looking at this point"
        case .Orbiting: "Orbiting this point"
        }
    }

    var stopPath: String {
        switch self {
        case .Looking: STOP_ROI_PATH
        case .Orbiting: PAUSE_VEHICLE_PATH
        }
    }
}

func pointing(looking: Bool, orbiting: Bool) -> Pointing? {
    orbiting ? .Orbiting : looking ? .Looking : nil
}

struct LookAtReticle: View {
    @Environment(FlyScreenState.self) private var flyScreen: FlyScreenState?
    @QgcPath(GUIDED_ACTIONS) private var actionsJson
    @QgcPath(ORBIT_VIEW) private var orbitJson

    var body: some View {
        if let shown = pointing(looking: actionsJson?["roiActive"].bool == true, orbiting: orbitReading(orbitJson)?.orbiting == true) {
            Canvas { context, size in
                let radius = min(size.width, size.height) / 2 - RETICLE_STROKE
                let centre = CGPoint(x: size.width / 2, y: size.height / 2)
                context.stroke(circle(centre, radius), with: .color(EDGE_COLOUR), lineWidth: RETICLE_STROKE * 2)
                context.stroke(circle(centre, radius), with: .color(.white), lineWidth: RETICLE_STROKE)
                context.fill(circle(centre, RETICLE_DOT), with: .color(.white))
            }
            .frame(width: RETICLE_SIZE, height: RETICLE_SIZE)
            .allowsHitTesting(false)
            .overlay(alignment: .bottom) {
                chip(shown)
                    .fixedSize()
                    .alignmentGuide(.bottom) { placed in placed[.top] - CHIP_GAP }
            }
        }
    }

    private func chip(_ shown: Pointing) -> some View {
        HStack(spacing: 0) {
            Text(shown.text).font(.labelLarge)
            Button("Stop") { stop(shown) }
                .buttonStyle(.text)
                .foregroundStyle(Color.white)
                .padding(.horizontal, Space.s2)
        }
        .foregroundStyle(Color.white)
        .padding(.leading, Space.s3)
        .background(Color.black.opacity(CHIP_SCRIM_ALPHA), in: RoundedRectangle(cornerRadius: Corner.small))
    }

    private func stop(_ shown: Pointing) {
        let path = shown.stopPath
        let flyScreen = flyScreen
        Task {
            if let refused = await offMain({ Qgc.refusalOf(path) }) { flyScreen?.refusal = refused }
        }
    }
}
