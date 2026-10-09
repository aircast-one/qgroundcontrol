import SwiftUI

private let PARAM_SCHEME = "param://"

func paramLinkName(_ url: String) -> String? {
    guard url.hasPrefix(PARAM_SCHEME) else { return nil }
    let name = url.removingPrefix(PARAM_SCHEME)
    return name.isBlank ? nil : name
}

let SETUP_DIALOG_OPENED = "setup.dialogOpened"
let SENSOR_SETTINGS_DIALOG = "sensorSettings"
private let PARAMETER_EDITOR_DIALOG = "parameterEditor"

func parameterLinkOpens(_ name: String) -> Bool {
    Qgc.call(SETUP_DIALOG_OPENED, PARAMETER_EDITOR_DIALOG, name)?["exists"].bool(true) ?? true
}

private struct HtmlRun {
    var text = AttributedString()
    var link: URL? = nil
    var bold = false
    var italic = false
}

private let HTML_ENTITIES: [String: String] = [
    "nbsp": "\u{00a0}", "lt": "<", "gt": ">", "quot": "\"", "apos": "'", "amp": "&",
    "deg": "\u{00b0}", "ndash": "\u{2013}", "mdash": "\u{2014}", "plusmn": "\u{00b1}",
    "micro": "\u{00b5}", "times": "\u{00d7}", "hellip": "\u{2026}", "middot": "\u{00b7}",
]
private let HEX_RADIX = 16

private func htmlEntity(_ name: Substring) -> String? {
    guard name.hasPrefix("#") else { return HTML_ENTITIES[String(name)] }
    let digits = name.dropFirst()
    let hex = digits.first == "x" || digits.first == "X"
    return (hex ? UInt32(digits.dropFirst(), radix: HEX_RADIX) : UInt32(digits))
        .flatMap { Unicode.Scalar($0) }
        .map { String(Character($0)) }
}

