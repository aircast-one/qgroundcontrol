import SwiftUI

let APM_FOLLOW_VIEW = "view.apmFollow"
let APM_FOLLOW_SCREEN = "apmFollow"
let APM_FOLLOW_ENABLE = "apmFollow.enable"
let APM_FOLLOW_RESET = "apmFollow.reset"
let APM_FOLLOW_POSITION = "apmFollow.position"
let APM_FOLLOW_POINT = "apmFollow.point"
let APM_FOLLOW_OFFSETS = "apmFollow.offsets"
let APM_FOLLOW_HEIGHT = "apmFollow.height"
private let WAITING_POLL_MS = 500

struct ApmFollow: Equatable {
    let available: Bool
    let enabled: Bool
    let waiting: Bool
    let supported: Bool
    let unsupportedText: String
    let showSettings: Bool
    let rover: Bool
    let positionOptions: [String]
    let positionIndex: Int
    let pointOptions: [String]
    let pointIndex: Int
    let angle: Double
    let distance: Double
    let height: Double
    let horizontal: DistanceUnit
    let vertical: DistanceUnit
}

private extension DistanceUnit {
    func shown(_ metres: Double) -> Double { metres / metresPerUnit }

    func metres(_ shown: Double) -> Double { shown * metresPerUnit }
}

extension DistanceUnit {
    func text(_ metres: Double) -> String { "\(oneDecimal(shown(metres))) \(name)" }
}

func apmFollow(_ view: JSON?) -> ApmFollow? {
    guard let it = view, it["available"].bool else { return nil }
    return ApmFollow(
        available: true,
        enabled: it["enabled"].bool,
        waiting: it["waiting"].bool,
        supported: it["supported"].bool(true),
        unsupportedText: it["unsupportedText"].string,
        showSettings: it["showSettings"].bool,
        rover: it["rover"].bool,
        positionOptions: it["positionOptions"].array.map(\.string),
        positionIndex: it["positionIndex"].int(0),
        pointOptions: it["pointOptions"].array.map(\.string),
        pointIndex: it["pointIndex"].int(-1),
        angle: it["angle"].double(.nan),
        distance: it["distance"].double(.nan),
        height: it["height"].double(.nan),
        horizontal: transformUnit(it, "horizontal"),
        vertical: transformUnit(it, "vertical")
    )
}

func oneDecimal(_ value: Double) -> String { String(format: "%.1f", value) }

private func refused(_ path: String, _ args: [Any?]) -> String? {
    refusal(Qgc.call(path, arguments: args))
}

struct ApmFollowScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: ApmFollow?
    @State private var loaded = false
    @State private var refusal: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if loaded {
                if let follow = read {
                    screen(follow)
                } else {
                    Text("This vehicle has no follow me parameters.").padding(16)
                }
            }
        }
        .task(id: revision) {
            let follow = await offMain { apmFollow(Qgc.get(APM_FOLLOW_VIEW)) }
            read = follow
            loaded = true
        }
        .task(id: read?.waiting) {
            while read?.waiting == true && !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(WAITING_POLL_MS))
                if !Task.isCancelled { revision += 1 }
            }
        }
    }

    private func screen(_ follow: ApmFollow) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                Toggle("Enable follow me", isOn: Binding(get: { follow.enabled }, set: { act(APM_FOLLOW_ENABLE, $0) }))
                if follow.waiting {
                    Text("Waiting for Vehicle to update").font(.bodyMedium)
                }
                if !follow.supported {
                    Text(follow.unsupportedText).foregroundStyle(theme.colors.error).padding(.vertical, 8)
                    Button("Reset to supported settings") { act(APM_FOLLOW_RESET) }.buttonStyle(.borderedProminent)
                }
                if follow.showSettings {
                    SectionHeader(text: "Follow me settings")
                    Choice(label: "Vehicle position", options: follow.positionOptions, index: follow.positionIndex) { act(APM_FOLLOW_POSITION, $0) }
                    if !follow.rover {
                        Choice(label: "Point vehicle", options: follow.pointOptions, index: follow.pointIndex) { act(APM_FOLLOW_POINT, $0) }
                    }
                    if follow.positionIndex == 1 {
                        Text("Vehicle offsets").font(.titleSmall).padding(.top, 12)
                        HStack(alignment: .bottom) {
                            OffsetGraphic(follow: follow) { act(APM_FOLLOW_OFFSETS, $0, follow.distance) }
                            if !follow.rover {
                                HeightGraphic(heightText: follow.vertical.text(follow.height))
                            }
                        }
                        NumberEntry(label: "Angle", units: "deg", value: follow.angle) { act(APM_FOLLOW_OFFSETS, $0, follow.distance) }
                        NumberEntry(label: "Distance", units: follow.horizontal.name, value: follow.horizontal.shown(follow.distance)) {
                            act(APM_FOLLOW_OFFSETS, follow.angle, follow.horizontal.metres($0))
                        }
                        if !follow.rover {
                            NumberEntry(label: "Height", units: follow.vertical.name, value: follow.vertical.shown(follow.height)) {
                                act(APM_FOLLOW_HEIGHT, follow.vertical.metres($0))
                            }
                        }
                    }
                }
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error).padding(.top, 8)
                }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
        }
    }

    private func act(_ path: String, _ args: Any...) {
        Task {
            refusal = await offMain { refused(path, args) }
            revision += 1
        }
    }
}

private struct Choice: View {
    let label: String
    let options: [String]
    let index: Int
    let onPick: (Int) -> Void

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                ForEach(Array(options.enumerated()), id: \.offset) { at, option in
                    Button(option) { onPick(at) }
                }
            } label: {
                Text(options.indices.contains(index) ? options[index] : "")
            }
            .buttonStyle(.bordered)
        }
        .padding(.vertical, 4)
    }
}

