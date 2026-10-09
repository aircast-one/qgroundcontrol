import SwiftUI

func itemFactsPath(_ index: Int) -> String { "view.itemFacts(\(index))" }

func itemCommandPath(_ index: Int) -> String { "plan.missionController.visualItems.\(index).command" }

func itemRawEditPath(_ index: Int) -> String { "plan.missionController.visualItems.\(index).rawEdit" }

let PRESETS_FIRST_SETTING = "settings.planViewSettings.displayPresetsTabFirst.rawValue"

func presetsShownFirst(_ setting: JSON?) -> Bool { setting?["value"].bool == true }

let RAW_EDIT_NOTE = "Provides advanced access to all commands/parameters. Be very careful!"

func itemNote(_ view: JSON?, rawOn: Bool) -> String? {
    rawOn ? RAW_EDIT_NOTE : view.flatMap { $0["commandDescription"].string.isBlank ? nil : $0["commandDescription"].string }
}

func wizardLines(_ view: JSON?) -> [String] {
    view?["wizardMode"].bool == true ? view?["wizardText"].strings ?? [] : []
}

func wizardModePath(_ index: Int) -> String { "plan.missionController.visualItems.\(index).wizardMode" }

func commandEditable(_ view: JSON?) -> Bool { view?["simple"].bool == true && view?["takeoff"].bool != true }

func startCategory(_ categories: [String], _ itemCategory: String?) -> String? {
    itemCategory.flatMap { categories.contains($0) ? $0 : nil } ?? categories.first
}

func mapCenterHintPath(_ index: Int) -> String { "plan.missionController.visualItems.\(index).setMapCenterHintForCommandChange" }

let RAW_EDIT_STUCK = "You have made changes to the mission item which cannot be shown in Simple Mode"

struct EntryPoint: Equatable {
    let label: String
    let value: String
    let path: String
}

func entryPoint(_ view: JSON?) -> EntryPoint? {
    guard let entry = view?["entryPoint"], entry.object != nil, !entry["path"].string.isBlank else { return nil }
    return EntryPoint(label: entry["label"].string, value: entry["value"].string, path: entry["path"].string)
}

func itemIsLandingPattern(_ view: JSON?) -> Bool { view?["landing"].bool == true }

func landingNotes(_ view: JSON?) -> [String] { (view?["landingNotes"].strings ?? []).filter { !$0.isBlank } }

func vehicleHeading(_ fact: JSON?) -> Double? { fact?["value"].double.flatMap { $0.isNaN ? nil : $0 } }

func vehicleCoordinate(_ coordinate: JSON?) -> JSON? {
    guard let coordinate, coordinate["valid"].bool(true), coordinate.has("latitude"), coordinate.has("longitude") else { return nil }
    return .object([
        "latitude": .number(coordinate["latitude"].double(.nan)),
        "longitude": .number(coordinate["longitude"].double(.nan)),
        "altitude": .number(0),
    ])
}

private func setToVehicleHeading(_ index: Int) -> String? {
    guard let heading = vehicleHeading(VehicleCommands.heading()) else { return "The vehicle has not reported its heading." }
    return PlanCommands.setLandingHeading(index, heading)
}

private func setToVehicleLocation(_ index: Int) -> String? {
    let read = VehicleCommands.coordinate()
    guard let coordinate = vehicleCoordinate(read["value"].objectOrNil ?? read) else { return "The vehicle has no position yet." }
    return PlanCommands.setLandingCoordinate(index, coordinate)
}

func areaHelp(_ view: JSON?) -> String? { view.flatMap { $0["areaHelp"].string.isBlank ? nil : $0["areaHelp"].string } }

func gridNote(_ view: JSON?) -> String? { view.flatMap { $0["gridNote"].string.isBlank ? nil : $0["gridNote"].string } }

struct RawEdit: Equatable {
    let on: Bool
    let friendlyAllowed: Bool
}

func rawEdit(_ view: JSON?) -> RawEdit? {
    guard let view, view["simple"].bool else { return nil }
    return RawEdit(on: view["rawEdit"].bool, friendlyAllowed: view["friendlyEditAllowed"].bool)
}

