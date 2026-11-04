import SwiftUI

let MAP_CLICK_PATH = "view.mapClick"

struct MapPoint: Equatable, Hashable {
    var latitude: Double
    var longitude: Double
}

let ORBIT_ACTION = "Orbit"
let GOTO_ACTION = "GoTo"
let RETURN_HOME_INSTEAD = "Return home instead"

struct GotoPreview: Equatable {
    var distance: String
    var height: String
    var verdict: String
    var pastReturnPoint: Bool
}

func gotoPreviewPath(_ point: MapPoint) -> String {
    String(format: "view.gotoPreview(%.7f,%.7f)", point.latitude, point.longitude)
}

func gotoPreview(_ view: JSON?) -> GotoPreview? {
    guard let view, view["available"].bool else { return nil }
    return GotoPreview(distance: view["distanceText"].string, height: view["heightText"].string, verdict: view["verdict"].string, pastReturnPoint: view["pastReturnPoint"].bool)
}

func gotoPreviewLines(_ preview: GotoPreview?, _ fallback: String) -> [String] {
    guard let preview else { return [fallback] }
    let lines = [[preview.distance, preview.height].filter { !$0.isBlank }.joined(separator: " \u{00b7} "), preview.verdict].filter { !$0.isBlank }
    return lines.isEmpty ? [fallback] : lines
}

private let COMPASS_POINTS = ["north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west"]

private func javaRound(_ value: Double) -> Int {
    saturatingInt((value + 0.5).rounded(.down))
}

func goHereText(_ from: MapPoint, _ to: MapPoint, _ unit: String, _ metresPerUnit: Double) -> String? {
    guard metresPerUnit > 0, !unit.isBlank else { return nil }
    let metres = metresBetween(TrackPoint(latitude: from.latitude, longitude: from.longitude), TrackPoint(latitude: to.latitude, longitude: to.longitude))
    let fromLat = from.latitude * .pi / 180
    let toLat = to.latitude * .pi / 180
    let deltaLon = (to.longitude - from.longitude) * .pi / 180
    let bearing = (atan2(sin(deltaLon) * cos(toLat), cos(fromLat) * sin(toLat) - sin(fromLat) * cos(toLat) * cos(deltaLon)) * 180 / .pi + 360)
        .truncatingRemainder(dividingBy: 360)
    let point = COMPASS_POINTS[javaRound(bearing / 45) % COMPASS_POINTS.count]
    return "\(javaRound(metres / metresPerUnit)) \(unit) \(point)"
}

struct MapClickAction: Equatable {
    var id: String
    var path: String
    var label: String
    var title: String
    var message: String
    var confirm: Bool
}

func mapClickActions(_ view: JSON?) -> [MapClickAction] {
    (view?["actions"].objects ?? []).map {
        MapClickAction(
            id: $0["id"].string,
            path: $0["path"].string,
            label: $0["label"].string,
            title: $0["title"].string,
            message: $0["message"].string,
            confirm: $0["confirm"].bool(true)
        )
    }
}

struct OrbitChoice: Equatable {
    var radiusMetres: Double
    var clockwise: Bool
    var aboveHomeMetres: Double
}

struct OrbitDefaults: Equatable {
    var radius: Double
    var unit: String
    var metresPerUnit: Double
    var clockwise: Bool
}

func orbitDefaults(_ view: JSON?) -> OrbitDefaults {
    OrbitDefaults(
        radius: view?["orbitDefaultRadius"].double.flatMap { $0.isNaN ? nil : $0 } ?? 0,
        unit: view?["orbitRadiusUnit"].string ?? "",
        metresPerUnit: view?["orbitMetresPerUnit"].double.flatMap { !$0.isNaN && $0 > 0 ? $0 : nil } ?? 1,
        clockwise: view?["orbitClockwise"].bool(true) ?? true
    )
}

func radiusMetres(_ entered: String, _ defaults: OrbitDefaults) -> Double? {
    typedNumber(entered.ifBlank(String(defaults.radius))).map { $0 * defaults.metresPerUnit }
}

func orbitOpened(_ point: MapPoint, _ defaults: OrbitDefaults) -> OrbitCircle {
    OrbitCircle(
        centre: TrackPoint(latitude: point.latitude, longitude: point.longitude),
        radiusMetres: max(radiusMetres("", defaults) ?? 0, MINIMUM_CIRCLE_RADIUS_METRES),
        clockwise: defaults.clockwise
    )
}

