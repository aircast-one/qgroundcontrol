import SwiftUI

private let MISSION_POPUP_DELAY_MS = 1000
private let GCS_POSITION = "view.gcsPosition"

struct ConfirmOption {
    let label: String
    let run: (Bool) -> Void
}

struct GuidedAction {
    let name: String
    let confirm: String
    let destructive: Bool
    var option: ConfirmOption? = nil
    var offerId: String? = nil
    let run: () -> Void
}

let CHECKLIST_PASSED = 1
let CHECKLIST_FAILED = 2

func checklistStateValue(_ passed: Bool) -> Int { passed ? CHECKLIST_PASSED : CHECKLIST_FAILED }

func offerWithdrawn(_ offerId: String?, _ offers: [String: GuidedOffer]) -> Bool {
    offerId.map { offers[$0]?.shown != true } ?? false
}

struct Instrument: Equatable {
    let label: String
    let reading: String
    let id: String
    let value: String
    let units: String
    let raw: Double?
    let defaultIcon: String

    init(label: String, reading: String, id: String = "", value: String? = nil, units: String = "", raw: Double? = nil, defaultIcon: String = "") {
        self.label = label
        self.reading = reading
        self.id = id
        self.value = value ?? reading
        self.units = units
        self.raw = raw
        self.defaultIcon = defaultIcon
    }
}

func displayFor(_ displays: [String: ValueDisplay], _ instrument: Instrument) -> ValueDisplay {
    displays[instrument.id] ?? ValueDisplay(icon: instrument.defaultIcon)
}

func rowWidth(_ count: Int) -> Int { count <= 4 ? count : (count + 1) / 2 }

func operatorDistance(_ view: JSON?) -> [Instrument] {
    guard let text = view?["distanceToVehicleText"].string, !text.isBlank else { return [] }
    let split = text.range(of: " ", options: .backwards)
    return [Instrument(
        label: "From you",
        reading: text,
        value: split.map { String(text[..<$0.lowerBound]) } ?? text,
        units: split.map { String(text[$0.upperBound...]) } ?? ""
    )]
}

let AWAITING_READING = "\u{2014}"

func instrumentReading(_ item: JSON) -> String? {
    if !item["missing"].bool {
        let units = item["units"].string
        let value = item["value"].string
        return units.isBlank ? value : "\(value) \(units)"
    }
    return item["missingReason"].string == "notReported" ? AWAITING_READING : nil
}

func instruments(_ view: JSON?) -> [Instrument] {
    (view?["items"].arrayOrNil ?? []).filter { $0.object != nil }.compactMap { item in
        instrumentReading(item).map { reading in
            let missing = item["missing"].bool
            return Instrument(
                label: item["label"].string,
                reading: reading,
                id: item["id"].string,
                value: missing ? reading : item["value"].string,
                units: missing ? "" : item["units"].string,
                raw: item["raw"].isNull ? nil : item["raw"].double.flatMap { $0.isNaN ? nil : $0 },
                defaultIcon: item["defaultIcon"].string
            )
        }
    }
}

struct VehicleTitle: View {
    @Environment(\.theme) private var theme
    @QgcPath(FLY_STATE) private var json

    var body: some View {
        let fly = flyState(json)
        let communicationLost = fly?.contactLost == true
        VStack(alignment: .leading) {
            Text("Aircast").font(.titleMedium)
            Text(vehicleSubtitle(fly))
                .font(.bodySmall)
                .fontWeight(communicationLost ? .bold : .regular)
                .foregroundStyle(communicationLost ? theme.colors.error : theme.colors.onSurfaceVariant)
        }
    }
}

private struct Replacing: Equatable {
    let index: Int
    let fromId: String
}

struct TelemetryRow: View {
    var columns: Int? = nil
    var valuesShown = true
    var chooser = true
    var compact = false
    var stacked = false
    @Environment(FlyScreenState.self) private var flyScreen
    @QgcPath(INSTRUMENTS_VIEW) private var classView
    @State private var chosen: [String]?
    @State private var displays: [String: ValueDisplay] = [:]
    @State private var styling: Instrument?
    @State private var replacing: Replacing?

    var body: some View {
        let vehicleClass = instrumentVehicleClass(classView)
        let picked = chosen ?? []
        ZStack {
            TelemetryReadings(
                path: chosen.map { instrumentsPath($0, vehicleClass: vehicleClass) },
                chosen: picked,
                displays: displays,
                columns: columns,
                valuesShown: valuesShown,
                chooser: chooser,
                compact: compact,
                stacked: stacked,
                onStyle: { styling = $0 }
            )
            Color.clear
                .frame(width: 0, height: 0)
                .accessibilityHidden(true)
                .background { sheets(vehicleClass, picked) }
                .onChange(of: vehicleClass, initial: true) { _, now in
                    chosen = readChosen(now)
                    displays = readDisplays(now)
                    flyScreen.layout.valueSize = readValueSize(now)
                }
        }
    }

    private func change(_ vehicleClass: String, _ next: [String]) {
        chosen = next
        writeChosen(vehicleClass, next)
        flyScreen.instrumentEdits += 1
        styling = nil
    }