func rawEditRefusal(_ current: RawEdit) -> String? { current.on && !current.friendlyAllowed ? RAW_EDIT_STUCK : nil }

struct CommandChoice: Equatable, Identifiable {
    let id: Int
    let name: String
    let description: String
}

func commandChoices(_ result: JSON?) -> [CommandChoice] {
    (result?.objects ?? []).map {
        CommandChoice(id: $0["command"].int(0), name: $0["friendlyName"].string, description: $0["description"].string)
    }
}

func categoryNames(_ result: JSON?) -> [String] { (result?.arrayOrNil ?? []).map(\.string) }

private struct OptionalFactRow: View {
    let fact: Fact
    let onWrite: () -> Void

    var body: some View {
        HStack {
            Toggle("", isOn: Binding(get: { fact.optionalSet }, set: { on in
                let path = fact.path
                Task {
                    _ = await offMain { Qgc.set(path, on ? 0.0 : nil) }
                    onWrite()
                }
            }))
            .labelsHidden()
            FactRow(fact: fact.optionalSet ? fact : withChanges(fact) { $0.enabled = false }, onWrite: onWrite)
                .frame(maxWidth: .infinity)
        }
        .padding(.leading, Space.s5)
    }
}

func altitudesRelative(_ view: JSON?) -> Bool? {
    guard let view, view["landing"].bool, view.has("altitudesAreRelative") else { return nil }
    return view["altitudesAreRelative"].bool
}

private let LANDING_ALTITUDES = ["finalApproachAltitude", "landingAltitude"]

func withLandingFrameUnits(_ fields: [Fact], _ relative: Bool?) -> [Fact] {
    guard let relative else { return fields }
    let frame = relative ? "Rel" : "AMSL"
    return fields.map { fact in
        guard LANDING_ALTITUDES.contains(where: { fact.path.hasSuffix(".\($0)") }) else { return fact }
        return withChanges(fact) { $0.units = [fact.units, frame].filter { !$0.isBlank }.joined(separator: " ") }
    }
}

func altitudeHint(_ view: JSON?) -> String? {
    view.flatMap { $0.has("altitudeHint") && !$0["altitudeHint"].string.isBlank ? $0["altitudeHint"].string : nil }
}

func previousCoordinate(_ view: JSON?) -> (Double, Double)? {
    guard let previous = view?["previousCoordinate"], previous.object != nil else { return nil }
    let latitude = previous["latitude"].double(.nan)
    let longitude = previous["longitude"].double(.nan)
    return latitude.isNaN || longitude.isNaN ? nil : (latitude, longitude)
}

func sectionStarts(_ view: JSON?) -> [String: String] {
    let rows = (view?["fields"].objects ?? []).map { ($0["path"].string, $0["section"].string) }
    let starts = rows.enumerated().filter { at, row in !row.1.isBlank && (at == 0 || row.1 != rows[at - 1].1) }.map(\.element)
    return Dictionary(starts, uniquingKeysWith: { _, last in last })
}

struct RadioChoice: Equatable {
    let path: String
    let value: Bool
    let selected: Bool
}

func radioChoices(_ view: JSON?) -> [String: RadioChoice] {
    let rows = (view?["fields"].objects ?? []).compactMap { row -> (String, RadioChoice)? in
        let choice = row["choice"]
        guard choice.object != nil else { return nil }
        return (row["path"].string, RadioChoice(path: choice["path"].string, value: choice["value"].bool, selected: choice["selected"].bool))
    }
    return Dictionary(rows, uniquingKeysWith: { _, last in last })
}

private let HOLD_LABEL = "Hold"

func withoutHeroFields(_ fields: [Fact], _ view: JSON?) -> [Fact] {
    view?["hold"].object == nil ? fields : fields.filter { $0.description != HOLD_LABEL }
}

func itemFields(_ view: JSON?) -> [Fact] {
    (view?["fields"].objects ?? []).compactMap { factFromControl($0) }
}

private let DELETE_LABEL = "Delete from the plan"

struct ItemEditor: View {
    let index: Int
    let at: TrackPoint?
    let mapCentre: (Double, Double)?
    let legDetail: String?
    let onRemove: (() -> Void)?

