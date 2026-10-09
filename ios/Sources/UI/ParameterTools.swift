import SwiftUI
import UniformTypeIdentifiers

let PARAMETER_TOOLS_PATH = "view.parameterTools"
let PARAMETER_REFRESH_PATH = "parameterTools.refresh"
let PARAMETER_SAVE_PATH = "parameterFile.save"
let PARAMETER_REVIEW_PATH = "parameterFile.review"
let PARAMETER_APPLY_PATH = "parameterFile.apply"
private let PARAMETER_FILE_NAME = "vehicle.params"
private let PARAMETER_FILE_TYPES: [UTType] = [.plainText, .data, .item]
private let UNABLE_TO_OPEN = "Unable to open file."

struct ParameterDiffRow: Equatable {
    var json: JSON
    var name: String
    var fileValue: String
    var vehicleValue: String
    var units: String
    var cannotSend: Bool
    var componentId: Int = 1
    var noVehicleValue: Bool = false

    var key: String { "\(componentId):\(name)" }
}

struct ParameterReview: Equatable {
    var rows: [ParameterDiffRow]
    var otherVehicle: Bool
    var multipleComponents: Bool
    var parsed: Int
    var unchanged: Int
    var readOnly: Int

    init(rows: [ParameterDiffRow], otherVehicle: Bool, multipleComponents: Bool, parsed: Int? = nil, unchanged: Int = 0, readOnly: Int = 0) {
        self.rows = rows
        self.otherVehicle = otherVehicle
        self.multipleComponents = multipleComponents
        self.parsed = parsed ?? rows.count
        self.unchanged = unchanged
        self.readOnly = readOnly
    }
}

func parameterReview(_ result: JSON?) -> ParameterReview? {
    guard let review = result else { return nil }
    return ParameterReview(
        rows: review["rows"].objects.map {
            ParameterDiffRow(
                json: $0,
                name: $0["name"].string,
                fileValue: $0["fileValue"].string,
                vehicleValue: $0["vehicleValue"].string,
                units: $0["units"].string,
                cannotSend: $0["cannotSend"].bool,
                componentId: $0["componentId"].int(1),
                noVehicleValue: $0["noVehicleValue"].bool
            )
        },
        otherVehicle: review["otherVehicle"].bool,
        multipleComponents: review["multipleComponents"].bool,
        parsed: review["parsed"].int(0),
        unchanged: review["unchanged"].int(0),
        readOnly: review["readOnly"].int(0)
    )
}

func sendableCount(_ review: ParameterReview) -> Int { review.rows.filter { !$0.cannotSend }.count }

func reviewSummary(_ review: ParameterReview) -> String {
    let sendable = sendableCount(review)
    let newToVehicle = review.rows.filter { $0.noVehicleValue && !$0.cannotSend }.count
    let missing = review.rows.filter(\.cannotSend).count
    let changed: String? = sendable == 0 ? nil
        : newToVehicle > 0 ? "\(sendable) will be changed (including \(newToVehicle) not currently on the Vehicle)"
        : "\(sendable) will be changed"
    let matching: String? = review.unchanged == 1 ? "1 already matches the Vehicle"
        : review.unchanged > 1 ? "\(review.unchanged) already match the Vehicle"
        : nil
    let readOnly: String? = review.readOnly == 1 ? "1 read-only parameter will not be sent"
        : review.readOnly > 1 ? "\(review.readOnly) read-only parameters will not be sent"
        : nil
    let clauses = [changed, matching, readOnly, missing > 0 ? "\(missing) not found on the Vehicle and cannot be sent" : nil].compactMap { $0 }
    let loaded = review.parsed == 1 ? "Loaded 1 parameter from file" : "Loaded \(review.parsed) parameters from file"
    return clauses.isEmpty ? "\(loaded)." : "\(loaded): \(clauses.joined(separator: ", "))."
}

let SEND_HINT = "Click 'Ok' to update the parameters below on the Vehicle."
let CANNOT_SEND_HINT = "Parameters marked 'not on Vehicle' cannot be sent since the file does not include type information. They may only exist after another parameter is changed and the Vehicle is rebooted, after which you can load this file again."

func checkedAll(_ review: ParameterReview, _ chosen: Set<String>, _ on: Bool) -> Set<String> {
    let sendable = Set(review.rows.filter { !$0.cannotSend }.map(\.key))
    return on ? chosen.union(sendable) : chosen.subtracting(sendable)
}

