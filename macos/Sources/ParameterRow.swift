import SwiftUI

struct ValueField: View {
    let value: String
    let units: String
    var width = 72.0
    let commit: (String) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack(spacing: 3) {
            TextField("", text: Binding(
                get: { EditedField.shown(typed: draft, held: value, editing: editing) },
                set: { draft = $0 }))
                .textFieldStyle(.plain)
                .multilineTextAlignment(.trailing)
                .font(.body.monospacedDigit())
                .frame(width: width)
                .focused($editing)
                .onSubmit { editing = false }
                .onChange(of: editing) { focused in
                    if focused { draft = value } else { send() }
                }
            if !units.isEmpty {
                Text(units).font(.caption).foregroundColor(.secondary).fixedSize()
            }
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(Color.primary.opacity(editing ? 0.10 : 0.06))
        .cornerRadius(6)
    }

    private func send() {
        let trimmed = draft.trimmingCharacters(in: .whitespaces)
        guard trimmed != value, !trimmed.isEmpty else { return }
        commit(trimmed)
    }
}

struct ParameterEditor: View {
    let value: String
    let units: String
    let options: [ParameterOption]
    let selectedRaw: String
    var offersManualEntry = false
    var rangeHint = ""
    let commit: (String) -> Void

    @ViewBuilder var body: some View {
        if options.isEmpty {
            ValueField(value: value, units: units, commit: commit)
                .help(rangeHint)
        } else {
            HStack(spacing: Overlay.unit / 2) {
                Picker("", selection: Binding(get: { selectedRaw }, set: { commit($0) })) {
                    if !options.contains(where: { $0.raw == selectedRaw }) {
                        Text(value).tag(selectedRaw)
                    }
                    ForEach(options) { option in
                        Text(option.label).tag(option.raw)
                    }
                }
                .labelsHidden()
                .frame(maxWidth: offersManualEntry ? 150 : 220)
                if offersManualEntry {
                    ValueField(value: selectedRaw, units: "", width: 56, commit: commit)
                        .help(ParameterOption.manualEntryHelp)
                }
            }
        }
    }
}

struct BitmaskToggles: View {
    let bits: [ControlBit]
    var disabled = false
    let toggle: (ControlBit, Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            ForEach(bits) { bit in
                Toggle(bit.label, isOn: Binding(get: { bit.set }, set: { toggle(bit, $0) }))
                    .disabled(disabled)
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing)
    }
}

struct ParameterRow: View {
    let name: String
    let label: String
    var detail = ""
    let value: String
    let units: String
    let options: [ParameterOption]
    let selectedRaw: String
    var offersManualEntry = false
    var showSeparator = true
    let bits: [ControlBit]
    var rangeHint = ""
    var togglesDisabled = false
    var expandsBits = true
    var reservesDot = false
    var showsNonDefaultDot = false
    let summary: String
    let toggle: (ControlBit, Bool) -> String
    let commit: (String) -> Void

    init(parameter: Parameter, showSeparator: Bool = true, expandsBits: Bool = true,
         commit: @escaping (String) -> Void) {
        name = parameter.name
        label = parameter.description
        value = parameter.value
        detail = parameter.rowDetail
        units = parameter.units
        options = parameter.options
        offersManualEntry = parameter.offersManualEntry
        rangeHint = parameter.rangeHint
        selectedRaw = parameter.selectedOption?.raw ?? ""
        bits = parameter.drawsBits ? parameter.bits : []
        summary = parameter.drawsBits && !expandsBits ? parameter.bitSummary : ""
        reservesDot = true
        showsNonDefaultDot = parameter.showsNonDefaultDot
        toggle = { parameter.toggling($0, on: $1) }
        self.expandsBits = expandsBits
        self.showSeparator = showSeparator
        self.commit = commit
    }

    init(control: SettingsControl, showSeparator: Bool = true,
         commit: @escaping (String) -> Void) {
        detail = control.rowDescription(label: control.label)
        name = control.name
        label = control.label
        value = control.display.isEmpty ? control.valueString : control.display
        units = control.units
        options = control.parameterOptions
        offersManualEntry = control.offersManualEntry
        rangeHint = control.rangeHint
        selectedRaw = control.valueString
        bits = control.drawsBits ? control.bits : []
        summary = ""
        togglesDisabled = control.readOnly
        toggle = { String(control.toggling($0, on: $1)) }
        self.showSeparator = showSeparator
        self.commit = commit
    }