    var body: some View {
        ItemEditorContent(index: index, at: at, mapCentre: mapCentre, legDetail: legDetail, onRemove: onRemove)
            .id(index)
    }
}

private struct ItemEditorContent: View {
    let index: Int
    let at: TrackPoint?
    let mapCentre: (Double, Double)?
    let legDetail: String?
    let onRemove: (() -> Void)?
    @Environment(\.theme) private var theme
    @HasVehicle private var connected
    @AdvancedUiShown private var advancedUi
    @State private var advanced = false
    @State private var revision = 0
    @State private var view: JSON?
    @State private var choosing = false
    @State private var editingPosition = false
    @State private var refusal: String?
    @State private var detailsPath: String?
    @State private var stats: SurveyStats?
    @State private var presetsFirst: Bool?

    var body: some View {
        let fields = withoutHeroFields(withLandingFrameUnits(itemFields(view), altitudesRelative(view)), view)
        let sections = sectionStarts(view)
        let choices = radioChoices(view)
        let raw = rawEdit(view)
        let camera = cameraCalc(view)
        let wizard = wizardLines(view)
        let positionStart = at ?? (mapCentre ?? previousCoordinate(view)).map { TrackPoint(latitude: $0.0, longitude: $0.1) }
        let simple = view?["simple"].bool == true
        VStack(alignment: .leading, spacing: 0) {
            if simple {
                Button { advanced.toggle() } label: {
                    HStack {
                        Text("Advanced").font(.titleSmall).frame(maxWidth: .infinity, alignment: .leading)
                        Image(advanced ? .arrowUp : .arrowDropDown)
                            .accessibilityLabel(advanced ? "Hide advanced settings" : "Show advanced settings")
                    }
                    .foregroundStyle(theme.colors.onSurface)
                    .padding(.horizontal, 12)
                    .frame(minHeight: 48)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
            if !simple || advanced {
                folded(fields, sections, choices, raw, camera, wizard, positionStart, simple: simple)
            }
            if let onRemove {
                Button(DELETE_LABEL, action: onRemove)
                    .buttonStyle(.borderless)
                    .font(.labelLarge)
                    .foregroundStyle(theme.colors.error)
                    .frame(minHeight: 40)
                    .padding(.horizontal, 20)
            }
        }
        .padding(.bottom, Space.s2)
        .background {
            if let fact = fields.first(where: { $0.path == detailsPath }) {
                ValueDetailsSheet(fact: fact, onWrite: { revision += 1 }, onDismiss: { detailsPath = nil })
            }
            if editingPosition, let positionStart {
                EditPositionDialog(
                    at: positionStart,
                    onDismiss: { editingPosition = false },
                    altitudeMode: view.flatMap { $0["altitudeMode"].isNull ? nil : $0["altitudeMode"].int(-1) },
                    onAltitude: { shown in
                        Task {
                            let set = await offMain { PlanBridge.setAltitude(index, shown) }
                            refusal = set ? nil : "The altitude could not be set."
                            revision += 1
                        }
                    }
                ) { latitude, longitude in
                    editingPosition = false
                    Task {
                        let moved = await offMain { PlanBridge.moveItem(index, latitude, longitude) }
                        refusal = moved ? nil : "The item could not be moved there."
                        revision += 1
                    }
                }
            }
            if choosing {
                CommandPicker(itemCategory: view.flatMap { $0["category"].string.isBlank ? nil : $0["category"].string }, onDismiss: { choosing = false }) { command in
                    choosing = false
                    let centre = mapCentre
                    run {
                        if let (latitude, longitude) = centre {
                            _ = Qgc.refusalOf(mapCenterHintPath(index), ["latitude": latitude, "longitude": longitude])
                        }
                        return Qgc.writeRefusal(itemCommandPath(index), command)
                    }
                }
            }
        }
        .task(id: revision) {
            let index = index
            view = await offMain { Qgc.get(itemFactsPath(index)) }
        }
        .task(id: "\(revision)|\(camera != nil)") {
            let index = index
            let wanted = camera != nil
            stats = await offMain { wanted ? surveyStats(Qgc.get("view.surveyStats(\(index))")) : nil }
        }
        .task {
            presetsFirst = await offMain { presetsShownFirst(Qgc.get(PRESETS_FIRST_SETTING)) }
        }
    }

    @ViewBuilder
    private func folded(_ fields: [Fact], _ sections: [String: String], _ choices: [String: RadioChoice], _ raw: RawEdit?, _ camera: CameraCalcBlock?, _ wizard: [String], _ positionStart: TrackPoint?, simple: Bool) -> some View {
        if let legDetail {
            note(legDetail).padding(.vertical, Space.s1)
        }
        header(positionStart)
        if !wizard.isEmpty { wizardBlock(wizard, fields) }
        if index == 0 { missionStart }
        if wizard.isEmpty, let relative = altitudesRelative(view) {
            Toggle(isOn: Binding(get: { relative }, set: { wanted in run { PlanCommands.setAltitudesRelative(index, wanted) } })) {
                Text("Altitudes relative to launch").font(.bodyMedium)
            }
            .padding(.horizontal, Space.s5)
        }
        if let hint = altitudeHint(view) {
            note(hint).padding(.vertical, Space.s1)
        }
        if advancedUi, let current = raw {
            Toggle(isOn: Binding(get: { current.on }, set: { wanted in
                if let stuck = rawEditRefusal(current) {
                    refusal = stuck
                    revision += 1
                } else {
                    run { Qgc.writeRefusal(itemRawEditPath(index), wanted) }
                }
            })) {
                Text("Show all values").font(.bodyMedium)
            }
            .padding(.horizontal, Space.s5)
        }
        if let line = itemNote(view, rawOn: raw?.on == true) { note(line) }
        if let refusal {
            Text(refusal).foregroundStyle(theme.colors.error).padding(.horizontal, Space.s5)
        }
        if itemIsLandingPattern(view) && connected && wizard.isEmpty {
            HStack(spacing: Space.s2) {
                Button("Set to vehicle heading") { run { setToVehicleHeading(index) } }
                Button("Set to vehicle location") { run { setToVehicleLocation(index) } }
            }
            .padding(.horizontal, Space.s5)
        }
        if wizard.isEmpty {
            ForEach(landingNotes(view), id: \.self) { line in
                Text(line).font(.bodySmall).foregroundStyle(theme.aircast.warning).padding(.horizontal, Space.s5)
            }
        }
        if let help = areaHelp(view) {
            Text(help).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                .padding(.horizontal, Space.s5).padding(.vertical, Space.s3)
        }
        if areaHelp(view) == nil && wizard.isEmpty, let presetsFirst {
            VStack(alignment: .leading, spacing: 0) {
                if presetsFirst { presets }
                ForEach(fields) { fact in fieldRow(fact, sections[fact.path], choices[fact.path]) }
                if !simple {
                    ItemCameraSection(index: index) { revision += 1 }
                        .padding(.horizontal, Space.s5)
                }
                if let camera {
                    CameraCalcHeader(block: camera) { path, value in run { Qgc.writeRefusal(path, value) } }
                    ForEach(shownCameraFacts(camera)) { fact in FactRow(fact: fact, onWrite: { revision += 1 }) }
                }
                if let line = gridNote(view) { note(line).padding(.vertical, Space.s1) }
                if let entry = entryPoint(view) { entryRow(entry) }
                if let stats { statistics(stats) }
                if !presetsFirst { presets }
            }
        }
    }

    private func run(_ write: @escaping () -> String?) {
        Task {
            refusal = await offMain { write() }
            revision += 1
        }
    }

    private func note(_ text: String) -> some View {
        Text(text).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.horizontal, Space.s5)
    }

    @ViewBuilder
    private func header(_ positionStart: TrackPoint?) -> some View {
        HStack {
            Spacer()
            if at != nil || view?["specifiesCoordinate"].bool == true {
                let previous = previousCoordinate(view)
                Menu("Position") {
                    Button("Move to Vehicle Position") { moveTo(nil) }.disabled(!connected)
                    Button("Move to Previous Item") { moveTo(previous) }.disabled(previous == nil)
                    Button("Edit position…") { editingPosition = true }.disabled(positionStart == nil)
                }
            }
            if commandEditable(view) {
                Button("Change command") { choosing = true }
            }
        }
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
    }

    private func moveTo(_ target: (Double, Double)?) {
        Task {
            let index = index
            let moved = await offMain {
                (target ?? geoOf(VehicleCommands.coordinate())).map { PlanBridge.moveItem(index, $0.0, $0.1) } ?? false
            }
            refusal = moved ? nil : "The item could not be moved there."
            revision += 1
        }
    }

    private func wizardBlock(_ wizard: [String], _ fields: [Fact]) -> some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text(wizard[0]).font(.bodyMedium)
            ForEach(Array(wizard.dropFirst()), id: \.self) { line in
                Text(line).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if let clockwise = fields.first(where: { $0.path.hasSuffix(".loiterClockwise") }) {
                FactRow(fact: clockwise, onWrite: { revision += 1 })
            }
            Button {
                run { Qgc.writeRefusal(wizardModePath(index), false) }
            } label: {
                Text("Done").frame(maxWidth: .infinity)
            }
            .buttonStyle(.filled)
        }
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
    }