private func htmlText(_ raw: String) -> String {
    raw.replacing(#/&(#[xX][0-9a-fA-F]+|#[0-9]+|[A-Za-z][A-Za-z0-9]*);/#) { match in htmlEntity(match.1) ?? String(match.0) }
}

private func htmlHref(_ tag: String) -> URL? {
    let pattern = #/href\s*=\s*["']([^"']*)["']/#.ignoresCase()
    return tag.firstMatch(of: pattern).flatMap { URL(string: htmlText(String($0.1))) }
}

private func htmlTagName(_ tag: String) -> String {
    String(tag.dropFirst().drop { $0 == "/" }.prefix { $0.isLetter || $0.isNumber }).lowercased()
}

private func isHtmlTag(_ token: String) -> Bool { token.count > 1 && token.hasPrefix("<") }

private func htmlStep(_ run: HtmlRun, _ token: String) -> HtmlRun {
    var next = run
    guard isHtmlTag(token) else {
        var piece = AttributedString(htmlText(token))
        piece.link = run.link
        piece.inlinePresentationIntent = (run.bold ? InlinePresentationIntent.stronglyEmphasized : []).union(run.italic ? .emphasized : [])
        next.text += piece
        return next
    }
    let closing = token.hasPrefix("</")
    switch htmlTagName(token) {
    case "a": next.link = closing ? nil : htmlHref(token)
    case "b", "strong": next.bold = !closing
    case "i", "em": next.italic = !closing
    case "br": next.text += AttributedString("\n")
    case "p", "div", "li": next.text += closing ? AttributedString("\n") : AttributedString()
    default: break
    }
    return next
}

func htmlAttributed(_ html: String) -> AttributedString {
    html.matches(of: #/<[A-Za-z\/!][^>]*>|<|[^<]+/#).map { String($0.output) }.reduce(HtmlRun(), htmlStep).text
}

struct LinkedText: View {
    let html: String
    let style: TypeScale
    let color: Color
    let onParameter: (String) -> Void
    @State private var scope = ViewScope()

    var body: some View {
        Text(htmlAttributed(html))
            .font(style)
            .foregroundStyle(color)
            .environment(\.openURL, OpenURLAction { url in
                guard let name = paramLinkName(url.absoluteString) else { return .systemAction }
                let opener = onParameter
                scope.launch {
                    let opens = await offMain { parameterLinkOpens(name) }
                    if opens && !Task.isCancelled { opener(name) }
                }
                return .handled
            })
            .onDisappear { scope.cancel() }
    }
}

let READ_ONLY_NOTE = "This parameter is read-only and cannot be modified."
let FORCE_EDIT_NOTE = "Warning: This parameter is read-only. Force edit is enabled."

func forceEditNote(_ readOnly: Bool, _ forced: Bool) -> String? {
    guard readOnly else { return nil }
    return forced ? FORCE_EDIT_NOTE : READ_ONLY_NOTE
}

private struct CheckRow: View {
    let text: String
    let checked: Bool
    let onChecked: (Bool) -> Void

    var body: some View {
        Toggle(isOn: Binding(get: { checked }, set: onChecked)) { Text(text).font(.bodySmall) }
    }
}

let IN_FLIGHT_WARNING = "Warning: Modifying values while vehicle is in flight can lead to vehicle instability and possible vehicle loss. Make sure you know what you are doing and double-check your values before Save!"

func parameterDefault(_ json: JSON?) -> JSON? {
    guard let json, json["defaultValueAvailable"].bool, json.has("defaultValue") else { return nil }
    return json["defaultValue"]
}

func manualEntryFact(_ fact: Fact) -> Fact {
    var manual = fact
    manual.enumStrings = []
    manual.enumValues = []
    manual.bitmaskStrings = []
    manual.bitmaskValues = []
    return manual
}

func parameterRangeLine(_ fact: Fact) -> String {
    let units = !fact.units.isBlank && !fact.isEnum ? " \(fact.units)" : ""
    let min = !fact.minIsDefaultForType && !fact.minString.isBlank ? fact.minString : nil
    let max = !fact.maxIsDefaultForType && !fact.maxString.isBlank ? fact.maxString : nil
    let bounds: String? = switch (min, max) {
    case let (min?, max?): "Range \(min)\u{2013}\(max)\(units)"
    case let (min?, nil): "Min \(min)\(units)"
    case let (nil, max?): "Max \(max)\(units)"
    default: nil
    }
    return [bounds, fact.defaultValueString.isBlank ? nil : "default \(fact.defaultValueString)\(units)"]
        .compactMap { $0 }
        .joined(separator: " \u{00b7} ")
}

func parameterRebootNotes(_ fact: Fact) -> [String] {
    [
        fact.vehicleRebootRequired ? "Vehicle reboot required after change" : nil,
        fact.qgcRebootRequired ? "Application restart required after change" : nil,
    ].compactMap { $0 }
}

struct ParameterEditDialog: View {
    let name: String
    let title: String
    let onDismiss: () -> Void
    @State private var revision = 0
    @State private var advanced = false
    @State private var forced = false
    @State private var manual = false
    @State private var forceSave = false
    @State private var rejected = false
    @State private var forcedText = ""
    @State private var forceRefusal: String?
    @State private var fact: Fact?
    @State private var defaultValue: JSON?
    @State private var scope = ViewScope()
    @AdvancedUiShown private var forceAllowed
    @Environment(\.theme) private var theme

    init(name: String, title: String? = nil, onDismiss: @escaping () -> Void) {
        self.name = name
        self.title = title ?? name
        self.onDismiss = onDismiss
    }

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s2) {
                    Text(title).font(.titleLarge)
                    if let loaded = fact {
                        editor(loaded)
                    } else {
                        Text("\(name) is not a parameter on this vehicle.").font(.bodySmall)
                    }
                    HStack(spacing: Space.s2) {
                        Spacer(minLength: 0)
                        if let loaded = fact, !loaded.readOnly || forced, let value = defaultValue {
                            Button("Reset to default") { reset(loaded, value) }.buttonStyle(.text)
                        }
                        Button("Done", action: onDismiss).buttonStyle(.filled)
                    }
                }
                .padding(.horizontal, Space.s6)
                .padding(.bottom, Space.s6)
            }
            .task(id: "\(name)|\(revision)") {
                let name = name
                let loaded = await offMain { parameterFact(name) }
                let initial = await offMain { parameterDefault(Qgc.get(parameterPath(name))) }
                guard !Task.isCancelled else { return }
                fact = loaded
                defaultValue = initial
            }
        }
        .onDisappear { scope.cancel() }
    }

    private func reset(_ loaded: Fact, _ value: JSON) {
        let path = loaded.path, force = forced
        scope.launch {
            let answer = await offMain { force ? Qgc.writeForcedRefusal(path, value.any) : Qgc.writeRefusal(path, value.any) }
            guard !Task.isCancelled else { return }
            forceRefusal = answer
            if answer == nil { onDismiss() }
        }
    }

    @ViewBuilder
    private func editor(_ loaded: Fact) -> some View {
        let editable = !loaded.readOnly || forced
        let hasChoices = loaded.isEnum || loaded.isBitmask
        let note = loaded.longDescription.ifBlank(loaded.description)
        if !note.isBlank {
            Text(note).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
        }
        if let warning = forceEditNote(loaded.readOnly, forced) {
            Text(warning).font(.bodySmall).foregroundStyle(forced ? theme.aircast.warning : theme.colors.onSurface)
        }
        FactRow(
            fact: manual ? manualEntryFact(loaded) : loaded,
            title: loaded.heading.ifBlank("Value"),
            subtitle: "",
            fieldModifier: EdgeInsets(top: 8, leading: 0, bottom: 8, trailing: 0),
            onRejected: { rejected = true },
            onWrite: { revision += 1 }
        )
        .environment(\.LocalBlockRebootNote, factRebootNote(loaded))
        let range = parameterRangeLine(loaded)
        if !range.isBlank {
            Text(range).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
        }
        ForEach(parameterRebootNotes(loaded), id: \.self) { Text($0).font(.bodySmall) }
        if editable {
            Text(IN_FLIGHT_WARNING).font(.bodySmall).foregroundStyle(theme.aircast.warning)
        }
        if (loaded.readOnly && forceAllowed) || (editable && hasChoices) {
            CheckRow(text: "Advanced settings", checked: advanced) { on in
                advanced = on
                forced = forced && on
                manual = manual && on
            }
            if advanced && loaded.readOnly && forceAllowed {
                CheckRow(text: "Force edit read-only param", checked: forced) { forced = $0 }
            }
            if advanced && editable && hasChoices {
                CheckRow(text: "Manual entry", checked: manual) { manual = $0 }
            }
        }
        if forced || (editable && (manual || !hasChoices) && !loaded.isString && !loaded.isBool) {
            if !forced && rejected && forceAllowed {
                CheckRow(text: "Force save (dangerous!)", checked: forceSave) { forceSave = $0 }
            }
            if forceSave || forced {
                TextField("Value", text: $forcedText).textFieldStyle(.roundedBorder)
                if let forceRefusal {
                    Text(forceRefusal).font(.bodySmall).foregroundStyle(theme.colors.error)
                }
                Button("Save") { forceWrite(loaded) }
                    .buttonStyle(.text)
                    .disabled(forcedText.isBlank)
            }
        }
    }

    private func forceWrite(_ loaded: Fact) {
        let typed = forcedText.trimmed
        let entered: Any = loaded.isString ? forcedText : Double(typed).map { $0 as Any } ?? typed
        let path = loaded.path
        scope.launch {
            let answer = await offMain { Qgc.writeForcedRefusal(path, entered) }
            guard !Task.isCancelled else { return }
            forceRefusal = answer
            if answer == nil { revision += 1 }
        }
    }
}