func orbitEdit(_ circle: OrbitCircle, _ defaults: OrbitDefaults) -> LoiterEdit {
    LoiterEdit(radiusMetres: circle.radiusMetres, clockwise: circle.clockwise, unit: defaults.unit, metresPerUnit: defaults.metresPerUnit)
}

func orbitArgs(_ point: MapPoint, _ choice: OrbitChoice) -> [Any] {
    [point.latitude, point.longitude, choice.radiusMetres, choice.clockwise, choice.aboveHomeMetres]
}

func coordinateLines(_ point: MapPoint) -> [String] {
    [String(format: "Lat: %.6f", point.latitude), String(format: "Lon: %.6f", point.longitude)]
}

private func send(_ action: MapClickAction, _ point: MapPoint, _ orbit: OrbitChoice?) -> String? {
    guard let orbit else {
        return Qgc.refusalOf(action.path, arguments: [["latitude": point.latitude, "longitude": point.longitude] as [String: Any]])
    }
    return Qgc.refusalOf(action.path, arguments: orbitArgs(point, orbit))
}

private struct RadiusField: View {
    let label: String
    let text: Binding<String>
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            Text(label).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            TextField(label, text: text)
                .keyboardType(.numbersAndPunctuation)
                .submitLabel(.done)
                .textFieldStyle(.roundedBorder)
        }
    }
}

private struct RefusalText: View {
    let text: String?
    @Environment(\.theme) private var theme

    var body: some View {
        if let text {
            Text(text).font(.bodyMedium).foregroundStyle(theme.colors.error)
        }
    }
}

func offeredClickActions(_ view: JSON?, _ only: [String]?) -> [MapClickAction] {
    mapClickActions(view).filter { action in only.map { $0.contains(action.id) } ?? true }
}

struct MapClickMenu: View {
    let point: MapPoint
    var only: [String]? = nil
    var warning: String? = nil
    let onDismiss: () -> Void
    @QgcPath(MAP_CLICK_PATH) private var view
    @QgcPath("vehicle.coordinate") private var vehicleCoordinate
    @State private var confirming: MapClickAction?
    @State private var refused: String?
    @State private var scope = ViewScope()
    @Environment(FlyMapEdits.self) private var mapEdits
    @Environment(\.theme) private var theme

    var body: some View {
        let actions = offeredClickActions(view, only)
        let defaults = orbitDefaults(view)
        let previewing = confirming?.id == GOTO_ACTION
        ZStack {
            if view != nil && actions.isEmpty {
                Color.clear
                    .frame(width: 0, height: 0)
                    .task(id: point) { onDismiss() }
            } else if let orbit = confirming, orbit.id == ORBIT_ACTION {
                OrbitPanel(point: point, action: orbit, defaults: defaults, onDone: onDismiss)
            } else {
                AircastSheet(onDismissRequest: { if confirming?.id != ORBIT_ACTION { onDismiss() } }) { sheet(actions, defaults) }
            }
        }
        .onChange(of: point) {
            confirming = nil
            refused = nil
        }
        .onChange(of: actions) { _, offered in
            guard let pending = confirming, !offered.contains(where: { $0.id == pending.id }) else { return }
            if pending.id == ORBIT_ACTION { onDismiss() } else { confirming = nil }
        }
        .onChange(of: Keys(previewing, point), initial: true) {
            mapEdits.gotoPreview = previewing ? TrackPoint(latitude: point.latitude, longitude: point.longitude) : nil
            mapEdits.gotoPreviewFrom = previewing ? geoOf(vehicleCoordinate).map { TrackPoint(latitude: $0.0, longitude: $0.1) } : nil
        }
        .onDisappear {
            scope.cancel()
            mapEdits.gotoPreview = nil
            mapEdits.gotoPreviewFrom = nil
        }
    }

    private func run(_ action: MapClickAction) {
        let point = point
        scope.launch {
            let answer = await offMain { send(action, point, nil) }
            guard !Task.isCancelled else { return }
            if answer == nil { onDismiss() } else { refused = answer }
        }
    }

    private func returnHome() {
        scope.launch {
            _ = await offMain { VehicleCommands.returnToLaunch(false) }
            guard !Task.isCancelled else { return }
            onDismiss()
        }
    }

