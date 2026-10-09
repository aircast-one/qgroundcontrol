import Charts
import SwiftUI

let INSPECTOR_CHARTS_VIEW = "view.inspectorCharts"
private let CHART_ADD = "mavlinkInspector.chart.add"
private let CHART_REMOVE = "mavlinkInspector.chart.remove"
private let CHART_RANGE_X = "mavlinkInspector.chart.rangeX"
private let CHART_RANGE_Y = "mavlinkInspector.chart.rangeY"

struct ChartSample: Equatable {
    let ageMs: Int64
    let value: Double
}

struct ChartPlot: Equatable {
    let label: String
    let field: String
    let colour: Int
    let points: [ChartSample]
}

struct ChartModel: Equatable {
    let rangeX: Int
    let rangeY: Int
    let windowMs: Int64
    let yMin: Double?
    let yMax: Double?
    let room: Bool
    let plots: [ChartPlot]
}

struct InspectorCharts: Equatable {
    let timeScales: [String]
    let ranges: [String]
    let charts: [ChartModel]
    let charted: [String: Int]
}

private func chartNumber(_ json: JSON, _ key: String) -> Double? {
    json[key].isNull ? nil : json[key].double.flatMap { $0.isNaN ? nil : $0 }
}

func inspectorCharts(_ view: JSON?) -> InspectorCharts? {
    guard let view, let charts = view["charts"].arrayOrNil else { return nil }
    return InspectorCharts(
        timeScales: view["timeScales"].array.map(\.string),
        ranges: view["ranges"].array.map(\.string),
        charts: charts.filter { $0.object != nil }.map { chart in
            ChartModel(
                rangeX: chart["rangeX"].int(0),
                rangeY: chart["rangeY"].int(0),
                windowMs: chart["windowMs"].int64 ?? 5000,
                yMin: chartNumber(chart, "yMin"),
                yMax: chartNumber(chart, "yMax"),
                room: chart["room"].bool(true),
                plots: chart["plots"].array.filter { $0.object != nil }.map { plot in
                    ChartPlot(
                        label: plot["label"].string,
                        field: plot["field"].string,
                        colour: plot["colour"].int(0),
                        points: plot["points"].array.filter { $0.arrayOrNil != nil }.map { ChartSample(ageMs: $0[0].int64 ?? 0, value: $0[1].double(.nan)) }
                    )
                }
            )
        },
        charted: Dictionary(
            view["selectedCharted"].array.filter { $0.object != nil }.map { ($0["field"].string, $0["chart"].int(0)) },
            uniquingKeysWith: { _, last in last }
        )
    )
}

private let CHART_HEIGHT: CGFloat = 220
private let TIME_TICKS = 3
private let CLOCK_TICK_MS = 1000
private let VALUE_TICKS = 4

func timeTicks(_ windowMs: Int64, _ nowMs: Int64, zone: TimeZone = .current) -> [(Float, String)] {
    (0...TIME_TICKS).map { step in
        let at = Float(step) / Float(TIME_TICKS)
        let instant = nowMs - Int64((1 - at) * Float(windowMs))
        let wallClock = instant + Int64(zone.secondsFromGMT(for: Date(timeIntervalSince1970: Double(instant) / 1000))) * 1000
        return (at, String(format: "%02d:%02d", Int((wallClock / 60_000) % 60), Int((wallClock / 1000) % 60)))
    }
}

func valueTicks(_ low: Double, _ high: Double) -> [(Float, String)] {
    (0...VALUE_TICKS).map { step in
        let fraction = Double(step) / Double(VALUE_TICKS)
        return (1 - Float(fraction), String(format: "%#.4g", low + (high - low) * fraction))
    }
}

func latestValue(_ points: [ChartSample]) -> String? {
    points.min { $0.ageMs < $1.ageMs }.map { String(format: "%#.4g", $0.value) }
}

private func seriesColours(_ theme: Theme) -> [Color] {
    [theme.colors.primary, theme.aircast.mission, theme.aircast.success, theme.colors.tertiary, theme.colors.error, theme.aircast.warning]
}

func chartPoint(_ ageMs: Int64, _ value: Double, _ windowMs: Int64, _ yMin: Double, _ yMax: Double) -> (Float, Float) {
    let span = yMax - yMin
    let height = Float((value - yMin) / (span > 0 ? span : 1))
    return (1 - Float(ageMs) / Float(max(windowMs, 1)), 1 - min(max(height, 0), 1))
}