func reviewWarnings(_ review: ParameterReview) -> [String] {
    [
        review.otherVehicle ? "The parameters in the file are from a different vehicle." : nil,
        review.multipleComponents ? "The file contains parameters for more than one component." : nil,
    ].compactMap { $0 }
}

struct ParameterTool: Equatable {
    var path: String
    var label: String
    var confirm: Bool
    var confirmTitle: String
    var confirmMessage: String
}

func parameterTools(_ view: JSON?) -> [ParameterTool] {
    (view?["tools"].objects ?? []).map {
        ParameterTool(
            path: $0["path"].string,
            label: $0["label"].string,
            confirm: $0["confirm"].bool,
            confirmTitle: $0["confirmTitle"].string,
            confirmMessage: $0["confirmMessage"].string
        )
    }
}

let NEW_TO_VEHICLE_HINT = "Parameters marked 'new to Vehicle' have not been reported by the Vehicle. They may only become visible after they are sent and the Vehicle is rebooted."

func diffLine(_ row: ParameterDiffRow) -> String {
    if row.cannotSend {
        return ["Vehicle N/A — not on Vehicle", "File \(row.fileValue)"].joined(separator: " · ")
    }
    let parts = row.noVehicleValue
        ? ["Vehicle N/A — new to Vehicle", "File \(row.fileValue)", row.units]
        : ["Vehicle \(row.vehicleValue)", "File \(row.fileValue)", row.units]
    return parts.filter { !$0.isBlank }.joined(separator: " · ")
}

func confirmLabel(_ tool: ParameterTool) -> String { tool.confirmTitle == "Reset All" ? "Reset" : "Ok" }

private struct ParameterFile: FileDocument {
    static var readableContentTypes: [UTType] { [.plainText] }
    let text: String

    init(text: String) { self.text = text }

    init(configuration: ReadConfiguration) throws {
        text = configuration.file.regularFileContents.flatMap { String(data: $0, encoding: .utf8) } ?? ""
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: Data(text.utf8))
    }
}

private func readText(_ url: URL) -> String? {
    let scoped = url.startAccessingSecurityScopedResource()
    defer { if scoped { url.stopAccessingSecurityScopedResource() } }
    return (try? Data(contentsOf: url)).map { String(decoding: $0, as: UTF8.self) }
}

struct ParameterToolsMenu: View {
    let onRefreshed: () -> Void
    @QgcPath(PARAMETER_TOOLS_PATH) private var view
    @State private var asking: ParameterTool?
    @State private var refusal: String?
    @State private var review: ParameterReview?
    @State private var chosen: Set<String> = []
    @State private var saving: ParameterFile?
    @State private var loading = false
    @State private var scope = ViewScope()
    @State private var probe = PresenterProbe()

    var body: some View {
        let tools = parameterTools(view)
        let reviewed = review
        if !tools.isEmpty {
            Menu {
                ForEach(tools, id: \.path) { tool in
                    Button(tool.label) {
                        if tool.confirm { asking = tool } else { run(tool) }
                    }
                }
            } label: {
                Text("Tools")
                    .frame(minWidth: MINIMUM_TOUCH_TARGET, minHeight: MINIMUM_TOUCH_TARGET)
                    .contentShape(Rectangle())
            }
            .alert(asking?.confirmTitle ?? "", isPresented: presented($asking), presenting: asking) { tool in
                Button("Cancel", role: .cancel) {}
                Button(confirmLabel(tool)) { run(tool) }
            } message: { tool in
                Text(tool.confirmMessage)
            }
            .alert("Parameters", isPresented: presented($refusal), presenting: refusal) { _ in
                Button("Close") { refusal = nil }
            } message: { shown in
                Text(shown)
            }
            .sheet(isPresented: presented($review)) {
                if let shown = reviewed {
                    ReviewDialog(review: shown, chosen: $chosen, onCancel: { review = nil }, onApply: { apply(shown) })
                }
            }
            .background {
                Color.clear.fileExporter(
                    isPresented: presented($saving),
                    document: saving,
                    contentType: .plainText,
                    defaultFilename: PARAMETER_FILE_NAME
                ) { result in
                    saving = nil
                    if case .failure = result { show { refusal = "The file could not be written." } }
                }
            }
            .fileImporter(isPresented: $loading, allowedContentTypes: PARAMETER_FILE_TYPES) { result in
                switch result {
                case .success(let url): load(url)
                case .failure: show { refusal = UNABLE_TO_OPEN }
                }
            }
            .background(PresenterProbeView(probe: probe).allowsHitTesting(false))
            .onDisappear { scope.cancel() }
        }
    }