    @ViewBuilder
    private var missionStart: some View {
        MissionAltitudeFrame()
        PlanVehicleRows()
        if let launch = view.flatMap({ $0["launchAltitude"].object != nil ? factFromControl($0["launchAltitude"]) : nil }) {
            Text("Launch position").font(.titleSmall).padding(.horizontal, Space.s5).padding(.vertical, Space.s1)
            FactRow(fact: launch, subtitle: "Actual position is set by the vehicle at flight time.", onWrite: { revision += 1 })
            HStack {
                Text("Position").font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                Button("Set to map center") {
                    guard let (latitude, longitude) = mapCentre else { return }
                    Task {
                        let moved = await offMain { PlanBridge.moveItem(0, latitude, longitude) }
                        refusal = moved ? nil : "The launch position could not be moved there."
                        revision += 1
                    }
                }
                .disabled(mapCentre == nil)
            }
            .padding(.horizontal, Space.s5)
        }
    }

    @ViewBuilder
    private var presets: some View {
        if let kind = presetKind(view) {
            PatternPresets(index: index, kind: kind) { revision += 1 }
        }
    }

    @ViewBuilder
    private func fieldRow(_ fact: Fact, _ section: String?, _ choice: RadioChoice?) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            if let section { SectionHeader(text: section) }
            if let choice {
                HStack {
                    Button {
                        run { Qgc.writeRefusal(choice.path, choice.value) }
                    } label: {
                        RadioIndicator(selected: choice.selected)
                    }
                    .buttonStyle(.plain)
                    FactRow(fact: fact, fieldModifier: EdgeInsets(top: 8, leading: 0, bottom: 8, trailing: 16), onWrite: { revision += 1 })
                        .frame(maxWidth: .infinity)
                }
                .padding(.leading, Space.s2)
            } else if fact.optional {
                OptionalFactRow(fact: fact) { revision += 1 }
            } else if !fact.valueDetails.isBlank {
                HStack {
                    FactRow(fact: fact, onWrite: { revision += 1 }).frame(maxWidth: .infinity)
                    Button("?") { detailsPath = fact.path }.padding(.trailing, Space.s2)
                }
            } else {
                FactRow(fact: fact, onWrite: { revision += 1 })
            }
        }
    }

    private func entryRow(_ entry: EntryPoint) -> some View {
        Button {
            run { Qgc.refusalOf(entry.path) }
        } label: {
            HStack {
                Text(entry.label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                Text(entry.value).font(.bodyMedium)
                Image(.chevronRight)
            }
            .frame(minHeight: 48)
            .padding(.horizontal, Space.s5)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func statistics(_ known: SurveyStats) -> some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            if !known.warning.isBlank {
                Text(known.warning).font(.bodyMedium).foregroundStyle(theme.aircast.warning)
            }
            Text("Statistics").font(.titleSmall)
            ForEach(statisticsRows(known), id: \.0) { label, value in
                HStack {
                    Text(label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                    Text(value.ifBlank("\u{2014}")).font(.bodyMedium)
                }
            }
        }
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
    }
}

private struct CommandPicker: View {
    let itemCategory: String?
    let onDismiss: () -> Void
    let onChosen: (Int) -> Void
    @Environment(\.theme) private var theme
    @State private var categories: [String] = []
    @State private var category: String?
    @State private var commands: [CommandChoice] = []

    var body: some View {
        let listedCategories = categories
        let listedCommands = commands
        PlanDialog(title: "Select mission command", onDismiss: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s2) {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: Space.s1) {
                        ForEach(listedCategories, id: \.self) { name in
                            CameraChip(label: sentenceCase(name), selected: name == category) { category = name }
                        }
                    }
                }
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(listedCommands) { command in
                        Button { onChosen(command.id) } label: {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(sentenceCase(command.name)).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                                Text(command.description).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.vertical, Space.s3)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        } buttons: {
            Button("Cancel", action: onDismiss)
        }
        .task {
            categories = await offMain { categoryNames(PlanCommands.commandCategories()) }
            category = startCategory(categories, itemCategory)
        }
        .task(id: category) {
            guard let chosen = category else { return }
            commands = await offMain { commandChoices(PlanCommands.commandsForCategory(chosen)) }
        }
    }
}

