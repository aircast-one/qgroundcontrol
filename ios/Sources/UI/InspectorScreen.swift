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
    (view?[key].array ?? []).filter { $0.object != nil }.map { InspectorChoice(id: $0["id"].int(0), title: $0["title"].string) }
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

private struct RatePicker: View {
    let messagePath: String
    let current: String
    let choices: [InspectorRate]

    var body: some View {
        if !choices.isEmpty {
            Menu {
                ForEach(choices, id: \.rate) { choice in
                    Button(choice.title) { requestRate(messagePath, choice.rate) }
                }
            } label: {
                Text(current.isBlank ? "Set Rate:" : "Rate: \(current)")
            }
            .buttonStyle(.borderless)
            .padding(8)
        }
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

private struct FieldList: View {
    let messagePath: String
    @QgcPath(INSPECTOR_VIEW) private var inspectorJson
    @State private var charts: InspectorCharts?
    @Environment(\.theme) private var theme

    var body: some View {
        let rows = parseInspectorFields(inspectorJson, messagePath)
        Group {
            if rows.isEmpty {
                InspectorNotice(text: "Waiting for this message to arrive again.")
                    .frame(maxHeight: .infinity, alignment: .top)
            } else {
                List {
                    if let shown = charts {
                        InspectorChartPanel(index: 0, charts: shown).listRowInsets(EdgeInsets())
                        InspectorChartPanel(index: 1, charts: shown).listRowInsets(EdgeInsets())
                    }
                    HStack(spacing: 0) {
                        Spacer()
                        ForEach(CHART_LABELS, id: \.self) { label in
                            Text(label).font(.labelSmall).lineLimit(1).frame(width: 56)
                        }
                    }
                    ForEach(rows, id: \.name) { field in
                        fieldRow(field)
                    }
                }
                .listStyle(.plain)
                .task {
                    while !Task.isCancelled {
                        charts = await offMain { inspectorCharts(Qgc.get(INSPECTOR_CHARTS_VIEW)) }
                        try? await Task.sleep(for: .milliseconds(CHART_POLL_MS))
                    }
                }
            }
        }
        .task(id: messagePath) { selectMessage(messagePath) }
    }

    private func fieldRow(_ field: InspectorField) -> some View {
        HStack(spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(field.name).monospaced()
                Text(field.type).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(field.value).monospaced()
            ForEach(0..<2, id: \.self) { chart in
                Toggle("", isOn: Binding(
                    get: { charts?.charted[field.name] == chart },
                    set: { toggleChartField(chart, field.name, $0) }
                ))
                .labelsHidden()
                .disabled(!chartToggleEnabled(charts, field.name, field.type, chart))
                .frame(width: 56)
                .accessibilityLabel("\(field.name) on \(CHART_LABELS[chart])")
            }
        }
    }
}

private struct InspectorChip: View {
    let selected: Bool
    let label: String
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack(spacing: 4) {
                if selected { Image(.check) }
                Text(label).font(.labelLarge)
            }
            .padding(.horizontal, 12)
            .frame(height: 32)
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
            .background(selected ? theme.colors.secondaryContainer : .clear, in: RoundedRectangle(cornerRadius: Corner.small))
            .overlay(RoundedRectangle(cornerRadius: Corner.small).stroke(selected ? .clear : theme.colors.outline))
        }
        .buttonStyle(.plain)
    }
}

struct InspectorScreen: View {
    @Environment(\.theme) private var theme
    @QgcPath(INSPECTOR_VIEW) private var inspectorJson
    @State private var openPath: String?
    @State private var filter = ""
    @State private var component: Int?

    var body: some View {
        let activeSystem = inspectorJson.flatMap { $0["systemId"].isNull ? nil : $0["systemId"].int(0) }
        content
            .onChange(of: activeSystem) { component = nil }
    }

    @ViewBuilder private var content: some View {
        let messages = inspectorMessages(inspectorJson)
        if let emptyText = inspectorEmptyText(inspectorJson) {
            InspectorNotice(text: emptyText).frame(maxHeight: .infinity, alignment: .top)
        } else if let open = openMessageIn(messages, openPath) {
            detail(open)
        } else {
            list(messages)
        }
    }

    private func detail(_ open: InspectorMessage) -> some View {
        VStack(spacing: 0) {
            Button { openPath = nil } label: {
                Text(open.name)
                    .font(.titleSmall)
                    .monospaced()
                    .foregroundStyle(theme.colors.onSurface)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(16)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            HStack {
                RatePicker(messagePath: open.path, current: open.targetRateTitle, choices: inspectorRateChoices(inspectorJson))
                Spacer()
            }
            .padding(.horizontal, 8)
            VStack(spacing: 0) {
                ForEach(inspectorDetails(open), id: \.0) { label, value in
                    HStack {
                        Text(label).font(.bodySmall).frame(maxWidth: .infinity, alignment: .leading)
                        Text(value).font(.bodySmall).monospaced()
                    }
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 4)
            Divider()
            FieldList(messagePath: open.path).frame(maxHeight: .infinity)
        }
    }

    private func list(_ messages: [InspectorMessage]) -> some View {
        let systems = inspectorChoices(inspectorJson, "systems")
        let components = inspectorChoices(inspectorJson, "components")
        let activeSystem = inspectorJson.flatMap { $0["systemId"].isNull ? nil : $0["systemId"].int(0) }
        let shown = inspectorShown(messages, filter, component)
        return VStack(alignment: .leading, spacing: 0) {
            if let line = inspectorSystemText(inspectorJson) {
                Text(line)
                    .font(.labelSmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(.horizontal, 20)
                    .padding(.vertical, 4)
            }
            if systems.count > 1 || components.count > 1 {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        if systems.count > 1 {
                            ForEach(systems, id: \.id) { system in
                                InspectorChip(selected: system.id == activeSystem, label: system.title) {
                                    offMain { SetupCommands.setInspectorSystem(system.id) }
                                }
                            }
                        }
                        if components.count > 1 {
                            ForEach([InspectorChoice(id: -1, title: "Comp All")] + components, id: \.id) { choice in
                                InspectorChip(selected: (component ?? -1) == choice.id, label: choice.title) {
                                    component = choice.id >= 0 ? choice.id : nil
                                }
                            }
                        }
                    }
                    .padding(.horizontal, 16)
                }
            }
            SearchPill(value: filter, onValueChange: { filter = $0 }, placeholder: "Filter messages")
            if shown.isEmpty {
                InspectorNotice(text: "No message matches \"\(filter)\".")
                Spacer(minLength: 0)
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(shown, id: \.path) { message in
                            SetupRow(
                                title: message.name.ifBlank(message.title) + (message.fieldSelected ? " *" : ""),
                                status: message.rateText,
                                onClick: { openPath = message.path },
                                subtitle: "comp \(message.compId)"
                            )
                        }
                    }
                }
            }
        }
    }
}

let CHART_LABELS = ["Plot 1", "Plot 2"]