    private func sheet(_ actions: [MapClickAction], _ defaults: OrbitDefaults) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            if let pending = confirming {
                let vehicleAt = geoOf(vehicleCoordinate).map { MapPoint(latitude: $0.0, longitude: $0.1) }
                let away = pending.id == GOTO_ACTION ? vehicleAt.flatMap { goHereText($0, point, defaults.unit, defaults.metresPerUnit) } : nil
                MapClickConfirm(
                    pending: pending,
                    away: away,
                    previewPath: pending.id == GOTO_ACTION ? gotoPreviewPath(point) : nil,
                    onConfirm: { run(pending) },
                    onCancel: { confirming = nil },
                    onReturnHome: returnHome
                )
                .padding(.horizontal, Space.s5)
            } else {
                ForEach(Array(actions.enumerated()), id: \.offset) { _, action in
                    Button {
                        refused = nil
                        if action.confirm { confirming = action } else { run(action) }
                    } label: {
                        Text(sentenceCase(action.label))
                            .font(.bodyLarge)
                            .foregroundStyle(theme.colors.onSurface)
                            .frame(maxWidth: .infinity, minHeight: 56, alignment: .leading)
                            .padding(.horizontal, Space.s4)
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
                if let warning {
                    Text(warning)
                        .font(.bodyMedium)
                        .foregroundStyle(theme.aircast.warning)
                        .padding(.horizontal, Space.s5)
                        .padding(.vertical, Space.s2)
                }
            }
            RefusalText(text: refused)
                .padding(.horizontal, Space.s5)
                .padding(.vertical, Space.s2)
            Divider().padding(.vertical, Space.s2)
            ForEach(coordinateLines(point), id: \.self) { line in
                Text(line)
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .frame(maxWidth: .infinity)
            }
        }
        .padding(.top, Space.s4)
        .padding(.bottom, Space.s6)
    }
}

private struct MapClickConfirm: View {
    let pending: MapClickAction
    let away: String?
    let onConfirm: () -> Void
    let onCancel: () -> Void
    let onReturnHome: () -> Void
    @QgcPath private var previewJson: JSON?
    @Environment(\.theme) private var theme

    init(pending: MapClickAction, away: String?, previewPath: String?, onConfirm: @escaping () -> Void, onCancel: @escaping () -> Void, onReturnHome: @escaping () -> Void) {
        self.pending = pending
        self.away = away
        self.onConfirm = onConfirm
        self.onCancel = onCancel
        self.onReturnHome = onReturnHome
        _previewJson = QgcPath(previewPath)
    }

    var body: some View {
        let preview = gotoPreview(previewJson)
        let lines = preview.map { shown -> [String] in
            var withAway = shown
            withAway.distance = away ?? shown.distance
            return gotoPreviewLines(withAway, pending.message)
        } ?? [away ?? pending.message]
        let warn = preview?.pastReturnPoint == true
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(sentenceCase(pending.title)).font(.titleLarge)
            ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                Text(line).font(.bodyMedium)
                    .foregroundStyle(warn && index == lines.count - 1 ? theme.aircast.warning : theme.colors.onSurfaceVariant)
            }
            if warn {
                Button(action: onReturnHome) {
                    Text(RETURN_HOME_INSTEAD).frame(maxWidth: .infinity)
                }
                .buttonStyle(.filled)
            }
            HoldOrCancel(label: pending.title, onConfirm: onConfirm, onCancel: onCancel)
        }
    }
}

private struct OrbitPanel: View {
    let point: MapPoint
    let action: MapClickAction
    let defaults: OrbitDefaults
    let onDone: () -> Void
    @Environment(FlyMapEdits.self) private var mapEdits
    @State private var opened: OrbitCircle?
    @State private var typed: String?
    @State private var height: GuidedAltitude?
    @State private var target: Double?
    @State private var settled: Double?
    @State private var refused: String?
    @State private var scope = ViewScope()
    @Environment(\.theme) private var theme