struct SpeedSection: Equatable {
    let specified: Bool
    let value: Double?
    let units: String
    let path: String
    let specifyPath: String
    var slider: FactSlider? = nil
}

func speedSection(_ view: JSON?) -> SpeedSection? { speedSectionOf(view?["speedSection"]) }

func speedSectionOf(_ served: JSON?) -> SpeedSection? {
    guard let served, served.object != nil, served["available"].bool else { return nil }
    return SpeedSection(
        specified: served["specified"].bool,
        value: served["value"].isNull ? nil : served["value"].double(.nan),
        units: served["units"].string,
        path: served["path"].string,
        specifyPath: served["specifyPath"].string,
        slider: served["slider"].object != nil ? factSlider(served["slider"], "") : nil
    )
}

struct SpeedSectionRow: View {
    let speed: SpeedSection
    var withSlider: Bool = false
    let onWrite: (@escaping () -> String?) -> Void
    @State private var typed = ""

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Toggle(isOn: Binding(get: { speed.specified }, set: { on in
                let path = speed.specifyPath
                onWrite { Qgc.writeRefusal(path, on) }
            })) {
                Text("Flight speed")
            }
            if speed.specified {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(["Speed", speed.units].filter { !$0.isBlank }.joined(separator: " ")).font(.bodySmall)
                        TextField("Speed", text: $typed)
                            .keyboardType(.decimalPad)
                            .textFieldStyle(.roundedBorder)
                    }
                    Button("Set") {
                        if let value = typedNumber(typed) {
                            let path = speed.path
                            onWrite { Qgc.writeRefusal(path, value) }
                        }
                    }
                    .disabled(typedNumber(typed) == nil)
                }
                if withSlider, let slider = speed.slider {
                    FieldSlider(value: speed.value, slider: slider, enabled: true) { value in
                        let path = speed.path
                        onWrite { Qgc.writeRefusal(path, value) }
                    }
                }
            }
        }
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
        .onChange(of: speed.value, initial: true) { _, value in
            typed = value.map { "\($0)" } ?? ""
        }
    }
}

