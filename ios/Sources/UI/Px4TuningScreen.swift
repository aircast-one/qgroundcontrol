import SwiftUI

let PX4_TUNING_VIEW = "view.px4Tuning"
let SET_TUNING_TELEMETRY = "vehicle.setPIDTuningTelemetryMode"
private let DEFAULT_CHART_SECONDS = 8.0

struct TuningParam: Equatable {
    let title: String
    let description: String
    let min: Float
    let max: Float
    let step: Float
    let fact: Fact
}

struct TuningPlot: Equatable {
    let name: String
    let path: String
}

struct TuningAxis: Equatable {
    let name: String
    var chartTitle: String = ""
    var plot: [TuningPlot] = []
    let params: [TuningParam]
}

struct TuningTab: Equatable {
    let name: String
    let title: String
    let unit: String
    let extras: [Fact]
    let axes: [TuningAxis]
    var tuningMode: Int = 0
    var autoModeChange: Bool = false
    var autoTuning: Bool = false
    var chartSeconds: Double = DEFAULT_CHART_SECONDS
}

private func mapObjects<T>(_ list: JSON, _ read: (JSON) -> T?) -> [T] {
    list.array.filter { $0.object != nil }.compactMap(read)
}

func tuningTabs(_ view: JSON?) -> [TuningTab] {
    guard let view, view["available"].bool else { return [] }
    return mapObjects(view["tabs"]) { tab in
        TuningTab(
            name: sentenceCase(tab["name"].string),
            title: tab["title"].string,
            unit: tab["unit"].string,
            extras: mapObjects(tab["extras"], factFromControl),
            axes: mapObjects(tab["axes"]) { axis in
                TuningAxis(
                    name: sentenceCase(axis["name"].string),
                    chartTitle: sentenceCase(axis["chartTitle"].string),
                    plot: mapObjects(axis["plot"]) { TuningPlot(name: $0["name"].string, path: $0["path"].string) },
                    params: mapObjects(axis["params"]) { param in
                        (param["fact"].object != nil ? factFromControl(param["fact"]) : nil).map { fact in
                            TuningParam(
                                title: sentenceCase(param["title"].string),
                                description: param["description"].string,
                                min: Float(param["min"].double(.nan)),
                                max: Float(param["max"].double(.nan)),
                                step: Float(param["step"].double(.nan)),
                                fact: fact
                            )
                        }
                    }
                )
            },
            tuningMode: tab["tuningMode"].int(0),
            autoModeChange: tab["autoModeChange"].bool,
            autoTuning: tab["autoTuning"].bool,
            chartSeconds: tab["chartSeconds"].double(DEFAULT_CHART_SECONDS)
        )
    }
}

struct TuningModes: Equatable {
    let stabilized: String
    let pause: String
}

func tuningModes(_ view: JSON?) -> TuningModes {
    TuningModes(stabilized: view?["stabilizedFlightMode"].string ?? "", pause: view?["pauseFlightMode"].string ?? "")
}

func sliderSteps(_ min: Float, _ max: Float, _ step: Float) -> Int {
    guard step > 0, max > min else { return 0 }
    let count = ((max - min) / step).rounded()
    return count.isFinite ? Swift.max(Int(count) - 1, 0) : 0
}

func factNumber(_ fact: Fact) -> Float? { rawNumber(fact).map(Float.init) }