    var body: some View {
        let start = opened ?? orbitOpened(point, defaults)
        let circle = mapEdits.orbit ?? start
        let edit = orbitEdit(circle, defaults)
        GuidedValuePanel(
            title: sentenceCase(action.title),
            sentence: action.message,
            commitLabel: action.title,
            commitEnabled: height != nil,
            onCommit: { commit(circle) },
            onCancel: onDone
        ) {
            RadiusField(
                label: ["Radius", defaults.unit].filter { !$0.isBlank }.joined(separator: " "),
                text: Binding(
                    get: { loiterRadiusField(typed, edit) },
                    set: { text in
                        typed = text
                        mapEdits.orbit = OrbitCircle(centre: circle.centre, radiusMetres: loiterTyped(text, edit).radiusMetres, clockwise: circle.clockwise)
                    }
                )
            )
            Toggle("Clockwise", isOn: Binding(
                get: { circle.clockwise },
                set: { mapEdits.orbit = OrbitCircle(centre: circle.centre, radiusMetres: circle.radiusMetres, clockwise: $0) }
            ))
            if let reading = height {
                let low = reading.minimum ?? 0
                Text(reading.label).font(.bodySmall)
                Slider(
                    value: Binding(get: { target ?? reading.current ?? 0 }, set: { target = $0 }),
                    in: low...max(low, reading.maximum ?? 0),
                    onEditingChanged: { editing in if !editing { settled = target } }
                )
                if let hint = rangeLabel(reading.minimum, reading.maximum, reading.unit) {
                    RangeHint(text: hint)
                }
            }
            RefusalText(text: refused)
        }
        .frame(maxWidth: 560)
        .padding(Space.s3)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
        .onAppear { open() }
        .onDisappear {
            scope.cancel()
            mapEdits.orbit = nil
        }
        .onChange(of: point) { open() }
        .task(id: settled) {
            let at = settled
            let read = await offMain { guidedAltitude(Qgc.get(at.map { guidedAltitudePath($0) } ?? GUIDED_ALTITUDE)) }
            guard !Task.isCancelled else { return }
            height = read
            if target == nil { target = read?.current }
        }
    }

    private func open() {
        let circle = orbitOpened(point, defaults)
        opened = circle
        typed = nil
        height = nil
        target = nil
        settled = nil
        refused = nil
        mapEdits.orbit = circle
    }

    private func commit(_ circle: OrbitCircle) {
        guard let above = height?.targetMeters ?? height?.currentMeters else {
            refused = "Choose a radius and a height for the orbit."
            return
        }
        let centre = MapPoint(latitude: circle.centre.latitude, longitude: circle.centre.longitude)
        let choice = OrbitChoice(radiusMetres: circle.radiusMetres, clockwise: circle.clockwise, aboveHomeMetres: above)
        let action = action
        scope.launch {
            let answer = await offMain { send(action, centre, choice) }
            guard !Task.isCancelled else { return }
            if answer == nil { onDone() } else { refused = answer }
        }
    }
}

let SET_WAYPOINT_PATH = "vehicle.setCurrentMissionSequence"

func waypointTarget(_ sequence: Int) -> Int { max(sequence, 1) }

func setWaypointMessage(_ sequence: Int) -> String { "Adjust current waypoint to \(waypointTarget(sequence))" }

struct SetWaypointSheet: View {
    let sequence: Int
    let onDismiss: () -> Void
    @State private var refused: String?
    @State private var scope = ViewScope()
    @Environment(\.theme) private var theme

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s3) {
                Text("Set waypoint").font(.titleLarge)
                Text(setWaypointMessage(sequence)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                HoldOrCancel(label: "Set waypoint", onConfirm: {
                    let target = waypointTarget(sequence)
                    scope.launch {
                        let answer = await offMain { Qgc.refusalOf(SET_WAYPOINT_PATH, target) }
                        guard !Task.isCancelled else { return }
                        if answer == nil { onDismiss() } else { refused = answer }
                    }
                }, onCancel: onDismiss)
                RefusalText(text: refused)
            }
            .padding(.horizontal, Space.s5)
            .padding(.bottom, Space.s6)
        }
        .onChange(of: sequence) { refused = nil }
        .onDisappear { scope.cancel() }
    }
}

struct LoiterOffer: Equatable {
    var latitude: Double
    var longitude: Double
    var title: String
    var message: String
    var defaultRadius: Double
    var clockwise: Bool
}

func loiterOffer(_ view: JSON?) -> LoiterOffer? {
    guard let loiter = view?["loiter"].objectOrNil else { return nil }
    return LoiterOffer(
        latitude: loiter["latitude"].double ?? .nan,
        longitude: loiter["longitude"].double ?? .nan,
        title: loiter["title"].string,
        message: loiter["message"].string,
        defaultRadius: loiter["defaultRadius"].double(0),
        clockwise: loiter["clockwise"].bool(true)
    )
}

func mapClickUnits(_ view: JSON?) -> OrbitDefaults { orbitDefaults(view) }

func signedLoiterRadius(_ metres: Double, _ clockwise: Bool) -> Double { clockwise ? abs(metres) : -abs(metres) }

func loiterEditOpened(_ offer: LoiterOffer, _ units: OrbitDefaults) -> LoiterEdit {
    LoiterEdit(radiusMetres: abs(offer.defaultRadius) * units.metresPerUnit, clockwise: offer.clockwise, unit: units.unit, metresPerUnit: units.metresPerUnit)
}