    @ViewBuilder
    private func sheets(_ vehicleClass: String, _ picked: [String]) -> some View {
        if let replaced = replacing {
            InstrumentSheet(
                chosen: picked,
                title: "Change reading",
                onToggle: { path in
                    if let carried = displays[replaced.fromId], displays[selectionId(path)] == nil {
                        writeDisplay(vehicleClass, selectionId(path), carried)
                        displays[selectionId(path)] = carried
                    }
                    change(vehicleClass, replacedInstrument(picked, replaced.index, path))
                    replacing = nil
                },
                onDismiss: { replacing = nil }
            )
        }
        if let instrument = styling {
            let index = picked.firstIndex { selectionId($0) == instrument.id }
            ValueDisplayDialog(
                label: instrument.label,
                initial: displayFor(displays, instrument),
                onDismiss: { styling = nil },
                extra: {
                    if let index {
                        PlanFlowRow(spacing: Space.s1) {
                            Button("Change reading") {
                                replacing = Replacing(index: index, fromId: instrument.id)
                                styling = nil
                            }
                            if index > 0 {
                                Button("Move left") { change(vehicleClass, movedInstrument(picked, index, -1)) }
                            }
                            if index < picked.count - 1 {
                                Button("Move right") { change(vehicleClass, movedInstrument(picked, index, 1)) }
                            }
                            Button("Remove") { change(vehicleClass, removedInstrument(picked, index)) }
                        }
                        .buttonStyle(.borderless)
                    }
                },
                onDone: { display in
                    writeDisplay(vehicleClass, instrument.id, display)
                    displays[instrument.id] = display
                    flyScreen.instrumentEdits += 1
                    styling = nil
                }
            )
        }
        if flyScreen.choosingReadings {
            InstrumentSheet(
                chosen: picked,
                onToggle: { name in
                    let next = withInstrument(chosen ?? [], name)
                    chosen = next
                    writeChosen(vehicleClass, next)
                    flyScreen.instrumentEdits += 1
                },
                onDismiss: { flyScreen.choosingReadings = false }
            )
        }
    }
}

private enum TelemetryTile {
    case reading(Instrument)
    case chooser
}

private struct TelemetryReadings: View {
    @Environment(\.theme) private var theme
    @Environment(FlyScreenState.self) private var flyScreen
    let chosen: [String]
    let displays: [String: ValueDisplay]
    let columns: Int?
    let valuesShown: Bool
    let chooser: Bool
    let compact: Bool
    let stacked: Bool
    let onStyle: (Instrument) -> Void
    @QgcPath private var view: JSON?
    @QgcPath(GCS_POSITION) private var gcsJson
    @QgcPath(FLY_STATE) private var stateJson
    @ScaledMetric(relativeTo: .body) private var telemetrySize = TypeScale.telemetry.size

    init(path: String?, chosen: [String], displays: [String: ValueDisplay], columns: Int?, valuesShown: Bool, chooser: Bool, compact: Bool, stacked: Bool, onStyle: @escaping (Instrument) -> Void) {
        self.chosen = chosen
        self.displays = displays
        self.columns = columns
        self.valuesShown = valuesShown
        self.chooser = chooser
        self.compact = compact
        self.stacked = stacked
        self.onStyle = onStyle
        _view = QgcPath(path, holdingLast: true)
    }

    var body: some View {
        let shown = (showsInstruments(chosen) ? instruments(view) : []) + operatorDistance(gcsJson)
        let stale = flyState(stateJson)?.staleNotice ?? ""
        let silent = !stale.isBlank
        if !shown.isEmpty {
            if compact {
                compactReadings(shown, silent)
            } else if valuesShown {
                fullReadings(shown, stale, silent)
                    .transition(.opacity.combined(with: .move(edge: .top)))
            }
        }
    }

    private func osdReading(_ instrument: Instrument, _ font: TypeScale) -> some View {
        HStack(alignment: .lastTextBaseline, spacing: Space.s1) {
            Text(osdLabel(instrument.label))
                .font(.labelMedium)
                .foregroundStyle(theme.aircast.outdoorForeground.opacity(OSD_LABEL_ALPHA))
                .lineLimit(1)
                .fixedSize()
            Text("\(instrument.value)\(instrument.units)")
                .font(font)
                .monospacedDigit()
                .lineLimit(1)
                .fixedSize()
                .foregroundStyle(displayColour(displayFor(displays, instrument), instrument.raw).map(argbColor) ?? theme.aircast.outdoorForeground)
        }
    }

    @ViewBuilder
    private func compactReadings(_ shown: [Instrument], _ silent: Bool) -> some View {
        if stacked {
            let speeds = shown.filter { osdIsSpeed($0.label) }
            let places = shown.filter { !osdIsSpeed($0.label) }
            VStack(alignment: .leading, spacing: 2) {
                if !speeds.isEmpty {
                    HStack(spacing: Space.s4) { ForEach(Array(speeds.enumerated()), id: \.offset) { osdReading($0.element, .labelLarge) } }
                }
                if !places.isEmpty {
                    HStack(spacing: Space.s4) { ForEach(Array(places.enumerated()), id: \.offset) { osdReading($0.element, .titleLarge) } }
                }
            }
            .opacity(silent ? 0.45 : 1)
        } else {
            ViewThatFits(in: .horizontal) {
                HStack(spacing: Space.s4) { ForEach(Array(shown.enumerated()), id: \.offset) { osdReading($0.element, .titleMedium) } }
                PlanFlowRow(spacing: Space.s4, lineSpacing: 2, alignment: .center) { ForEach(Array(shown.enumerated()), id: \.offset) { osdReading($0.element, .titleMedium) } }
            }
            .opacity(silent ? 0.45 : 1)
        }
    }

