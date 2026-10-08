import SwiftUI

let VIBRATION_VIEW = "view.vibration"
private let SCALE_STEPS = 4

private let AXIS_WIDTH: CGFloat = 32
private let BAR_WIDTH: CGFloat = 56

struct VibrationAxis: Equatable {
    let axis: String
    let value: Double?
    let fraction: Float
    let severity: String?
}

struct VibrationReading: Equatable {
    let units: String
    let scaleMaximum: Double
    let warningLevel: Double
    let dangerLevel: Double
    let axes: [VibrationAxis]
    let clipCounts: [Int]
}

private func doubleOrNull(_ json: JSON, _ key: String) -> Double? {
    json[key].double.flatMap { $0.isNaN ? nil : $0 }
}

private func stringOrNull(_ json: JSON, _ key: String) -> String? {
    json[key].isNull ? nil : json[key].string.isBlank ? nil : json[key].string
}

private func whole(_ value: Double) -> Int { value.isFinite ? Int(value) : 0 }

struct SilentState: Equatable {
    let title: String
    let body: String
}

func silentState(_ view: JSON?) -> SilentState? {
    guard let view else { return SilentState(title: "No vehicle connected", body: CONNECT_PROMPT) }
    let token = view["silentReason"].string
    if token.isBlank { return nil }
    let title = view["silentText"].string.ifBlank("Vibration is not being reported")
    let body = switch token {
    case "noVehicle": CONNECT_PROMPT
    case "notReported": "The autopilot has not sent a VIBRATION message. Not all firmware and airframes publish one."
    default: "Nothing has reported a vibration level on this vehicle."
    }
    return SilentState(title: title, body: body)
}

let PARTIAL_TITLE = "Only some vibration axes are reported"
let PARTIAL_BODY = "This screen draws all three axes together, and this vehicle is reporting some of them."

private let CONNECT_PROMPT = "Connect a vehicle from the Fly view to see its vibration levels."

func vibrationReading(_ view: JSON?) -> VibrationReading? {
    guard let view, view["available"].bool, let axes = view["axes"].arrayOrNil else { return nil }
    return VibrationReading(
        units: view["units"].string,
        scaleMaximum: view["scaleMaximum"].double(0),
        warningLevel: view["warningLevel"].double(0),
        dangerLevel: view["dangerLevel"].double(0),
        axes: axes.filter { $0.object != nil }.map { axis in
            VibrationAxis(
                axis: stringOrNull(axis, "label") ?? axis["axis"].string.uppercased(with: Locale(identifier: "en_US")),
                value: doubleOrNull(axis, "value"),
                fraction: doubleOrNull(axis, "fraction").map(Float.init) ?? 0,
                severity: stringOrNull(axis, "severity")
            )
        },
        clipCounts: view["clipCounts"].array.map { $0.int(0) }
    )
}

func vibrationGlance(_ reading: VibrationReading) -> String? {
    let parts = reading.axes.compactMap { axis in
        axis.value.flatMap { $0.isFinite ? "\(axis.axis) \(Int64($0.rounded()))" : nil }
    }
    guard !parts.isEmpty else { return nil }
    return parts.joined(separator: " \u{00b7} ") + (reading.units.isBlank ? "" : " \(reading.units)")
}

func worstSeverity(_ reading: VibrationReading) -> String? {
    ["danger", "warning", "normal"].first { level in reading.axes.contains { $0.severity == level } }
}

func vibrationVerdict(_ reading: VibrationReading) -> String {
    let worst = worstSeverity(reading)
    let axes = reading.axes.filter { $0.severity == worst }.map(\.axis).joined(separator: " and ")
    let level = switch worst {
    case "danger": "Vibration on \(axes) is over the unsafe limit of \(whole(reading.dangerLevel))."
    case "warning": "Vibration on \(axes) is above \(whole(reading.warningLevel)); watch it."
    default: "Vibration is well under the limit."
    }
    let clips = reading.clipCounts.reduce(0, +)
    let clipping = clips == 0 ? "No clipping." : "The accelerometers clipped \(clips) times; expect zero in flight."
    return "\(level) \(clipping)"
}

func severityLabel(_ severity: String?) -> String {
    switch severity {
    case "danger": "Unsafe"
    case "warning": "Watch"
    case "normal": "Healthy"
    default: ""
    }
}

func vibrationHeading(_ units: String) -> String {
    units.isBlank ? "Vibration" : "Vibration (\(units))"
}

func bandCaption(_ warningLevel: Double, _ dangerLevel: Double) -> String {
    let warn = whole(warningLevel)
    let danger = whole(dangerLevel)
    return "Under \(warn) healthy · \(warn)-\(danger) watch · over \(danger) unsafe"
}

func scaleLabels(_ scaleMaximum: Double, _ warningLevel: Double, _ dangerLevel: Double) -> [String] {
    [scaleMaximum, dangerLevel, warningLevel, 0].map { String(whole($0)) }
}

private func colorFor(_ severity: String?, _ theme: Theme) -> Color {
    switch severity {
    case "danger": theme.colors.error
    case "warning": theme.aircast.warning
    case "normal": theme.colors.primary
    default: theme.colors.surfaceVariant
    }
}