    private func show(_ change: @escaping @MainActor () -> Void) {
        scope.launch { await whenPresenterFree(change) }
    }

    private func whenPresenterFree(_ change: @MainActor () -> Void) async {
        if await presenterFreed(probe) { change() }
    }

    private func run(_ tool: ParameterTool) {
        switch tool.path {
        case PARAMETER_SAVE_PATH:
            scope.launch {
                let saved = await offMain { () -> String? in
                    if case .string(let text) = Qgc.invokeResult(PARAMETER_SAVE_PATH) { return text }
                    return nil
                }
                await whenPresenterFree {
                    if let saved {
                        saving = ParameterFile(text: saved)
                    } else {
                        refusal = "The parameters could not be read from the vehicle."
                    }
                }
            }
        case PARAMETER_REVIEW_PATH:
            loading = true
        default:
            let path = tool.path
            scope.launch {
                let answer = await offMain { Qgc.refusalOf(path) }
                guard !Task.isCancelled else { return }
                guard let answer else {
                    onRefreshed()
                    return
                }
                await whenPresenterFree { refusal = answer }
            }
        }
    }

    private func load(_ url: URL) {
        scope.launch {
            guard let text = await offMain({ readText(url) }) else {
                await whenPresenterFree { refusal = UNABLE_TO_OPEN }
                return
            }
            let reviewed = await offMain { Qgc.call(PARAMETER_REVIEW_PATH, text) }
            let result = reviewed?["result"]
            let parsed = parameterReview(result?.objectOrNil)
            await whenPresenterFree {
                if let parsed {
                    review = parsed
                    chosen = Set(parsed.rows.filter { !$0.cannotSend }.map(\.key))
                } else {
                    refusal = (reviewed?["reason"].string ?? "").ifBlank("The file could not be reviewed.")
                }
            }
        }
    }

    private func apply(_ shown: ParameterReview) {
        let rows = shown.rows.filter { chosen.contains($0.key) }.map(\.json.any)
        review = nil
        scope.launch {
            let answer = await offMain { Qgc.refusalOf(PARAMETER_APPLY_PATH, rows) }
            guard !Task.isCancelled else { return }
            onRefreshed()
            if let answer { await whenPresenterFree { refusal = answer } }
        }
    }
}

private let REVIEW_LIST_MAX_HEIGHT: CGFloat = 360

private struct ReviewDialog: View {
    let review: ParameterReview
    @Binding var chosen: Set<String>
    let onCancel: () -> Void
    let onApply: () -> Void
    @State private var all = true
    @Environment(\.theme) private var theme

    var body: some View {
        SetupDialog(title: "Load Parameters") {
            ForEach(reviewWarnings(review), id: \.self) { Text($0).foregroundStyle(theme.aircast.warning) }
            Text(reviewSummary(review))
            if sendableCount(review) > 0 { Text(SEND_HINT) }
            if !review.rows.isEmpty {
                Toggle(isOn: Binding(get: { all }, set: { on in
                    all = on
                    chosen = checkedAll(review, chosen, on)
                })) { Text("Name").font(.labelLarge) }
            }
            ScrollView {
                LazyVStack(alignment: .leading, spacing: Space.s1) {
                    ForEach(review.rows, id: \.key) { row in
                        Toggle(isOn: Binding(get: { chosen.contains(row.key) }, set: { on in
                            chosen = on ? chosen.union([row.key]) : chosen.subtracting([row.key])
                        })) {
                            VStack(alignment: .leading, spacing: 0) {
                                Text(review.multipleComponents ? "\(row.componentId): \(row.name)" : row.name).font(.bodyMedium)
                                Text(diffLine(row)).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                        }
                        .disabled(row.cannotSend)
                    }
                }
                .padding(.horizontal, Space.s6)
            }
            .frame(maxHeight: REVIEW_LIST_MAX_HEIGHT)
            .padding(.horizontal, -Space.s6)
            if review.rows.contains(where: { $0.noVehicleValue && !$0.cannotSend }) {
                Text(NEW_TO_VEHICLE_HINT).font(.bodySmall)
            }
            if review.rows.contains(where: \.cannotSend) {
                Text(CANNOT_SEND_HINT).font(.bodySmall)
            }
        } buttons: {
            Button("Cancel", action: onCancel)
            if sendableCount(review) > 0 {
                Button("Ok", action: onApply)
                    .buttonStyle(.filled)
                    .disabled(chosen.isEmpty)
            }
        }
    }
}
