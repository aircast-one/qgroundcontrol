import SwiftUI

func inspectorSystemText(_ view: JSON?) -> String? {
    let system = view.flatMap { $0["available"].bool ? $0["systemId"].int(0) : nil } ?? 0
    return system > 0 ? "System \(system)" : nil
}

let INSPECTOR_VIEW = "view.inspector"

func inspectorRateText(_ view: JSON?) -> String? {
    guard let view, view["available"].bool, let listed = view["messages"].arrayOrNil else { return nil }
    let total = listed.filter { $0.object != nil }.map { $0["rateHz"].double(0) }.filter(\.isFinite).reduce(0, +)
    return total > 0 ? "\(Int64(total.rounded())) messages/s" : nil
}

func inspectorEmptyText(_ view: JSON?) -> String? {
    view.flatMap { $0["emptyText"].string.isBlank ? nil : $0["emptyText"].string }
}

struct InspectorMessage: Equatable {
    let index: Int
    let id: Int
    let name: String
    let rateText: String
    let count: Int64
    let path: String
    let compId: Int
    let title: String
    var targetRateTitle: String = ""
    var fieldSelected: Bool = false
}

struct InspectorRate: Equatable {
    let rate: Int
    let title: String
}

struct InspectorChoice: Equatable {
    let id: Int
    let title: String
}

func inspectorChoices(_ view: JSON?, _ key: String) -> [InspectorChoice] {
    (view?[key].objects ?? []).map { InspectorChoice(id: $0["id"].int(0), title: $0["title"].string) }
}

func inspectorDetails(_ message: InspectorMessage) -> [(String, String)] {
    [
        ("Message", "\(message.name) (\(message.id))"),
        ("Component", String(message.compId)),
        ("Count", String(message.count)),
        ("Actual Rate", message.rateText),
    ]
}

func inspectorShown(_ messages: [InspectorMessage], _ filter: String, _ component: Int?) -> [InspectorMessage] {
    messages.filter { (filter.isEmpty || $0.name.range(of: filter, options: .caseInsensitive) != nil) && (component == nil || $0.compId == component) }
}

func inspectorRateChoices(_ view: JSON?) -> [InspectorRate] {
    (view?["rateChoices"].array ?? [])
        .filter { $0.object != nil }
        .map { InspectorRate(rate: $0["rate"].int(0), title: $0["title"].string) }
        .filter { !$0.title.isBlank }
}

struct InspectorField: Equatable {
    let name: String
    let type: String
    let value: String
}

func inspectorMessages(_ view: JSON?) -> [InspectorMessage] {
    guard let items = view?["messages"].arrayOrNil else { return [] }
    return items.enumerated().filter { $0.element.object != nil }.map { index, message in
        InspectorMessage(
            index: message["index"].int(index),
            id: message["id"].int(0),
            name: message["name"].string,
            rateText: message["rateText"].string,
            count: message["count"].int64 ?? 0,
            path: message["path"].string,
            compId: message["compId"].int(0),
            title: message["title"].string.ifBlank(message["name"].string),
            targetRateTitle: message["targetRateTitle"].string,
            fieldSelected: message["fieldSelected"].bool
        )
    }.sorted { $0.name < $1.name }
}

func parseInspectorFields(_ view: JSON?, _ messagePath: String) -> [InspectorField] {
    guard let messages = view?["messages"].arrayOrNil,
          messages.contains(where: { $0["selected"].bool && $0["path"].string == messagePath }),
          let fields = view?["fields"].arrayOrNil else { return [] }
    return fields.filter { $0.object != nil }.map { InspectorField(name: $0["name"].string, type: $0["type"].string, value: $0["value"].string) }
}

private let CHART_POLL_MS = 200

func selectedPathFor(_ messagePath: String) -> String {
    (messagePath.range(of: ".messages.").map { String(messagePath[..<$0.lowerBound]) } ?? messagePath) + ".selected"
}

func messageIndexIn(_ messagePath: String) -> Int {
    messagePath.components(separatedBy: ".").last.flatMap { Int($0) } ?? -1
}

private func selectMessage(_ messagePath: String) {
    offMain { Qgc.set(selectedPathFor(messagePath), messageIndexIn(messagePath)) }
}

private func requestRate(_ messagePath: String, _ rate: Int) {
    offMain {
        Qgc.set(selectedPathFor(messagePath), messageIndexIn(messagePath))
        SetupCommands.setInspectorMessageInterval(rate)
    }
}

func openMessageIn(_ messages: [InspectorMessage], _ path: String?) -> InspectorMessage? {
    path.flatMap { wanted in messages.first { $0.path == wanted } }
}

private struct InspectorNotice: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.bodyLarge)
            .multilineTextAlignment(.center)
            .padding(24)
            .frame(maxWidth: .infinity)
    }
}

// One message: its details and requested rate, the charts (drawn: no system control plots them), and its fields as
// rows, each with the two plot switches.
private struct InspectorMessageDetail: View {
    let message: InspectorMessage
    let rateChoices: [InspectorRate]
    @QgcPath(INSPECTOR_VIEW) private var inspectorJson
    @State private var charts: InspectorCharts?