func loiterRadiusField(_ typed: String?, _ edit: LoiterEdit) -> String {
    guard let typed, typed.isBlank || typedNumber(typed).map({ $0 * edit.metresPerUnit }) == edit.radiusMetres else {
        return loiterEditNumber(edit)
    }
    return typed
}

func loiterTyped(_ text: String, _ edit: LoiterEdit) -> LoiterEdit {
    typedNumber(text).flatMap { $0 > 0 ? LoiterEdit(radiusMetres: $0 * edit.metresPerUnit, clockwise: edit.clockwise, unit: edit.unit, metresPerUnit: edit.metresPerUnit) : nil } ?? edit
}

struct LoiterRadiusPanel: View {
    let offer: LoiterOffer
    let units: OrbitDefaults
    let onRefused: (String) -> Void
    let onDone: () -> Void
    @Environment(FlyMapEdits.self) private var mapEdits
    @Environment(FlyScreenState.self) private var flyScreen
    @State private var typed: String?
    @State private var scope = ViewScope()

    var body: some View {
        let opened = loiterEditOpened(offer, units)
        let edit = mapEdits.gotoLoiter ?? opened
        GuidedValuePanel(
            title: offer.title,
            sentence: offer.message,
            commitLabel: offer.title,
            commitEnabled: true,
            onCommit: {
                let sent = signedLoiterRadius(edit.radiusMetres, edit.clockwise)
                let offer = offer
                scope.launch {
                    let answer = await offMain {
                        Qgc.refusalOf("vehicle.guidedModeGotoLocation", ["latitude": offer.latitude, "longitude": offer.longitude] as [String: Any], sent)
                    }
                    guard !Task.isCancelled else { return }
                    if let answer { onRefused(answer) }
                    onDone()
                }
            },
            onCancel: onDone
        ) {
            RadiusField(
                label: ["Radius", edit.unit].filter { !$0.isBlank }.joined(separator: " "),
                text: Binding(
                    get: { loiterRadiusField(typed, edit) },
                    set: { text in
                        typed = text
                        mapEdits.gotoLoiter = loiterTyped(text, edit)
                    }
                )
            )
            Toggle("Clockwise", isOn: Binding(
                get: { edit.clockwise },
                set: { mapEdits.gotoLoiter = LoiterEdit(radiusMetres: edit.radiusMetres, clockwise: $0, unit: edit.unit, metresPerUnit: edit.metresPerUnit) }
            ))
        }
        .onAppear { if mapEdits.gotoLoiter == nil { mapEdits.gotoLoiter = opened } }
        .onDisappear {
            scope.cancel()
            let actions = flyScreen.flightActions
            let mapEdits = mapEdits
            Task { @MainActor in if actions.editingLoiter == nil { mapEdits.gotoLoiter = nil } }
        }
        .onChange(of: offer) {
            typed = nil
            mapEdits.gotoLoiter = loiterEditOpened(offer, units)
        }
    }
}

let STOP_ROI_PATH = "vehicle.stopGuidedModeROI"
let SET_ROI_PATH = "vehicle.guidedModeROI"

struct RoiSheet: View {
    let at: TrackPoint
    let onDismiss: () -> Void
    @State private var editing = false
    @State private var refused: String?
    @State private var scope = ViewScope()

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s3) {
                Text("ROI").font(.titleMedium)
                HStack(spacing: Space.s2) {
                    Button("Cancel ROI") { run(STOP_ROI_PATH, []) }.buttonStyle(.bordered)
                    Button("Edit position") { editing = true }.buttonStyle(.bordered)
                }
                RefusalText(text: refused)
                if editing {
                    EditPositionDialog(at: at, onDismiss: { editing = false }, title: "Edit ROI Position") { latitude, longitude in
                        editing = false
                        run(SET_ROI_PATH, [["latitude": latitude, "longitude": longitude] as [String: Any]])
                    }
                }
            }
            .padding(.horizontal, Space.s5)
            .padding(.bottom, Space.s6)
        }
        .onChange(of: at) {
            editing = false
            refused = nil
        }
        .onDisappear { scope.cancel() }
    }

    private func run(_ path: String, _ args: [Any]) {
        scope.launch {
            let answer = await offMain { Qgc.refusalOf(path, arguments: args) }
            guard !Task.isCancelled else { return }
            if answer == nil { onDismiss() } else { refused = answer }
        }
    }
}
