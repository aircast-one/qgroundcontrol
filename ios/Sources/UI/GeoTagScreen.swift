import SwiftUI
import UniformTypeIdentifiers

private let GEOTAG_BLUE = Color(hex: 0x2196F3)
private let GEOTAG_GREEN = Color(hex: 0x00C853)
private let GEOTAG_ORANGE = Color(hex: 0xFF9800)

private enum GeoTagPick { case log, images, output }

struct GeoTagScreen: View {
    @Environment(\.theme) private var theme
    @QgcString(settingControl("settings.appSettings.logSavePath")) private var logSavePath
    @State private var offsetText: String?
    @State private var logs: [URL] = []
    @State private var pick = GeoTagPick.log
    @State private var picking = false

    private var run: GeoTagRun { .shared }

    var body: some View {
        content
            .task(id: logSavePath) {
                let folder = logSavePath
                logs = await offMain { downloadedLogs(folder) }
            }
            .task { await run.refresh() }
            .fileImporter(isPresented: $picking, allowedContentTypes: pick == .log ? [.item] : [.folder]) { result in
                switch (result, pick) {
                case (.success(let chosen), .log): run.pickLog(chosen)
                case (.success(let chosen), .images): run.pickImages(chosen)
                case (.success(let chosen), .output): run.pickOutput(chosen)
                case (.failure(let error), _): run.note = error.localizedDescription
                }
            }
    }

    private func browse(_ what: GeoTagPick) {
        pick = what
        picking = true
    }

    @ViewBuilder private var content: some View {
        if let current = run.state {
            form(current)
        } else {
            Color.clear
        }
    }

    // The three files as disclosure rows that open the picker, the options as form rows, and Start (or Preview, or
    // Cancel while tagging) as the page's primary action.
    private func form(_ current: GeoTagState) -> some View {
        let editable = !current.inProgress && !run.busy
        let canRun = current.inProgress || (!run.busy && !current.logFile.isBlank && !current.imageDirectory.isBlank)
        let messages = [geoTagError(current), run.note].compactMap { $0 }
        return NativeForm {
            if current.inProgress {
                Section {
                    VStack(alignment: .leading, spacing: Space.s2) {
                        Text("Geotagging in progress...")
                        ProgressView(value: min(max(current.progress / 100, 0), 1))
                    }
                    .padding(.vertical, Space.s1)
                }
            }
            if !messages.isEmpty || geoTagSummary(current) != nil {
                Section {
                    ForEach(messages, id: \.self) { message in
                        Text(message).foregroundStyle(theme.colors.error)
                    }
                    if let summary = geoTagSummary(current) {
                        Text(summary).foregroundStyle(theme.colors.primary)
                    }
                }
            }
            Section {
                GeoTagStepRow(
                    mark: geoTagStep(!current.logFile.isBlank, 1),
                    title: "Flight log",
                    detail: current.logFile.isBlank ? "No file selected" : current.logFile.components(separatedBy: "/").last ?? "",
                    enabled: editable
                ) { browse(.log) }
                GeoTagStepRow(
                    mark: geoTagStep(!current.imageDirectory.isBlank, 2),
                    title: "Photos folder",
                    detail: run.imageTree.map(treeName) ?? "No folder selected",
                    enabled: editable
                ) { browse(.images) }
                GeoTagStepRow(
                    mark: "3",
                    title: "Output folder (optional)",
                    detail: geoTagOutputText(run.outputTree.map(treeName), run.imageTree.map(treeName)),
                    enabled: editable
                ) { browse(.output) }
            } footer: {
                Text("Tag images from a survey mission with GPS coordinates from your flight log.")
            }
            if !logs.isEmpty && editable {
                Section {
                    ForEach(logs, id: \.self) { log in
                        Button(log.lastPathComponent) { run.pickDownloadedLog(log.path) }
                    }
                } header: {
                    SectionHeader(text: "Downloaded flight logs")
                }
            }
            Section {
                FieldRow(
                    label: "Time offset (s)",
                    value: offsetText ?? shownOffset(current.timeOffsetSecs),
                    hint: "Adjust if camera clock differs from flight log",
                    enabled: editable,
                    keyboard: .numbersAndPunctuation,
                    onChange: { typed in
                        offsetText = typed
                        if let seconds = parsedOffset(typed) { run.set("timeOffsetSecs", seconds) }
                    }
                )
                Toggle(isOn: Binding(get: { current.previewMode }, set: { run.set("previewMode", $0) })) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Preview mode (don't write files)")
                        Text("Verify time offset before committing").font(.subheadline).foregroundStyle(Color.secondary)
                    }
                }
                .disabled(!editable)
            } header: {
                SectionHeader(text: "Advanced options")
            }
            if !current.images.isEmpty {
                Section {
                    GeoTagLegend()
                    ForEach(Array(current.images.enumerated()), id: \.offset) { _, image in
                        GeoTagImageRow(image: image)
                    }
                } header: {
                    SectionHeader(text: "Images (\(current.images.count))")
                }
            }
        }
        .pagePrimaryAction(geoTagButton(current), enabled: canRun) {
            if current.inProgress { run.cancel() } else { run.start() }
        }
    }
}

private func geoTagStatusColour(_ status: Int, _ theme: Theme) -> Color {
    switch status {
    case 1: GEOTAG_BLUE
    case GEOTAG_TAGGED: GEOTAG_GREEN
    case 3: GEOTAG_ORANGE
    case 4: theme.colors.error
    default: theme.colors.onSurface.opacity(0.5)
    }
}

let GEOTAG_LEGEND: [(Int, String)] = [(0, "Pending"), (1, "Processing"), (GEOTAG_TAGGED, "Tagged"), (3, "Skipped"), (4, "Failed")]

private struct GeoTagLegend: View {
    @Environment(\.theme) private var theme

    var body: some View {
        PlanFlowRow(spacing: 12, lineSpacing: 0) {
            ForEach(GEOTAG_LEGEND, id: \.0) { status, label in
                HStack(spacing: 4) {
                    RoundedRectangle(cornerRadius: 2).fill(geoTagStatusColour(status, theme)).frame(width: 10, height: 10)
                    Text(label).font(.bodySmall)
                }
            }
        }
    }
}

private struct GeoTagImageRow: View {
    let image: GeoTagImage
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 12) {
            RoundedRectangle(cornerRadius: 2).fill(geoTagStatusColour(image.status, theme)).frame(width: 12, height: 12)
            VStack(alignment: .leading) {
                Text(image.fileName).font(.bodyMedium).lineLimit(1).truncationMode(.tail)
                if let coordinate = geoTagCoordinate(image) {
                    Text(coordinate).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(geoTagImageText(image))
                .font(.bodySmall)
                .foregroundStyle(image.status == 3 || image.status == 4 ? geoTagStatusColour(image.status, theme) : theme.colors.onSurface)
        }
    }
}

// One of the three files: its step (a number, or a check once chosen), name and choice, opening the picker.
private struct GeoTagStepRow: View {
    let mark: String
    let title: String
    let detail: String
    let enabled: Bool
    let onBrowse: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let done = !mark.allSatisfy(\.isNumber)
        Button(action: onBrowse) {
            HStack(spacing: Space.s3) {
                Image(systemName: done ? "checkmark.circle.fill" : "\(mark).circle")
                    .font(.title2)
                    .foregroundStyle(done ? theme.colors.primary : Color.secondary)
                    .accessibilityHidden(true)
                NativeRowLabel(title: title, subtitle: Text(detail), dimmed: !enabled)
            }
            .contentShape(Rectangle())
        }
        .listRowButton()
        .disabled(!enabled)
    }
}
