import SwiftUI

private let SAMPLE_MS = 10
let FLIGHT_MODE_PATH = "vehicle.flightMode"
private let HISTORY_SECONDS = 180.0
private let SERIES_COLOURS = [Color(hex: 0x2196F3), Color(hex: 0xFF9800)]

struct Sample: Equatable {
    let seconds: Double
    let value: Double
}

func withSample(_ series: [Sample], _ sample: Sample) -> [Sample] {
    Array((series + [sample]).drop { $0.seconds < sample.seconds - HISTORY_SECONDS })
}

private let TICK_SEPARATION = 5.0

private func grownLow(_ current: Double, _ value: Double) -> Double {
    let low = min(current, value)
    return low.truncatingRemainder(dividingBy: TICK_SEPARATION) != 0 ? ((low - TICK_SEPARATION) / TICK_SEPARATION).rounded(.down) * TICK_SEPARATION : low
}

private func grownHigh(_ current: Double, _ value: Double) -> Double {
    let high = max(current, value)
    return high.truncatingRemainder(dividingBy: TICK_SEPARATION) != 0 ? ((high + TICK_SEPARATION) / TICK_SEPARATION).rounded(.down) * TICK_SEPARATION : high
}

func grownRange(_ range: (Double, Double)?, _ values: [Double]) -> (Double, Double)? {
    values.reduce(range) { held, value in
        held.map { (grownLow($0.0, value), grownHigh($0.1, value)) } ?? (value, value)
    }
}

private func factValue(_ path: String) -> Double? {
    let value = Qgc.get(path)["value"].double(.nan)
    return value.isNaN ? nil : value
}

private struct SampleKey: Equatable {
    let axis: TuningAxis
    let cleared: Int
    let running: Bool
}

struct TuningChart: View {
    let axis: TuningAxis
    let unit: String
    let windowSeconds: Double
    let modes: TuningModes?
    @Environment(\.theme) private var theme
    @QgcPath(FLY_STATE) private var flyJson
    @State private var running = true
    @State private var autoModeChange = false
    @State private var cleared = 0
    @State private var series: [[Sample]] = []
    @State private var now = 0.0
    @State private var range: (Double, Double)?

    private var armed: Bool { flyState(flyJson)?.armed == true }

    var body: some View {
        let from = now - windowSeconds
        VStack(alignment: .leading, spacing: 6) {
            Text(axis.chartTitle).font(.titleSmall)
            Canvas { context, size in draw(context, size, from) }
                .frame(maxWidth: .infinity)
                .frame(height: 180)
                .background(theme.colors.surfaceContainer)
            HStack(spacing: 12) {
                ForEach(Array(axis.plot.enumerated()), id: \.offset) { index, plot in
                    HStack(spacing: 4) {
                        Rectangle().fill(SERIES_COLOURS[index % SERIES_COLOURS.count]).frame(width: 10, height: 10)
                        Text(plot.name).font(.labelSmall)
                    }
                }
                Text(range.map { String(format: "%.2f … %.2f %@", $0.0, $0.1, unit) } ?? "").font(.labelSmall)
            }
            HStack(spacing: 8) {
                Button("Clear") { cleared += 1 }
                Button(running ? "Stop" : "Start") { toggleRunning() }
            }
            .buttonStyle(.bordered)
            if let names = modes {
                Toggle("Automatic flight mode switching", isOn: Binding(get: { autoModeChange }, set: { checked in
                    autoModeChange = checked
                    if checked { running = false }
                }))
                if autoModeChange {
                    Text("Switches to 'Stabilized' when you click Start.").font(.bodySmall)
                    Text("Switches to '\(names.pause)' when you click Stop.").font(.bodySmall)
                }
            }
        }
        .onChange(of: armed, initial: true) { if armed && !running { running = true } }
        .onChange(of: axis) {
            running = true
            reset()
        }
        .onChange(of: cleared) { reset() }
        .task(id: SampleKey(axis: axis, cleared: cleared, running: running)) { await sample() }
    }

    private func reset() {
        series = axis.plot.map { _ in [] }
        now = 0
        range = nil
    }

    private func sample() async {
        guard running else { return }
        let plots = axis.plot
        let started = Date().addingTimeInterval(-now)
        while !Task.isCancelled {
            let values = await offMain { plots.map { factValue($0.path) } }
            guard !Task.isCancelled else { return }
            now = Date().timeIntervalSince(started)
            let held = series.count == values.count ? series : values.map { _ in [] }
            series = zip(held, values).map { list, value in value.map { withSample(list, Sample(seconds: now, value: $0)) } ?? list }
            range = grownRange(range, values.compactMap { $0 })
            try? await Task.sleep(for: .milliseconds(SAMPLE_MS))
        }
    }

    private func toggleRunning() {
        running.toggle()
        guard let modes, autoModeChange else { return }
        let mode = running ? modes.stabilized : modes.pause
        offMain { _ = Qgc.writeRefusal(FLIGHT_MODE_PATH, mode) }
    }

    private func draw(_ context: GraphicsContext, _ size: CGSize, _ from: Double) {
        var midline = Path()
        midline.move(to: CGPoint(x: 0, y: size.height / 2))
        midline.addLine(to: CGPoint(x: size.width, y: size.height / 2))
        context.stroke(midline, with: .color(theme.colors.outlineVariant), lineWidth: 1)
        guard let (low, high) = range else { return }
        let span = high - low > 0 ? high - low : 1
        series.enumerated().forEach { index, list in
            let shown = list.filter { $0.seconds >= from }
            guard shown.count >= 2 else { return }
            let points = shown.map {
                CGPoint(x: ($0.seconds - from) / windowSeconds * size.width, y: size.height - ($0.value - low) / span * size.height)
            }
            var path = Path()
            path.addLines(points)
            context.stroke(path, with: .color(SERIES_COLOURS[index % SERIES_COLOURS.count]), lineWidth: 2)
        }
    }
}