enum CoordinateSystem: CaseIterable, Equatable {
    case Geographic, Utm, Mgrs, Vehicle

    var label: String {
        switch self {
        case .Geographic: "Geographic"
        case .Utm: "Universal Transverse Mercator"
        case .Mgrs: "Military Grid Reference"
        case .Vehicle: "Vehicle Position"
        }
    }
}

struct PositionForms: Equatable {
    let zone: String
    let southern: Bool
    let easting: String
    let northing: String
    let mgrs: String
}

func positionForms(_ view: JSON?) -> PositionForms? {
    guard let view else { return nil }
    let utm = view["utm"].objectOrNil
    return PositionForms(
        zone: utm.map { String($0["zone"].int(0)) } ?? "",
        southern: utm?["southern"].bool ?? false,
        easting: utm.map { String(format: "%.2f", $0["easting"].double(.nan)) } ?? "",
        northing: utm.map { String(format: "%.2f", $0["northing"].double(.nan)) } ?? "",
        mgrs: view["mgrs"].string
    )
}

func positionFormsPath(_ at: TrackPoint) -> String {
    String(format: "view.positionForms(%.8f,%.8f)", at.latitude, at.longitude)
}

func utmToGeoPath(_ easting: String, _ northing: String, _ zone: String, _ southern: Bool) -> String {
    "view.utmToGeo(\(easting),\(northing),\(zone),\(southern))"
}