    private func fullReadings(_ shown: [Instrument], _ stale: String, _ silent: Bool) -> some View {
        let tiles = shown.map(TelemetryTile.reading) + (chooser ? [TelemetryTile.chooser] : [])
        let perRow = max(columns ?? rowWidth(shown.count + 1), 1)
        let rows = stride(from: 0, to: tiles.count, by: perRow).map { Array(tiles[$0..<min($0 + perRow, tiles.count)]) }
        return VStack(alignment: .leading, spacing: 0) {
            if silent {
                Text(stale)
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.error)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s2)
            }
            VStack(alignment: .leading, spacing: 6) {
                ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                    HStack(alignment: .bottom, spacing: 0) {
                        ForEach(Array(row.enumerated()), id: \.offset) { at, tile in
                            if at > 0 { Spacer(minLength: 0) }
                            tileView(tile)
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .opacity(silent ? 0.45 : 1)
        }
    }

    @ViewBuilder
    private func tileView(_ tile: TelemetryTile) -> some View {
        switch tile {
        case .reading(let instrument):
            let display = displayFor(displays, instrument)
            VStack(alignment: .leading, spacing: 0) {
                ValueLabel(display: display, raw: instrument.raw, label: instrument.label.uppercased(), fallback: theme.colors.onSurfaceVariant)
                HStack(alignment: .lastTextBaseline, spacing: 3) {
                    Text(instrument.value)
                        .font(scaledNumber(flyScreen.layout.valueSize.scale, base: telemetrySize))
                        .foregroundStyle(displayColour(display, instrument.raw).map(argbColor) ?? theme.colors.onSurface)
                    if display.showUnits && !instrument.units.isBlank {
                        Text(instrument.units).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
            }
            .padding(.horizontal, Space.s3)
            .padding(.vertical, 6)
            .contentShape(Rectangle())
            .onTapGesture { if !instrument.id.isBlank { onStyle(instrument) } }
        case .chooser:
            Button { flyScreen.choosingReadings = true } label: {
                Image(.tune).foregroundStyle(theme.colors.onSurfaceVariant).frame(width: 48, height: 48)
            }
            .accessibilityLabel("Readings")
        }
    }
}

func scaledNumber(_ scale: Double, base: CGFloat = TypeScale.telemetry.size) -> Font {
    TypeScale.telemetry.font(base * scale)
}

@Observable
final class FlightActionsState {
    var pending: GuidedAction?
    var sentName: String?
    var sentSnapshot: String?
    var guidedValue: OpenGuidedValue?
    var showMore = false
    var showGripper = false
    var editingLoiter: LoiterOffer?
    var missionReady: Set<String>?
    var popupDue: GuidedOffer?
    @ObservationIgnored var mounted = 0
    @ObservationIgnored let scope = ViewScope()

    @MainActor
    func reset() {
        scope.cancel()
        pending = nil
        sentName = nil
        sentSnapshot = nil
        guidedValue = nil
        showMore = false
        showGripper = false
        editingLoiter = nil
        missionReady = nil
        popupDue = nil
    }
}

struct FlightActions<Center: View>: View {
    var layout: FlyDeckLayout = .Bottom
    @ViewBuilder var center: () -> Center
    @Environment(\.theme) private var theme
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(FlyMapEdits.self) private var mapEdits
    @QgcPath(FLY_STATE) private var stateJson
    @QgcPath(MAP_CLICK_PATH) private var mapClickJson
    @QgcPath(PREFLIGHT) private var preflightJson
    @QgcPath(GUIDED_ACTIONS) private var actionsJson
    @QgcBool(settingControl("settings.flyViewSettings.enableAutomaticMissionPopups")) private var automaticMissionPopups

    private var actions: FlightActionsState { flyScreen.flightActions }
    private var checklist: PreflightChecklistState { flyScreen.checklist }
    private var state: FlyState? { flyState(stateJson) }
    private var armed: Bool { state?.armed == true }
    private var offers: [String: GuidedOffer] { guidedOffers(actionsJson) }

    var body: some View {
        flightActions
            .onAppear { actions.mounted += 1 }
            .onDisappear { left() }
    }

    private func left() {
        let actions = actions
        let mapEdits = mapEdits
        actions.mounted -= 1
        Task { @MainActor in
            if actions.mounted == 0 {
                actions.reset()
                mapEdits.gotoLoiter = nil
            }
        }
    }

    @ViewBuilder
    private var flightActions: some View {
        let available = state?.connected == true
        if !available {
            Group {
                if layout != .Rail {
                    Text("Connect a vehicle to enable flight controls.")
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .padding(Space.s4)
                } else {
                    Color.clear.frame(width: 0, height: 0)
                }
            }
            .background { PreflightChecklistReset(checklist: checklist, available: false) }
            .onChange(of: offers, initial: true) { _, now in offersChanged(now) }
            .task(id: actions.popupDue) { await popupSettles() }
        } else {
            let rail = layout == .Rail
            Group {
                deck
                    .background { hosts }
                    .onChange(of: offers, initial: true) { _, now in offersChanged(now) }
                    .task(id: actions.popupDue) { await popupSettles() }
                    .onChange(of: flyScreen.deckRequest, initial: true) { _, asked in deckRequested(asked) }
                    .onChange(of: loiterOffer(mapClickJson) == nil, initial: true) { _, gone in if gone { actions.editingLoiter = nil } }
                    .onChange(of: gripperOffers(moreActions(offers)).isEmpty, initial: true) { _, empty in if empty { actions.showGripper = false } }
                if let open = actions.guidedValue {
                    DecisionHost(rail: rail) { GuidedValueFlow(open: open) { actions.guidedValue = nil } }
                }
                if let offer = actions.editingLoiter {
                    DecisionHost(rail: rail) {
                        LoiterRadiusPanel(offer: offer, units: mapClickUnits(mapClickJson), onRefused: { flyScreen.refusal = $0 }, onDone: { actions.editingLoiter = nil })
                    }
                }
            }
        }
    }

    private var readiness: Readiness? { guidedReadiness(state) }

    private func entries() -> [DeckEntry] {
        let actions = actions
        return flightDeckEntries(FlightDeckContext(
            offers: offers,
            armed: armed,
            scope: actions.scope,
            confirm: { actions.pending = $0 },
            openValue: openValue,
            report: { flyScreen.refusal = $0 },
            withdraw: { flyScreen.refusal = withdrawn(flyScreen.refusal, $0) },
            openChecklist: preflightOffered(preflightJson) ? { checklist.open() } : nil,
            readiness: readiness
        ))
    }

    private func openValue(_ kind: GuidedValueKind) {
        let actions = actions
        actions.scope.launch {
            let opened = await openGuidedValue(kind)
            if let opened { actions.guidedValue = opened } else { flyScreen.refusal = kind.missingRange }
        }
    }

    private func guidedActionFor(_ offer: GuidedOffer) -> GuidedAction? {
        guidedCommand(offer.id, resumeFromSequence(actionsJson)).map { command in
            GuidedAction(name: offer.title, confirm: offer.prompt, destructive: offer.destructive, offerId: offer.id, run: command)
        }
    }

    private func offersChanged(_ now: [String: GuidedOffer]) {
        if offerWithdrawn(actions.pending?.offerId, now) { actions.pending = nil }
        if offerWithdrawn(actions.guidedValue?.kind.offerId, now) { actions.guidedValue = nil }
        let popup = actions.missionReady.flatMap { autoMissionPopup($0, now, automaticMissionPopups) }
        actions.missionReady = Set(AUTO_POPUP_ACTIONS.filter { now[$0]?.ready == true })
        if let popup { actions.popupDue = popup }
    }

    private func popupSettles() async {
        guard let due = actions.popupDue else { return }
        try? await Task.sleep(for: .milliseconds(MISSION_POPUP_DELAY_MS))
        guard !Task.isCancelled else { return }
        actions.popupDue = nil
        guard let settled = offers[due.id], settled.ready else { return }
        if actions.pending == nil || popupReplacesOpenConfirm(settled.id), let action = guidedActionFor(settled) {
            actions.pending = action
        }
    }

    private func deckRequested(_ asked: String?) {
        guard let asked else { return }
        let deckEntries = entries()
        if asked == ARM_REQUEST {
            if let entry = deckEntries.first(where: { $0.id == ARM_REQUEST && $0.enabled }) {
                entry.onClick()
            } else if let stop = armedStopOffer(offers, armed) {
                actions.pending = emergencyStopAction(stop)
            } else {
                flyScreen.refusal = deckRequestRefusal(offers[armed ? "disarm" : "arm"])
            }
        } else if let entry = deckEntries.first(where: { $0.id == asked && $0.enabled }) {
            entry.onClick()
        } else if let offer = offers[asked], offer.ready, let action = guidedActionFor(offer) {
            actions.pending = action
        }
        flyScreen.deckRequest = nil
    }

    @ViewBuilder
    private var hosts: some View {
        PreflightChecklistReset(checklist: checklist, available: true)
        PreflightChecklist(checklist: checklist, deciding: actions.pending != nil || actions.guidedValue != nil || actions.editingLoiter != nil)
        OpenOnRequest(name: "more", open: { actions.showMore = true })
        if actions.showMore {
            MoreActionsSheet(tiles: moreTiles(), onDismiss: { actions.showMore = false }) {
                FlyViewMavlinkActions { actions.showMore = false }
            }
        }
        let gripper = gripperOffers(moreActions(offers))
        if actions.showGripper && !gripper.isEmpty {
            GripperPanel(offers: gripper) { actions.showGripper = false }
        }
    }

    private func moreTiles() -> [MoreTile] {
        let actions = actions
        let deckEntries = entries()
        let deck = deckIds(Set(deckEntries.map(\.id)), armed)
        let deckShown = Set(deck.map(\.0))
        let deckRest = deckEntries.filter { entry in !deck.contains { $0.0 == entry.id } }
        let extras = moreActions(offers)
        let gripper = gripperOffers(extras)
        let loiter = loiterOffer(mapClickJson)
        let checklistPast = checklistOffered(armed)
        let stop: [MoreTile] = [armedStopOffer(offers, armed).map { offer in
            MoreTile(label: STOP_MOTORS, icon: .warning, enabled: true, warning: true, onClick: { actions.pending = emergencyStopAction(offer) })
        }].compactMap { $0 }
        let rest = deckRest.filter { $0.id != CHECKLIST }.map { MoreTile(label: $0.label, icon: $0.icon, enabled: $0.enabled, warning: $0.warning, onClick: $0.onClick) }
        let fixed: [MoreTile] = [
            preflightOffered(preflightJson) ? MoreTile(label: "Checklist", icon: .checkCircle, enabled: checklistPast == nil, onClick: { checklist.open() }) : nil,
            loiter.map { offer in MoreTile(label: offer.title, icon: .myLocation, enabled: true, onClick: { actions.editingLoiter = offer }) },
            gripper.isEmpty ? nil : MoreTile(label: "Gripper", icon: .download, enabled: gripper.contains(where: \.ready), onClick: { actions.showGripper = true }),
            MoreTile(label: "Choose readings", icon: .tune, enabled: true, onClick: { flyScreen.choosingReadings = true }),
            armed ? nil : MoreTile(label: "Edit layout", icon: .edit, enabled: true, onClick: { flyScreen.layout.startEditing() }),
        ].compactMap { $0 }
        let offered = extras.filter { !deckShown.contains($0.id) && !GRIPPER_ACTIONS.contains($0.id) }.map { offer in
            MoreTile(label: offer.title, icon: guidedIcon(offer.id), enabled: offer.ready, warning: offer.destructive, onClick: {
                if offer.id == PAUSE {
                    openValue(altitudeValue(true))
                } else if let action = guidedActionFor(offer) {
                    actions.pending = action
                }
            })
        }
        return stop + rest + fixed + offered
    }

    @ViewBuilder
    private var deck: some View {
        let deckEntries = entries()
        let deck = deckIds(Set(deckEntries.map(\.id)), armed)
        let liveActions = actionsJson?.text
        let showingSent = sentIsStillShowing(actions.sentName, actions.sentSnapshot, liveActions)
        let deciding = actions.pending != nil || actions.guidedValue != nil || actions.editingLoiter != nil
        let more = DeckEntry(id: "more", label: "More", icon: .moreVert, enabled: true, onClick: { actions.showMore = true })
        if layout == .Rail {
            ZStack {
                if !deciding {
                    VStack(spacing: Space.s2) {
                        ForEach(deck, id: \.0) { id, primary in
                            if let entry = deckEntries.first(where: { $0.id == id }) {
                                RailDeckButton(entry: entry, primary: primary, labelled: armed)
                            }
                        }
                        RailDeckButton(entry: more, primary: false)
                    }
                    .padding(.leading, Space.s3)
                    .padding(.bottom, RAIL_LIFT)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                }
                if flyScreen.refusal != nil || actions.pending != nil || showingSent {
                    VStack(alignment: .leading, spacing: Space.s3) {
                        if let message = flyScreen.refusal {
                            Text(message).font(.bodyMedium).foregroundStyle(theme.colors.error)
                        }
                        decision(showingSent, liveActions)
                    }
                    .padding(Space.s4)
                    .background(theme.colors.surfaceContainerLow, in: RoundedRectangle(cornerRadius: Corner.extraLarge))
                    .frame(maxWidth: DECISION_CARD_WIDTH)
                    .padding(Space.s3)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            VStack(alignment: .leading, spacing: Space.s3) {
                if let message = flyScreen.refusal {
                    Text(message)
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.error)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                decision(showingSent, liveActions)
                TelemetryRow(valuesShown: true, chooser: false, compact: true)
                    .frame(maxWidth: .infinity)
                if !deciding, let readiness {
                    DeckReadiness(readiness: readiness)
                }
                if actions.pending == nil {
                    let buttons = deck.compactMap { id, primary in deckEntries.first { $0.id == id }.map { ($0, primary) } } + [(more, false)]
                    let leading = (buttons.count + 1) / 2
                    HStack(spacing: 10) {
                        ForEach(Array(buttons.prefix(leading).enumerated()), id: \.offset) { _, button in
                            deckButton(button.0, button.1, deck.isEmpty)
                        }
                        center()
                        ForEach(Array(buttons.dropFirst(leading).enumerated()), id: \.offset) { _, button in
                            deckButton(button.0, button.1, deck.isEmpty)
                        }
                    }
                }
            }
            .padding(Space.s4)
        }
    }

    @ViewBuilder
    private func deckButton(_ entry: DeckEntry, _ primary: Bool, _ deckEmpty: Bool) -> some View {
        if entry.id == "more" && !deckEmpty {
            DeckButton(entry: entry, primary: primary).frame(width: 64)
        } else {
            DeckButton(entry: entry, primary: primary).frame(maxWidth: .infinity)
        }
    }

    @ViewBuilder
    private func decision(_ showingSent: Bool, _ liveActions: String?) -> some View {
        let actions = actions
        if let confirming = actions.pending {
            ConfirmTrack(
                action: confirming,
                onSent: {
                    actions.sentName = confirming.name
                    actions.sentSnapshot = liveActions
                    actions.pending = nil
                },
                onCancel: { actions.pending = nil }
            )
        } else if showingSent {
            SentNotice(name: actions.sentName ?? "", onDismiss: { actions.sentName = nil })
        }
    }
}

extension FlightActions where Center == EmptyView {
    init(layout: FlyDeckLayout = .Bottom) {
        self.init(layout: layout, center: { EmptyView() })
    }
}

private let DECISION_CARD_WIDTH: CGFloat = 460
private let OSD_LABEL_ALPHA = 0.75
let STOP_MOTORS = "Stop motors"
private let RAIL_LIFT: CGFloat = 56

private let OSD_LABELS = [
    "distance to home": "D",
    "alt (rel)": "H",
    "altitude": "H",
    "ground speed": "H.S",
    "climb rate": "V.S",
    "air speed": "A.S",
    "airspeed": "A.S",
    "distance to operator": "D.OP",
    "from you": "D.OP",
    "heading": "HDG",
    "distancetohome": "D",
    "altituderelative": "H",
    "groundspeed": "H.S",
    "climbrate": "V.S",
]

func osdLabel(_ label: String) -> String { OSD_LABELS[label.trimmed.lowercased()] ?? label.uppercased() }

func osdIsSpeed(_ label: String) -> Bool { ["H.S", "V.S", "A.S"].contains(osdLabel(label)) }

private struct DecisionHost<Content: View>: View {
    let rail: Bool
    @ViewBuilder let content: () -> Content

    var body: some View {
        if rail {
            content()
                .frame(maxWidth: DECISION_CARD_WIDTH)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .padding(Space.s3)
        } else {
            content()
        }
    }
}

struct RangeHint: View {
    @Environment(\.theme) private var theme
    let text: String

    var body: some View {
        Text(text).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
    }
}

private func armedNow() -> Bool { flyState(Qgc.get(FLY_STATE))?.armed == true }

private func flightModeNow() -> String { flyState(Qgc.get(FLY_STATE))?.mode ?? "" }

private func reachedWithin(_ reached: @escaping @Sendable () -> Bool, _ timeoutMs: Int?, _ pollMs: Int) async -> Bool {
    let started = Date()
    while !Task.isCancelled {
        if await offMain(reached) { return true }
        if let timeoutMs, Date().timeIntervalSince(started) * 1000 >= Double(timeoutMs) { return false }
        try? await Task.sleep(for: .milliseconds(pollMs))
    }
    return false
}

@MainActor
private func attemptCommand(_ scope: ViewScope, _ action: String, report: @escaping (String?) -> Void, withdraw: @escaping (String) -> Void, reached: @escaping @Sendable () -> Bool, call: @escaping @Sendable () -> Void) {
    scope.launch {
        report(nil)
        _ = await offMain(call)
        let confirmed = await reachedWithin(reached, COMMAND_SETTLE_MS, 200)
        guard let refused = commandRefusal(action, confirmed) else { return }
        report(refused)
        guard await reachedWithin(reached, nil, LATE_CONFIRM_POLL_MS) else { return }
        withdraw(refused)
    }
}

let COMMAND_SETTLE_MS = 4000
let LATE_CONFIRM_POLL_MS = 500

func withdrawn(_ shown: String?, _ late: String) -> String? { shown == late ? nil : shown }

func commandRefusal(_ action: String, _ confirmed: Bool) -> String? {
    confirmed ? nil : "\(action) was not confirmed by the aircraft."
}

let FLIGHT_MODE_SETTINGS_PAGE = "Flight Mode Settings"

private func modeSettled(_ mode: String, _ before: ModeAck?) async -> ModeOutcome {
    let started = Date()
    while !Task.isCancelled {
        try? await Task.sleep(for: .milliseconds(200))
        let elapsed = Int64(Date().timeIntervalSince(started) * 1000)
        let outcome = await offMain { modeOutcome(mode, before, modeAck(Qgc.get(FLIGHT_MODES)), flightModeNow() == mode, elapsed) }
        if case .Pending = outcome { continue }
        return outcome
    }
    return .Settled
}

struct FlightModeMenu: View {
    let expanded: Bool
    let onDismiss: () -> Void
    let onStatus: () -> Void
    let onMessages: () -> Void
    @Environment(\.theme) private var theme
    @Environment(AppNavigationState.self) private var navigation
    @Environment(FlyScreenState.self) private var flyScreen
    @AdvancedUiShown private var advanced
    @QgcPath(FLIGHT_MODES) private var json
    @QgcPath(FLY_STATE) private var flyJson
    @QgcPath(SETUP) private var setupJson
    @State private var showFolded = false
    @State private var editing = false
    @State private var confirming: FlightModeOption?
    @State private var settings = false

    var body: some View {
        if let modes = flightModesView(json) {
            Color.clear
                .accessibilityHidden(true)
                .popover(isPresented: Binding(get: { expanded }, set: { shown in
                    if !shown {
                        onDismiss()
                        showFolded = false
                        editing = false
                    }
                })) {
                    ViewThatFits(in: .vertical) {
                        menu(modes)
                        ScrollView { menu(modes) }
                    }
                    .frame(minWidth: 280, maxWidth: 320)
                    .presentationCompactAdaptation(.popover)
                }
                .background {
                    if settings {
                        AircastSheet(onDismissRequest: { settings = false }) {
                            ParameterForm(page: FLIGHT_MODE_SETTINGS_PAGE)
                            if setupComponents(setupJson).contains(where: { $0.name == FLIGHT_MODES_PAGE }) && advanced {
                                Button("Configure flight modes") {
                                    settings = false
                                    navigation.setupPage = FLIGHT_MODES_PAGE
                                }
                                .buttonStyle(.borderless)
                                .padding(Space.s2)
                            }
                        }
                    }
                }
                .alert(
                    "Set flight mode",
                    isPresented: Binding(get: { confirming != nil }, set: { if !$0 { confirming = nil } }),
                    presenting: confirming
                ) { mode in
                    Button("Confirm") {
                        confirming = nil
                        send(mode)
                    }
                    Button("Cancel", role: .cancel) { confirming = nil }
                } message: { mode in
                    Text("Set the vehicle flight mode to \(mode.name)")
                }
        }
    }

    private func onRefusal(_ text: String?) { flyScreen.refusal = text }

    private func onWithdraw(_ text: String) { flyScreen.refusal = withdrawn(flyScreen.refusal, text) }

    private func send(_ mode: FlightModeOption) {
        Task {
            onRefusal(nil)
            flyScreen.pendingMode = mode.name
            let before = await offMain { modeAck(Qgc.get(FLIGHT_MODES)) }
            _ = await offMain { VehicleCommands.setFlightMode(mode.name) }
            let outcome = await modeSettled(mode.name, before)
            flyScreen.pendingMode = nil
            if case .Rejected(let text) = outcome {
                onRefusal(text)
                try? await Task.sleep(for: .milliseconds(MODE_REJECTION_MS))
                onWithdraw(text)
            }
        }
    }

    private func toggleHidden(_ name: String, _ setting: String) {
        offMain {
            if let now = flightModesView(Qgc.get(FLIGHT_MODES)) {
                Qgc.set(setting, hiddenModesAfter(now.hidden, name, !now.hidden.contains(name)))
            }
        }
    }

    private func choose(_ mode: FlightModeOption) {
        onDismiss()
        showFolded = false
        if mode.needsConfirm { confirming = mode } else { send(mode) }
    }

    private func menu(_ modes: FlightModesView) -> some View {
        let warning = readinessWarning(flyState(flyJson))
        let setting = modes.hiddenSetting
        let heading = modeHeading(modes)
        let shown = editing && setting != nil ? [] : showFolded ? modes.all : modes.everyday
        return VStack(alignment: .leading, spacing: 0) {
            menuRow(action: {
                onDismiss()
                onStatus()
            }) {
                Image(warning == nil ? .checkCircle : .warning)
                    .foregroundStyle(warning == nil ? theme.colors.onSurfaceVariant : theme.aircast.warning)
                Text(warning ?? VEHICLE_STATUS).font(.bodyMedium).frame(maxWidth: 280, alignment: .leading)
                if warning != nil {
                    Text("Details").font(.labelLarge).foregroundStyle(theme.colors.primary)
                }
            }
            Divider()
            menuRow(action: {
                onDismiss()
                onMessages()
            }) {
                Image(.notifications).foregroundStyle(theme.colors.onSurfaceVariant)
                Text("Messages").font(.bodyMedium)
            }
            Divider()
            if heading != nil || setting != nil {
                HStack {
                    Text(heading ?? "")
                        .font(.labelSmall)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .padding(.vertical, Space.s2)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    if setting != nil {
                        Button(editing ? "Done" : "Edit") { editing.toggle() }.buttonStyle(.borderless)
                    }
                }
                .padding(.leading, Space.s4)
                .padding(.trailing, Space.s1)
            }
            if editing, let setting {
                ForEach(modes.all, id: \.name) { mode in
                    menuRow(action: {
                        let value = hiddenModesAfter(modes.hidden, mode.name, !mode.hidden)
                        offMain { Qgc.set(setting, value) }
                    }) {
                        Text(mode.name).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                        Toggle("", isOn: .constant(!mode.hidden)).labelsHidden().allowsHitTesting(false)
                    }
                }
            }
            if !modes.unknownModeNotice.isBlank {
                Text(modes.unknownModeNotice)
                    .font(.labelSmall)
                    .foregroundStyle(theme.aircast.warning)
                    .frame(maxWidth: 280, alignment: .leading)
                    .padding(.horizontal, Space.s4)
                    .padding(.vertical, Space.s2)
            }
            ForEach(Array(shown.enumerated()), id: \.element.name) { index, mode in
                if startsSection(shown, index) { Divider() }
                modeRow(mode, modes)
            }
            if !modes.folded.isEmpty && !showFolded && !editing {
                menuRow(action: { showFolded = true }) { Text("More modes").font(.bodyMedium) }
            }
            menuRow(action: {
                onDismiss()
                settings = true
            }) { Text("Flight mode settings").font(.bodyMedium) }
        }
        .padding(.vertical, Space.s2)
    }

    private func modeRow(_ mode: FlightModeOption, _ modes: FlightModesView) -> some View {
        HStack(spacing: Space.s3) {
            Image(flightModeIcon(mode.name)).foregroundStyle(theme.colors.onSurfaceVariant).frame(width: 24)
            VStack(alignment: .leading, spacing: 0) {
                Text(mode.name).font(.titleSmall)
                if !mode.summary.isBlank {
                    Text(mode.summary).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .frame(maxWidth: 280, alignment: .leading)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, Space.s3)
        .frame(minHeight: 48)
        .background(mode.current ? theme.colors.secondaryContainer : Color.clear)
        .opacity(mode.hidden ? HIDDEN_MODE_ALPHA : 1)
        .opacity(modes.canSet ? 1 : 0.38)
        .contentShape(Rectangle())
        .onTapGesture { if modes.canSet { choose(mode) } }
        .onLongPressGesture { if let setting = modes.hiddenSetting { toggleHidden(mode.name, setting) } }
    }

    private func menuRow<Content: View>(action: @escaping () -> Void, @ViewBuilder content: () -> Content) -> some View {
        Button(action: action) {
            HStack(spacing: Space.s3) { content() }
                .padding(.horizontal, Space.s3)
                .frame(minHeight: 48)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct InstrumentSheet: View {
    @Environment(\.theme) private var theme
    @HasVehicle private var connected
    let chosen: [String]
    var title: String = "Instrument tiles"
    let onToggle: (String) -> Void
    let onDismiss: () -> Void
    @State private var groups: [InstrumentGroup] = []
    @State private var shownAtOpen: [String]?

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            Text(title).font(.titleLarge).padding(.horizontal, Space.s4)
            Text(instrumentChoiceNote(chosen))
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .padding(.horizontal, Space.s4)
                .padding(.bottom, Space.s2)
            if groups.isEmpty {
                FootNote(text: emptyCatalogueText(connected))
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        ForEach(shownFirst(groups, shownAtOpen ?? chosen), id: \.group) { group in
                            SectionHeader(text: sentenceCase(group.title))
                            ForEach(group.facts, id: \.path) { fact in
                                HStack(spacing: Space.s4) {
                                    Image(systemName: chosen.contains(fact.path) ? "checkmark.square.fill" : "square")
                                        .font(.title3)
                                        .foregroundStyle(chosen.contains(fact.path) ? theme.colors.primary : theme.colors.onSurfaceVariant)
                                    Text(sentenceCase(fact.label)).font(.bodyLarge)
                                    Spacer()
                                }
                                .padding(.horizontal, Space.s4)
                                .frame(minHeight: 56)
                                .contentShape(Rectangle())
                                .onTapGesture { onToggle(fact.path) }
                            }
                        }
                    }
                }
            }
        }
        .onAppear { if shownAtOpen == nil { shownAtOpen = chosen } }
        .task(id: connected) {
            groups = await offMain {
                let catalogue = Qgc.get(INSTRUMENT_GROUPS)
                return [vehicleOwnGroup(catalogue)].compactMap { $0 } + instrumentGroups(catalogue)
            }
        }
    }
}

private let FLIGHT_MODE_ICONS: [([String], Icon)] = [
    (["rtl", "return", "smart_rtl", "smartrtl"], .home),
    (["land", "precland"], .flightLand),
    (["takeoff"], .flightTakeoff),
    (["auto", "mission"], .route),
    (["guided", "offboard", "follow"], .locationOn),
    (["althold", "altitude", "alt"], .height),
    (["loiter", "position", "poshold", "hold", "brake"], .myLocation),
    (["stabilize", "stabilized", "manual", "acro", "sport", "drift"], .gamepad),
]

func flightModeIcon(_ name: String) -> Icon {
    let words = name.lowercased().split(whereSeparator: { $0 == " " || $0 == "_" || $0 == "-" }).map(String.init)
    let joined = words.joined()
    return FLIGHT_MODE_ICONS.first { keys, _ in keys.contains { $0 == joined || words.contains($0) } }?.1 ?? .flight
}

private struct DeckReadiness: View {
    @Environment(\.theme) private var theme
    let readiness: Readiness

    var body: some View {
        HStack(spacing: 10) {
            Image(.warning)
                .font(.system(size: 16))
                .foregroundStyle(readiness.blocks ? theme.colors.error : theme.aircast.warning)
                .frame(width: 20, height: 20)
            Text(readiness.text).font(.bodyMedium).lineLimit(2)
        }
        .padding(.horizontal, Space.s3)
        .padding(.vertical, Space.s2)
        .frame(maxWidth: .infinity, alignment: .leading)
        .foregroundStyle(readiness.blocks ? theme.colors.onErrorContainer : theme.colors.onSurface)
        .background(readiness.blocks ? theme.colors.errorContainer : theme.aircast.warningContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
    }
}

let VEHICLE_STATUS = "Vehicle status"

final class FlightDeckContext {
    let offers: [String: GuidedOffer]
    let armed: Bool
    let scope: ViewScope
    let confirm: (GuidedAction) -> Void
    let openValue: (GuidedValueKind) -> Void
    let report: (String?) -> Void
    let withdraw: (String) -> Void
    let openChecklist: (() -> Void)?
    let readiness: Readiness?

    init(offers: [String: GuidedOffer], armed: Bool, scope: ViewScope = ViewScope(), confirm: @escaping (GuidedAction) -> Void, openValue: @escaping (GuidedValueKind) -> Void, report: @escaping (String?) -> Void, withdraw: @escaping (String) -> Void, openChecklist: (() -> Void)?, readiness: Readiness? = nil) {
        self.offers = offers
        self.armed = armed
        self.scope = scope
        self.confirm = confirm
        self.openValue = openValue
        self.report = report
        self.withdraw = withdraw
        self.openChecklist = openChecklist
        self.readiness = readiness
    }
}

@MainActor
func flightDeckEntries(_ deck: FlightDeckContext) -> [DeckEntry] {
    let offers = deck.offers
    let armed = deck.armed
    let armAction = offers[armed ? "disarm" : "arm"]
    let armTitle = armAction?.title ?? (armed ? "Disarm" : "Arm")
    let takeoff = offers["takeoff"]
    let rtl = offers["rtl"]
    let land = offers["land"]
    let entries: [DeckEntry?] = [
        armAction?.shown == true ? DeckEntry(id: "arm", label: armTitle, icon: .bolt, enabled: armAction?.ready == true, warning: armed, onClick: {
            deck.confirm(GuidedAction(
                name: armTitle,
                confirm: armAction.flatMap { $0.prompt.isBlank ? nil : $0.prompt } ?? (armed ? "Disarm the vehicle" : "Arm the vehicle."),
                destructive: armAction?.destructive ?? true,
                offerId: armed ? "disarm" : "arm",
                run: {
                    let target = !armed
                    attemptCommand(deck.scope, target ? "Arm" : "Disarm", report: deck.report, withdraw: deck.withdraw, reached: { armedNow() == target }, call: { VehicleCommands.setArmed(target) })
                }
            ))
        }) : nil,
        takeoff?.shown == true ? DeckEntry(
            id: "takeoff",
            label: deck.readiness == nil ? HOLD_TO_TAKE_OFF : TAKE_OFF,
            icon: .flightTakeoff,
            enabled: takeoff?.ready == true,
            onHold: deck.readiness != nil ? nil : {
                let heightless = takeoff?.carriesValue == false
                deck.scope.launch {
                    let refused = await offMain { () -> String? in
                        let target = heightless ? nil : holdTakeoffHeight(guidedTakeoff(Qgc.get(GUIDED_TAKEOFF))).flatMap { height in guidedTakeoff(Qgc.get(guidedTakeoffPath(height))) }
                        if heightless { return VehicleCommands.takeoffRefusal() }
                        guard let target else { return "This vehicle did not report a takeoff height range." }
                        return VehicleCommands.takeoffRefusal(target.targetMeters)
                    }
                    deck.report(refused)
                }
            },
            onClick: {
                if takeoff?.carriesValue == false {
                    deck.confirm(GuidedAction(
                        name: takeoff?.title ?? "Takeoff",
                        confirm: takeoff.flatMap { $0.prompt.isBlank ? nil : $0.prompt } ?? "Takeoff from ground and hold position.",
                        destructive: false,
                        offerId: "takeoff",
                        run: { offMain { VehicleCommands.takeoff() } }
                    ))
                } else {
                    deck.openValue(takeoffValue(takeoff))
                }
            }
        ) : nil,
        offers[PAUSE]?.shown == true ? DeckEntry(id: PAUSE, label: offers[PAUSE]?.title ?? "Pause", icon: .pause, enabled: offers[PAUSE]?.ready == true, onClick: {
            deck.openValue(altitudeValue(true))
        }) : nil,
        rtl?.shown == true ? DeckEntry(id: "rtl", label: "Return", icon: .home, enabled: rtl?.ready == true, onClick: {
            deck.confirm(GuidedAction(
                name: rtl?.title ?? "Return",
                confirm: rtl.flatMap { $0.prompt.isBlank ? nil : $0.prompt } ?? "Return to the launch position of the vehicle",
                destructive: false,
                option: rtl.flatMap { $0.option.isBlank ? nil : $0.option }.map { label in
                    ConfirmOption(label: label, run: { smart in offMain { VehicleCommands.returnToLaunch(smart) } })
                },
                offerId: "rtl",
                run: { offMain { VehicleCommands.returnToLaunch(false) } }
            ))
        }) : nil,
        land?.shown == true ? DeckEntry(
            id: "land",
            label: holdLabel(land?.title ?? "Land"),
            icon: .flightLand,
            enabled: land?.ready == true,
            onHold: { deck.scope.launch { deck.report(await offMain { VehicleCommands.landRefusal() }) } },
            onClick: {
                deck.confirm(GuidedAction(
                    name: land?.title ?? "Land",
                    confirm: land.flatMap { $0.prompt.isBlank ? nil : $0.prompt } ?? "Land the vehicle at the current position",
                    destructive: false,
                    offerId: "land",
                    run: { offMain { VehicleCommands.land() } }
                ))
            }
        ) : nil,
        offers["changeSpeed"]?.shown == true ? DeckEntry(id: "changeSpeed", label: "Speed", icon: .speed, enabled: offers["changeSpeed"]?.ready == true, onClick: {
            deck.openValue(speedValue(offers["changeSpeed"]))
        }) : nil,
        offers["changeAltitude"]?.shown == true ? DeckEntry(id: "changeAltitude", label: "Altitude", icon: .height, enabled: offers["changeAltitude"]?.ready == true, onClick: {
            deck.openValue(altitudeValue(false))
        }) : nil,
        deck.openChecklist != nil && checklistOffered(armed) == nil ? DeckEntry(id: CHECKLIST, label: "Checklist", icon: .checkCircle, enabled: true, onClick: {
            deck.openChecklist?()
        }) : nil,
    ]
    return entries.compactMap { $0 }
}