struct Px4TuningScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var tabs: [TuningTab] = []
    @State private var modes = TuningModes(stabilized: "", pause: "")
    @State private var useAutoTuning = false
    @State private var loaded = false
    @State private var tabIndex = 0
    @State private var axisIndex = 0
    @State private var clipboard: [(Fact, String)] = []
    @State private var refusal: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if !loaded {
                Text("Reading parameters from the vehicle.").padding(16)
            } else if tabs.indices.contains(tabIndex) {
                screen(tabs[tabIndex])
            } else {
                Text("This vehicle has no PX4 tuning page.").padding(16)
            }
        }
        .task(id: revision) {
            let read = await offMain { Qgc.get(PX4_TUNING_VIEW) }
            tabs = tuningTabs(read)
            modes = tuningModes(read)
            loaded = true
        }
    }

    private func currentAxis(_ tab: TuningTab) -> TuningAxis? {
        tab.axes.indices.contains(axisIndex) ? tab.axes[axisIndex] : tab.axes.first
    }

    private func screen(_ tab: TuningTab) -> some View {
        let axis = currentAxis(tab)
        return VStack(alignment: .leading, spacing: 0) {
            ScrollView(.horizontal, showsIndicators: false) {
                Picker("", selection: Binding(get: { tabIndex }, set: { index in
                    tabIndex = index
                    axisIndex = 0
                })) {
                    ForEach(Array(tabs.enumerated()), id: \.offset) { index, each in
                        Text(each.name).tag(index)
                    }
                }
                .pickerStyle(.segmented)
                .fixedSize()
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    ForEach(tab.extras) { fact in
                        FactRow(fact: fact, onWrite: { revision += 1 })
                    }
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 8) {
                            ForEach(Array(tab.axes.enumerated()), id: \.offset) { index, each in
                                PlanChip(label: each.name, selected: each == axis) { axisIndex = index }
                            }
                        }
                    }
                    if let axis {
                        TuningChart(axis: axis, unit: tab.unit, windowSeconds: tab.chartSeconds, modes: tab.autoModeChange ? modes : nil)
                    }
                    if tab.autoTuning {
                        HStack(spacing: 16) {
                            TuningChoice(label: "Use auto-tuning", selected: useAutoTuning) { useAutoTuning = true }
                            TuningChoice(label: "Use manual tuning", selected: !useAutoTuning) { useAutoTuning = false }
                        }
                    }
                    if tab.autoTuning && useAutoTuning {
                        AutotuneSection()
                    } else if let axis {
                        ForEach(axis.params, id: \.fact.path) { param in
                            TuningSlider(param: param) { write(param.fact.path, $0) }
                        }
                        HStack(spacing: 8) {
                            Button("Save to clipboard") { clipboard = axis.params.map { ($0.fact, $0.fact.valueString) } }
                            Button("Restore from clipboard") {
                                clipboard.forEach { fact, value in
                                    if let number = Double(value) { write(fact.path, number) }
                                }
                            }
                            .disabled(clipboard.isEmpty)
                        }
                        .buttonStyle(.bordered)
                    }
                    if let refusal {
                        Text(refusal).foregroundStyle(theme.colors.error)
                    }
                    if !clipboard.isEmpty {
                        Text("Clipboard values:").font(.labelLarge)
                        ForEach(Array(clipboard.enumerated()), id: \.offset) { _, entry in
                            Text("\(entry.0.name)  \(entry.1)").font(.bodySmall)
                        }
                    }
                }
                .padding(16)
            }
        }
        .onChange(of: [tabIndex, axisIndex], initial: true) {
            clipboard = currentAxis(tab).map { $0.params.map { ($0.fact, $0.fact.valueString) } } ?? clipboard
        }
        .onChange(of: tab.tuningMode, initial: true) {
            let mode = tab.tuningMode
            offMainInOrder { Qgc.invoke(SET_TUNING_TELEMETRY, mode) }
        }
        .onDisappear { offMainInOrder { Qgc.invoke(SET_TUNING_TELEMETRY, 0) } }
    }

    private func write(_ path: String, _ value: Any) {
        Task {
            refusal = await offMain { Qgc.writeRefusal(path, value) }
            revision += 1
        }
    }
}

private struct TuningChoice: View {
    let label: String
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack {
                Image(systemName: selected ? "largecircle.fill.circle" : "circle")
                    .foregroundStyle(selected ? theme.colors.primary : theme.colors.onSurfaceVariant)
                Text(label).foregroundStyle(theme.colors.onSurface)
            }
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

private func sliderRange(_ min: Float, _ max: Float) -> ClosedRange<Float> {
    min.isFinite && max.isFinite && max > min ? min...max : 0...1
}

private struct TuningSlider: View {
    let param: TuningParam
    let onWrite: (Float) -> Void
    @Environment(\.theme) private var theme
    @State private var dragging: Float = 0

    var body: some View {
        let range = sliderRange(param.min, param.max)
        let shown = Binding(get: { Swift.min(Swift.max(dragging, range.lowerBound), range.upperBound) }, set: { dragging = $0 })
        let finished: (Bool) -> Void = { editing in if !editing { onWrite(dragging) } }
        VStack(alignment: .leading, spacing: 2) {
            Text(param.title).font(.bodyMedium)
            Text(param.description).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            HStack {
                Group {
                    if sliderSteps(param.min, param.max, param.step) > 0 {
                        Slider(value: shown, in: range, step: param.step, onEditingChanged: finished)
                    } else {
                        Slider(value: shown, in: range, onEditingChanged: finished)
                    }
                }
                .frame(maxWidth: .infinity)
                Text(param.fact.valueString).padding(.leading, 8)
            }
        }
        .onChange(of: TuningSliderKey(path: param.fact.path, current: factNumber(param.fact)), initial: true) {
            dragging = factNumber(param.fact) ?? param.min
        }
    }
}

private struct TuningSliderKey: Equatable {
    let path: String
    let current: Float?
}