private struct NumberEntry: View {
    let label: String
    let units: String
    let value: Double
    let onDone: (Double) -> Void
    @State private var typed = ""

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            HStack(spacing: 4) {
                TextField("", text: $typed)
                    .keyboardType(.numbersAndPunctuation)
                    .submitLabel(.done)
                    .onSubmit { if let number = Double(typed) { onDone(number) } }
                Text(units)
            }
            .textFieldStyle(.roundedBorder)
            .frame(maxWidth: .infinity)
        }
        .padding(.vertical, 4)
        .onChange(of: value, initial: true) { typed = oneDecimal(value) }
    }
}

func headingOfTap(_ x: Float, _ y: Float) -> Double {
    let geometric = atan2(Double(y), Double(x)) * 180 / .pi
    let heading = 90 - geometric
    return heading < 0 ? heading + 360 : heading > 360 ? heading - 360 : heading
}

func vehicleYaw(_ follow: ApmFollow) -> Double {
    if follow.rover || follow.pointIndex == 0 { return 0 }
    return follow.pointIndex == 1 ? 180 : -follow.angle
}

private let OFFSET_GRAPHIC: CGFloat = 240
private let DISTANCE_LABEL_RADIUS: CGFloat = (OFFSET_GRAPHIC / 2 - 16) / 2

private func triangle(_ tip: CGPoint, _ left: CGPoint, _ right: CGPoint) -> Path {
    Path { path in
        path.move(to: tip)
        path.addLine(to: left)
        path.addLine(to: right)
        path.closeSubpath()
    }
}

private func segment(_ from: CGPoint, _ to: CGPoint) -> Path {
    Path { path in
        path.move(to: from)
        path.addLine(to: to)
    }
}

private struct OffsetGraphic: View {
    let follow: ApmFollow
    let onAngle: (Double) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Click in the graphic to change angle").font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            ZStack {
                Canvas { context, size in draw(context, size) }
                    .frame(width: OFFSET_GRAPHIC, height: OFFSET_GRAPHIC)
                    .contentShape(Rectangle())
                    .onTapGesture { tap in
                        onAngle(headingOfTap(Float(tap.x - OFFSET_GRAPHIC / 2), Float(OFFSET_GRAPHIC / 2 - tap.y)))
                    }
                let labelRadians = follow.angle * .pi / 180
                Text(follow.horizontal.text(follow.distance))
                    .font(.labelSmall)
                    .background(theme.colors.surface)
                    .offset(x: DISTANCE_LABEL_RADIUS * CGFloat(sin(labelRadians)), y: -DISTANCE_LABEL_RADIUS * CGFloat(cos(labelRadians)))
                    .allowsHitTesting(false)
            }
            .frame(width: OFFSET_GRAPHIC, height: OFFSET_GRAPHIC)
        }
    }

    private func draw(_ context: GraphicsContext, _ size: CGSize) {
        let center = CGPoint(x: size.width / 2, y: size.height / 2)
        let shade = theme.colors.outlineVariant
        let ink = theme.colors.onSurface
        context.stroke(segment(CGPoint(x: center.x, y: 0), CGPoint(x: center.x, y: size.height)), with: .color(shade), lineWidth: 3)
        context.stroke(segment(CGPoint(x: 0, y: center.y), CGPoint(x: size.width, y: center.y)), with: .color(shade), lineWidth: 3)
        let arrow: CGFloat = 14
        context.fill(
            triangle(CGPoint(x: center.x, y: center.y - arrow), CGPoint(x: center.x - arrow * 0.7, y: center.y + arrow * 0.7), CGPoint(x: center.x + arrow * 0.7, y: center.y + arrow * 0.7)),
            with: .color(theme.colors.primary)
        )
        var around = context
        around.translateBy(x: center.x, y: center.y)
        around.rotate(by: .degrees(follow.angle))
        around.translateBy(x: -center.x, y: -center.y)
        let at = CGPoint(x: center.x, y: 20)
        around.stroke(segment(CGPoint(x: center.x, y: at.y + 14), CGPoint(x: center.x, y: center.y - arrow - 4)), with: .color(ink.opacity(0.4)), lineWidth: 2)
        var vehicle = around
        vehicle.translateBy(x: at.x, y: at.y)
        vehicle.rotate(by: .degrees(vehicleYaw(follow)))
        vehicle.translateBy(x: -at.x, y: -at.y)
        vehicle.fill(triangle(CGPoint(x: at.x, y: at.y - 12), CGPoint(x: at.x - 9, y: at.y + 10), CGPoint(x: at.x + 9, y: at.y + 10)), with: .color(ink))
    }
}

private struct HeightGraphic: View {
    let heightText: String
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack {
            Canvas { context, size in
                let ink = theme.colors.onSurface
                let x = size.width / 2
                let tick: CGFloat = 8
                context.stroke(segment(CGPoint(x: x, y: 0), CGPoint(x: x, y: size.height)), with: .color(ink.opacity(0.4)), lineWidth: 2)
                context.stroke(segment(CGPoint(x: x - tick, y: 1), CGPoint(x: x + tick, y: 1)), with: .color(ink), lineWidth: 2)
                context.stroke(segment(CGPoint(x: x - tick, y: size.height - 1), CGPoint(x: x + tick, y: size.height - 1)), with: .color(ink), lineWidth: 2)
            }
            Text(heightText).font(.labelSmall).background(theme.colors.surface)
        }
        .frame(width: 56, height: OFFSET_GRAPHIC)
    }
}