    var body: some View {
        GroupRow(
            title: label.isEmpty ? name : label,
            description: detail,
            value: summary,
            showSeparator: showSeparator,
            leading: { dot },
            trailing: {
                if bits.isEmpty || !expandsBits {
                    ParameterEditor(value: value, units: units, options: options,
                                    selectedRaw: selectedRaw,
                                    offersManualEntry: offersManualEntry,
                                    rangeHint: rangeHint, commit: commit)
                } else {
                    BitmaskToggles(bits: bits, disabled: togglesDisabled) {
                        commit(toggle($0, $1))
                    }
                }
            })
    }

    @ViewBuilder private var dot: some View {
        if reservesDot {
            Circle()
                .fill(showsNonDefaultDot ? Color.orange : Color.clear)
                .frame(width: 7, height: 7)
        }
    }
}

struct SetupPageBody<Content: View>: View {
    let title: String
    var note = ""
    var connected = true
    @ViewBuilder var content: Content

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Overlay.unit * 1.25) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(title).font(.title2.weight(.semibold))
                    if !note.isEmpty {
                        Text(note).font(.callout).foregroundColor(.secondary)
                    }
                }
                if connected {
                    content
                } else {
                    GroupCard {
                        EmptyStateRow(text: "Connect a vehicle to set this up.")
                    }
                }
            }
            .padding(Overlay.unit * 1.25)
            .frame(maxWidth: 660, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct SetupSections: View {
    let sections: [SettingsSection]
    @ObservedObject var store: ParametersStore
    @State private var calibratingEscs = false

    var body: some View {
        Group {
            ForEach(sections) { section in
                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: section.title)
                    GroupCard {
                        ForEach(Array(section.controls.enumerated()), id: \.element.id) { index, control in
                            row(control, showSeparator: index > 0)
                        }
                    }
                    if !section.note.isEmpty {
                        Text(section.note)
                            .font(.caption).foregroundColor(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                            .padding(.horizontal, Overlay.horizontalPadding)
                            .padding(.top, Overlay.unit * 0.35)
                    }
                }
            }
        }
        .sheet(isPresented: $calibratingEscs) {
            EscCalibrationSheet { calibratingEscs = false }
        }
    }

    @ViewBuilder private func row(_ control: SettingsControl, showSeparator: Bool) -> some View {
        switch control.kind {
        case .label:
            GroupRow(title: control.label, showSeparator: showSeparator)
        case .dialog:
            GroupRow(title: control.label, showSeparator: showSeparator, trailing: {
                Button("Open") { calibratingEscs = true }
                    .disabled(!control.enabled || control.dialog != EscCalibrationReading.dialog)
            })
        default:
            ParameterRow(control: control, showSeparator: showSeparator) {
                store.writeControl(control, $0)
            }
        }
    }
}

struct EscCalibrationReading: Equatable {
    static let dialog = "escCalibration"
    static let starting = "Starting ESC calibration..."

    let highlight: String
    let text: String
    let running: Bool

    init?(_ json: [String: Any]) {
        guard (json["open"] as? NSNumber)?.boolValue == true else { return nil }
        highlight = (json["highlight"] as? String) ?? ""
        text = (json["text"] as? String) ?? ""
        running = (json["running"] as? NSNumber)?.boolValue ?? false
    }
}

final class EscCalibrationStore: ObservableObject {
    @Published private(set) var reading: EscCalibrationReading?
    private var poll: Timer?

    func start() {
        Bridge.invoke("escCalibration.start")
        poll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.reading = EscCalibrationReading(Bridge.group("view.escCalibration"))
        }
    }

    func close() {
        poll?.invalidate()
        poll = nil
        Bridge.invoke("escCalibration.close")
    }
}

struct EscCalibrationSheet: View {
    let done: () -> Void
    @StateObject private var calibration = EscCalibrationStore()

    var body: some View {
        VStack(alignment: .leading, spacing: Overlay.unit) {
            Text("ESC Calibration").font(.headline)
            (Text(calibration.reading?.highlight ?? "").bold().foregroundColor(.red)
                + Text(calibration.reading?.text ?? EscCalibrationReading.starting))
                .fixedSize(horizontal: false, vertical: true)
            HStack {
                Spacer()
                Button("OK") {
                    calibration.close()
                    done()
                }
                .disabled(calibration.reading?.running != false)
            }
        }
        .padding(Overlay.unit * 1.25)
        .frame(width: 380)
        .onAppear(perform: calibration.start)
    }
}