private struct ScaleAxis: View {
    let labels: [String]

    var body: some View {
        VStack(alignment: .trailing) {
            ForEach(Array(labels.enumerated()), id: \.offset) { at, label in
                if at > 0 { Spacer(minLength: 0) }
                Text(label).font(.labelSmall)
            }
        }
        .frame(width: AXIS_WIDTH, alignment: .trailing)
        .frame(maxHeight: .infinity)
    }
}

private struct VibrationBarGraphic: View {
    let axis: VibrationAxis
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack(alignment: .bottom) {
            theme.colors.surfaceVariant
            GeometryReader { box in
                VStack {
                    Spacer(minLength: 0)
                    colorFor(axis.severity, theme).frame(height: box.size.height * CGFloat(min(max(axis.fraction, 0), 1)))
                }
            }
            VStack {
                ForEach(0..<SCALE_STEPS, id: \.self) { at in
                    if at > 0 { Spacer(minLength: 0) }
                    Rectangle().fill(theme.colors.onSurfaceVariant).frame(height: 1)
                }
            }
        }
        .frame(width: BAR_WIDTH)
        .frame(maxHeight: .infinity)
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .frame(maxWidth: .infinity)
    }
}

private struct VibrationReadout: View {
    let axis: VibrationAxis

    var body: some View {
        VStack {
            Text(axis.value.map { String(format: "%.1f", $0) } ?? "--").font(.titleMedium)
            Text(axis.axis).font(.labelLarge)
            Text(severityLabel(axis.severity)).font(.labelMedium)
        }
        .frame(maxWidth: .infinity)
    }
}

private struct VibrationSilence: View {
    let message: String
    let detail: String

    var body: some View {
        VStack(spacing: 8) {
            Text(sentenceCase(message)).font(.titleMedium).multilineTextAlignment(.center)
            Text(detail).font(.bodyMedium).multilineTextAlignment(.center)
        }
        .padding(24)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }
}

struct VibrationScreen: View {
    @QgcPath(VIBRATION_VIEW) private var view

    var body: some View {
        let reading = vibrationReading(view)
        if let empty = vibrationEmptyState(view, reading) {
            VibrationSilence(message: empty.title, detail: empty.body)
        } else if let reading {
            VibrationBody(reading: reading)
        }
    }
}

func vibrationCaveat(_ view: JSON?) -> String? {
    guard let view, !view["available"].bool else { return nil }
    if view["silentReason"].string == "notReported" { return "This vehicle is not reporting vibration" }
    if view["silentReason"].string.isBlank { return "This vehicle is reporting only some vibration axes" }
    return nil
}

func vibrationEmptyState(_ view: JSON?, _ reading: VibrationReading?) -> SilentState? {
    silentState(view) ?? (reading == nil ? partlyReported(view) : nil)
}

private func partlyReported(_ view: JSON?) -> SilentState {
    view?["connected"].bool == true
        ? SilentState(title: PARTIAL_TITLE, body: PARTIAL_BODY)
        : SilentState(title: "No vehicle connected", body: CONNECT_PROMPT)
}

private struct VibrationBody: View {
    let reading: VibrationReading
    @Environment(\.theme) private var theme

    var body: some View {
        let worst = worstSeverity(reading)
        VStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 12) {
                Text(vibrationHeading(reading.units)).font(.titleSmall)
                HStack(spacing: 0) {
                    ScaleAxis(labels: scaleLabels(reading.scaleMaximum, reading.warningLevel, reading.dangerLevel))
                    ForEach(Array(reading.axes.enumerated()), id: \.offset) { _, axis in
                        VibrationBarGraphic(axis: axis)
                    }
                }
                .frame(maxHeight: .infinity)
                HStack(spacing: 0) {
                    Color.clear.frame(width: AXIS_WIDTH, height: 1)
                    ForEach(Array(reading.axes.enumerated()), id: \.offset) { _, axis in
                        VibrationReadout(axis: axis)
                    }
                }
                Text(bandCaption(reading.warningLevel, reading.dangerLevel))
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .padding(16)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.medium))

            HStack(spacing: 12) {
                Image(worst == "danger" ? .error : worst == "warning" ? .warning : .checkCircle)
                Text(vibrationVerdict(reading)).font(.bodyMedium)
                Spacer(minLength: 0)
            }
            .padding(16)
            .foregroundStyle(worst == "danger" ? theme.colors.onErrorContainer : worst == "warning" ? theme.aircast.warning : theme.aircast.success)
            .background(
                worst == "danger" ? theme.colors.errorContainer : worst == "warning" ? theme.aircast.warningContainer : theme.aircast.successContainer,
                in: RoundedRectangle(cornerRadius: Corner.medium)
            )

            HStack {
                Text("Clipping events").font(.bodyLarge)
                Spacer(minLength: 8)
                Text(clipText(reading.clipCounts)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

func clipText(_ counts: [Int]) -> String {
    counts.enumerated().map { index, count in "Accel \(index + 1): \(count)" }.joined(separator: " \u{00b7} ")
}