func geoOf(_ view: JSON?) -> (Double, Double)? {
    guard let view, view["valid"].bool else { return nil }
    return (view["latitude"].double(.nan), view["longitude"].double(.nan))
}

private struct PositionTarget: Sendable {
    let system: CoordinateSystem
    let latitude: String
    let longitude: String
    let easting: String
    let northing: String
    let zone: String
    let southern: Bool
    let mgrs: String
}

private func positionTarget(_ wanted: PositionTarget) -> (Double, Double)? {
    switch wanted.system {
    case .Geographic: typedNumber(wanted.latitude).flatMap { lat in typedNumber(wanted.longitude).map { (lat, $0) } }
    case .Utm: geoOf(Qgc.get(utmToGeoPath(wanted.easting, wanted.northing, wanted.zone, wanted.southern)))
    case .Mgrs: geoOf(Qgc.get("view.mgrsToGeo(\(wanted.mgrs.replacingOccurrences(of: " ", with: "")))"))
    case .Vehicle: geoOf(VehicleCommands.coordinate())
    }
}

struct EditPositionDialog: View {
    let at: TrackPoint
    let onDismiss: () -> Void
    var title: String = "Edit position"
    var confirm: String = "Move"
    var vehicleConfirm: String? = nil
    var altitudeMode: Int? = nil
    var onAltitude: ((Double) -> Void)? = nil
    let onMove: (Double, Double) -> Void
    @Environment(\.theme) private var theme
    @HasVehicle private var connected
    @State private var setPosition = true
    @State private var setAltitude = false
    @State private var system = CoordinateSystem.Geographic
    @State private var latitude: String?
    @State private var longitude: String?
    @State private var zone = ""
    @State private var southern = false
    @State private var easting = ""
    @State private var northing = ""
    @State private var mgrs = ""
    @State private var problem: String?
    @State private var vehicleShown: VehiclePosition?

    private var altitudePath: String? { onAltitude == nil ? nil : vehicleAltitudePath(altitudeMode) }