func fieldChartable(_ type: String) -> Bool { !type.hasPrefix("char") }

func chartToggleEnabled(_ charts: InspectorCharts?, _ field: String, _ type: String, _ chart: Int) -> Bool {
    let on = charts?.charted[field]
    if on == chart { return true }
    if !fieldChartable(type) || on != nil { return false }
    guard let models = charts?.charts, models.indices.contains(chart) else { return true }
    return models[chart].room
}

func toggleChartField(_ chart: Int, _ field: String, _ on: Bool) {
    offMainInOrder { Qgc.invoke(on ? CHART_ADD : CHART_REMOVE, chart, field) }
}

private struct ChartChoice: View {
    let title: String
    let options: [String]
    let chosen: Int
    let onChosen: (Int) -> Void

    var body: some View {
        Menu {
            ForEach(Array(options.enumerated()), id: \.offset) { index, option in
                Button(option) { onChosen(index) }
            }
        } label: {
            Text("\(title): \(options.indices.contains(chosen) ? options[chosen] : "")")
        }
        .buttonStyle(.borderless)
    }
}

struct InspectorChartPanel: View {
    let index: Int
    let charts: InspectorCharts
    @Environment(\.theme) private var theme

    var body: some View {
        if charts.charts.indices.contains(index), !charts.charts[index].plots.isEmpty {
            panel(charts.charts[index])
        }
    }

    private func panel(_ chart: ChartModel) -> some View {
        let colours = seriesColours(theme)
        let at = index
        return VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 4) {
                Text("Chart \(index + 1)").font(.titleSmall)
                ChartChoice(title: "Scale", options: charts.timeScales, chosen: chart.rangeX) { chosen in offMain { Qgc.invoke(CHART_RANGE_X, at, chosen) } }
                ChartChoice(title: "Range", options: charts.ranges, chosen: chart.rangeY) { chosen in offMain { Qgc.invoke(CHART_RANGE_Y, at, chosen) } }
            }
            .padding(.top, 12)
            VStack(spacing: 8) {
                TimelineView(.periodic(from: .now, by: Double(CLOCK_TICK_MS) / 1000)) { timeline in
                    plot(chart, colours, Int64(timeline.date.timeIntervalSince1970 * 1000))
                }
                .frame(height: CHART_HEIGHT)
                PlanFlowRow(spacing: 16, lineSpacing: 0, alignment: .center) {
                    ForEach(Array(chart.plots.enumerated()), id: \.offset) { _, plot in
                        HStack(spacing: 6) {
                            Circle().fill(colours[plot.colour % colours.count]).frame(width: 8, height: 8)
                            Text([plot.label, latestValue(plot.points)].compactMap { $0 }.joined(separator: "  ")).font(.labelMedium)
                        }
                    }
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity)
            .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.large))
            .padding(.top, 8)
        }
        .padding(.horizontal, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private func plot(_ chart: ChartModel, _ colours: [Color], _ nowMs: Int64) -> some View {
        if let low = chart.yMin, let high = chart.yMax {
            let values = valueTicks(low, high)
            let times = timeTicks(chart.windowMs, nowMs)
            Chart {
                ForEach(Array(chart.plots.enumerated()), id: \.offset) { slot, plot in
                    ForEach(Array(plot.points.enumerated()), id: \.offset) { _, sample in
                        let (x, y) = chartPoint(sample.ageMs, sample.value, chart.windowMs, low, high)
                        LineMark(x: .value("Time", Double(x)), y: .value("Value", Double(1 - y)), series: .value("Plot", slot))
                            .foregroundStyle(colours[plot.colour % colours.count])
                            .lineStyle(StrokeStyle(lineWidth: 2))
                    }
                }
            }
            .chartXScale(domain: 0...1)
            .chartYScale(domain: 0...1)
            .chartXAxis {
                AxisMarks(values: times.map { Double($0.0) }) { mark in
                    AxisValueLabel { tick(times[mark.index].1) }
                }
            }
            .chartYAxis {
                AxisMarks(position: .leading, values: values.map { Double(1 - $0.0) }) { mark in
                    AxisGridLine().foregroundStyle(theme.colors.outlineVariant)
                    AxisValueLabel { tick(values[mark.index].1) }
                }
            }
            .chartLegend(.hidden)
            .chartPlotStyle { $0.clipped() }
        }
    }

    private func tick(_ label: String) -> some View {
        Text(label).font(.labelSmall).monospaced().foregroundStyle(theme.colors.onSurfaceVariant)
    }
}