let VALUE_DETAILS_TITLE = "Value Details"

private let APP_SETTING_PREFIX = "settings."

let SENT_TO_AIRCRAFT_NOTE = "Changes go to the aircraft straight away, even in flight."

let EDIT_PARAMETER_TITLE = "Edit parameter"

func valueDetailsNotes(_ fact: Fact) -> [String] {
    let details = fact.longDescription.ifBlank(fact.valueDetails)
    let range = parameterRangeLine(fact)
    return [details.isBlank ? nil : details, range.isBlank ? nil : range].compactMap { $0 } + parameterRebootNotes(fact)
}

struct ValueDetailsSheet: View {
    let fact: Fact
    var title: String = VALUE_DETAILS_TITLE
    let onWrite: () -> Void
    let onDismiss: () -> Void
    @State private var refusal: String?
    @State private var scope = ViewScope()
    @Environment(\.theme) private var theme

    var body: some View {
        let defaultValue = fact.readOnly ? nil : Double(fact.defaultValueString)
        AircastSheet(onDismissRequest: onDismiss) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s2) {
                    Text(title).font(.titleLarge)
                    if opensAsValue(fact) {
                        ValueControls(fact: fact, onWrite: onWrite) { typedEntry }
                    } else {
                        typedEntry
                    }
                    ForEach(valueDetailsNotes(fact), id: \.self) {
                        Text($0).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    if !fact.readOnly && !fact.path.hasPrefix(APP_SETTING_PREFIX) {
                        Text(SENT_TO_AIRCRAFT_NOTE).font(.bodySmall).foregroundStyle(theme.aircast.warning)
                    }
                    if let refusal {
                        Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error)
                    }
                    HStack(spacing: Space.s2) {
                        Spacer(minLength: 0)
                        if let value = defaultValue {
                            Button("Reset to default") { reset(value) }.buttonStyle(.text)
                        }
                        Button("Done", action: onDismiss).buttonStyle(.filled)
                    }
                }
                .padding(.horizontal, Space.s6)
                .padding(.bottom, Space.s6)
            }
            .environment(\.LocalSettingsList, false)
            .onChange(of: fact.path) { refusal = nil }
        }
        .onDisappear { scope.cancel() }
    }

    private var typedEntry: some View {
        FactRow(
            fact: fact,
            title: opensAsValue(fact) ? "Type a value" : fact.heading,
            subtitle: "",
            fieldModifier: EdgeInsets(top: 8, leading: 0, bottom: 8, trailing: 0),
            onWrite: onWrite
        )
    }

    private func reset(_ value: Double) {
        let path = fact.path
        scope.launch {
            let answer = await offMain { Qgc.writeRefusal(path, value) }
            guard !Task.isCancelled else { return }
            refusal = answer
            if answer == nil {
                onWrite()
                onDismiss()
            }
        }
    }
}