    var body: some View {
        PlanDialog(title: title, onDismiss: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s2) {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: Space.s1) {
                        ForEach(CoordinateSystem.allCases.filter { $0 != .Vehicle || connected }, id: \.self) { choice in
                            CameraChip(label: choice.label, selected: choice == system) {
                                system = choice
                                problem = nil
                            }
                        }
                    }
                }
                fields
                if let problem { Text(problem).foregroundStyle(theme.colors.error) }
            }
        } buttons: {
            Button("Cancel", action: onDismiss)
            Button(system == .Vehicle ? vehicleConfirm ?? confirm : confirm) { move() }
        }
        .task(id: "\(system)|\(altitudePath ?? "")") {
            let path = altitudePath
            while system == .Vehicle && vehicleConfirm == nil && !Task.isCancelled {
                vehicleShown = await offMain { vehiclePositionOf(VehicleCommands.coordinate(), path.map { Qgc.get($0) }) }
                try? await Task.sleep(for: .milliseconds(VEHICLE_POSITION_POLL_MS))
            }
        }
        .task(id: at) {
            let at = at
            guard let forms = await offMain({ positionForms(Qgc.get(positionFormsPath(at))) }) else { return }
            zone = forms.zone
            southern = forms.southern
            easting = forms.easting
            northing = forms.northing
            mgrs = forms.mgrs
        }
    }

    @ViewBuilder
    private var fields: some View {
        switch system {
        case .Geographic:
            PositionField(label: "Latitude", value: latitude ?? String(format: "%.7f", at.latitude)) { latitude = $0 }
            PositionField(label: "Longitude", value: longitude ?? String(format: "%.7f", at.longitude)) { longitude = $0 }
        case .Utm:
            PositionField(label: "Zone", value: zone) { zone = $0 }
            Toggle("Southern hemisphere", isOn: $southern)
            PositionField(label: "Easting", value: easting) { easting = $0 }
            PositionField(label: "Northing", value: northing) { northing = $0 }
        case .Mgrs:
            PositionField(label: "MGRS", value: mgrs, keyboard: .default) { mgrs = $0 }
        case .Vehicle:
            if vehicleConfirm == nil {
                PositionRow(label: "Latitude", value: vehicleShown?.latitude ?? "")
                PositionRow(label: "Longitude", value: vehicleShown?.longitude ?? "")
                if altitudePath != nil, let label = vehicleAltitudeLabel(altitudeMode) {
                    PositionRow(label: label, value: vehicleShown?.altitude ?? "")
                }
                Toggle("Set position from vehicle", isOn: $setPosition)
                if altitudePath != nil {
                    Toggle("Set altitude from vehicle", isOn: $setAltitude)
                }
            }
        }
    }

    private func fromVehicle() {
        let wantPosition = setPosition
        let path = setAltitude ? altitudePath : nil
        Task {
            let position = await offMain { wantPosition ? geoOf(VehicleCommands.coordinate()) : nil }
            let altitude = await offMain { path.map { Qgc.get($0)["value"].double(.nan) }.flatMap { $0.isFinite ? $0 : nil } }
            if let altitude { onAltitude?(altitude) }
            if wantPosition && position == nil {
                problem = "That position could not be read."
            } else if let position {
                onMove(position.0, position.1)
            } else {
                onDismiss()
            }
        }
    }

    private func move() {
        guard system != .Vehicle else { return fromVehicle() }
        let wanted = PositionTarget(
            system: system,
            latitude: latitude ?? String(format: "%.7f", at.latitude),
            longitude: longitude ?? String(format: "%.7f", at.longitude),
            easting: easting,
            northing: northing,
            zone: zone,
            southern: southern,
            mgrs: mgrs
        )
        Task {
            if let target = await offMain({ positionTarget(wanted) }) {
                onMove(target.0, target.1)
            } else {
                problem = "That position could not be read."
            }
        }
    }
}

private let VEHICLE_POSITION_POLL_MS = 500

struct VehiclePosition: Equatable {
    let latitude: String
    let longitude: String
    let altitude: String
}

func vehiclePositionOf(_ coordinate: JSON?, _ altitude: JSON?) -> VehiclePosition? {
    geoOf(coordinate).map { latitude, longitude in
        VehiclePosition(
            latitude: String(format: "%.7f", latitude),
            longitude: String(format: "%.7f", longitude),
            altitude: altitude.flatMap { read in
                read["valueString"].string.isEmpty ? nil : "\(read["valueString"].string) \(read["units"].string)".trimmed
            } ?? ""
        )
    }
}

func vehicleAltitudeLabel(_ altitudeMode: Int?) -> String? {
    switch altitudeMode {
    case 1: "Alt (Rel)"
    case 2: "Alt (AMSL)"
    case 3, 4: "Alt (AGL)"
    default: nil
    }
}

private struct PositionRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Text(value)
        }
        .padding(.vertical, Space.s1)
    }
}

func vehicleAltitudePath(_ altitudeMode: Int?) -> String? {
    switch altitudeMode {
    case 1: "vehicle.altitudeRelative"
    case 2: "vehicle.altitudeAMSL"
    case 3, 4: "vehicle.altitudeAboveTerr"
    default: nil
    }
}

private struct PositionField: View {
    let label: String
    let value: String
    var keyboard: UIKeyboardType = .numbersAndPunctuation
    let onChange: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            TextField(label, text: Binding(get: { value }, set: onChange))
                .keyboardType(keyboard)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.characters)
                .textFieldStyle(.roundedBorder)
        }
    }
}