    var body: some View {
        let rows = parseInspectorFields(inspectorJson, message.path)
        NativeForm {
            Section {
                ForEach(inspectorDetails(message), id: \.0) { label, value in
                    LabeledContent(label) { Text(value).monospaced() }
                }
                if !rateChoices.isEmpty {
                    LabeledContent("Requested rate") {
                        Menu {
                            ForEach(rateChoices, id: \.rate) { choice in
                                Button(choice.title) { requestRate(message.path, choice.rate) }
                            }
                        } label: {
                            Text(message.targetRateTitle.ifBlank("Set rate"))
                        }
                    }
                }
            }
            if rows.isEmpty {
                Section {
                    Text("Waiting for this message to arrive again.").foregroundStyle(Color.secondary)
                }
            } else {
                if let shown = charts {
                    let plotted = [0, 1].filter { shown.charts.indices.contains($0) && !shown.charts[$0].plots.isEmpty }
                    if !plotted.isEmpty {
                        Section {
                            ForEach(plotted, id: \.self) { index in
                                InspectorChartPanel(index: index, charts: shown).legacyFormRow()
                            }
                        }
                    }
                }
                Section {
                    ForEach(rows, id: \.name) { field in
                        fieldRow(field)
                    }
                } header: {
                    HStack(spacing: 0) {
                        SectionHeader(text: "Fields")
                        Spacer(minLength: 0)
                        ForEach(CHART_LABELS, id: \.self) { label in
                            Text(label).lineLimit(1).frame(width: CHART_SWITCH_WIDTH)
                        }
                    }
                }
            }
        }
        .task(id: message.path) { selectMessage(message.path) }
        .task(id: rows.isEmpty) {
            guard !rows.isEmpty else { return }
            while !Task.isCancelled {
                charts = await offMain { inspectorCharts(Qgc.get(INSPECTOR_CHARTS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(CHART_POLL_MS))
            }
        }
    }

    private func fieldRow(_ field: InspectorField) -> some View {
        HStack(spacing: Space.s2) {
            VStack(alignment: .leading, spacing: 2) {
                Text(field.name).monospaced()
                Text(field.type).font(.subheadline).foregroundStyle(Color.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(field.value).monospaced().foregroundStyle(Color.secondary)
            ForEach(0..<2, id: \.self) { chart in
                Toggle("", isOn: Binding(
                    get: { charts?.charted[field.name] == chart },
                    set: { toggleChartField(chart, field.name, $0) }
                ))
                .labelsHidden()
                .disabled(!chartToggleEnabled(charts, field.name, field.type, chart))
                .frame(width: CHART_SWITCH_WIDTH)
                .accessibilityLabel("\(field.name) on \(CHART_LABELS[chart])")
            }
        }
    }
}

private let CHART_SWITCH_WIDTH: CGFloat = 56

func inspectorActiveSystem(_ view: JSON?) -> Int? {
    view.flatMap { $0["systemId"].isNull ? nil : $0["systemId"].int(0) }
}

private let ALL_COMPONENTS = -1

// The messages as a native list: the vehicle and component as pickers, the filter as the navigation bar's search
// field, and each message a row that opens it. An open message steps back to the list with the bar's back.
struct InspectorScreen: View {
    @QgcPath(INSPECTOR_VIEW) private var inspectorJson
    @State private var openPath: String?
    @State private var filter = ""
    @State private var component: Int?

    var body: some View {
        content
            .onChange(of: inspectorActiveSystem(inspectorJson)) { component = nil }
    }

    @ViewBuilder private var content: some View {
        let messages = inspectorMessages(inspectorJson)
        if let emptyText = inspectorEmptyText(inspectorJson) {
            InspectorNotice(text: emptyText).frame(maxHeight: .infinity, alignment: .top)
        } else if let open = openMessageIn(messages, openPath) {
            InspectorMessageDetail(message: open, rateChoices: inspectorRateChoices(inspectorJson))
                .background { OverridePageHeading(title: open.name, onBack: { openPath = nil }) }
        } else {
            list(messages)
        }
    }

    private func list(_ messages: [InspectorMessage]) -> some View {
        let systems = inspectorChoices(inspectorJson, "systems")
        let components = inspectorChoices(inspectorJson, "components")
        let activeSystem = inspectorActiveSystem(inspectorJson)
        let systemLine = inspectorSystemText(inspectorJson)
        let shown = inspectorShown(messages, filter, component)
        return NativeForm {
            if systems.count > 1 || components.count > 1 || systemLine != nil {
                Section {
                    if systems.count > 1 {
                        Picker("Vehicle", selection: Binding(
                            get: { activeSystem ?? systems.first?.id ?? 0 },
                            set: { picked in offMain { SetupCommands.setInspectorSystem(picked) } }
                        )) {
                            ForEach(systems, id: \.id) { system in Text(system.title).tag(system.id) }
                        }
                    } else if let systemLine {
                        LabeledContent("Vehicle", value: systemLine)
                    }
                    if components.count > 1 {
                        Picker("Component", selection: Binding(
                            get: { component ?? ALL_COMPONENTS },
                            set: { picked in component = picked == ALL_COMPONENTS ? nil : picked }
                        )) {
                            Text("All").tag(ALL_COMPONENTS)
                            ForEach(components, id: \.id) { choice in Text(choice.title).tag(choice.id) }
                        }
                    }
                }
            }
            Section {
                if shown.isEmpty {
                    Text("No message matches \"\(filter)\".").foregroundStyle(Color.secondary)
                }
                ForEach(shown, id: \.path) { message in
                    SetupRow(
                        title: message.name.ifBlank(message.title) + (message.fieldSelected ? " *" : ""),
                        status: message.rateText,
                        onClick: { openPath = message.path },
                        subtitle: "comp \(message.compId)"
                    )
                }
            } header: {
                SectionHeader(text: "Messages")
            }
        }
        .searchable(text: $filter, prompt: "Filter messages")
    }
}

let CHART_LABELS = ["Plot 1", "Plot 2"]
