import Combine
import SwiftUI
import UniformTypeIdentifiers

private let PLAN_POLL_MS = 700
private let FAILURE_MESSAGE_MS = 2500
private let PANEL_MAX_FRACTION: CGFloat = 0.5
private let SIDE_PANEL_WIDTH: CGFloat = 380
private let SIDE_PANEL_MIN_WIDTH_DP: CGFloat = 840
let SUMMARY_MAX_FRACTION: CGFloat = 0.74
private let WAITING_FOR_QGC = "Waiting for QGroundControl"
private let FALLBACK_FENCE_DEGREES = 0.002
private let SURVEY_FIT_INSET = 0.8
private let ITEM_MARKER_SIZE: CGFloat = 40
private let PANEL_TOP = "panelTop"

private func pause(_ milliseconds: Int) async {
    try? await Task.sleep(for: .milliseconds(milliseconds))
}

func fenceWindow(_ visible: [TrackPoint], _ at: TrackPoint) -> (TrackPoint, TrackPoint) {
    visible.count == 4
        ? (visible[0], visible[2])
        : (TrackPoint(latitude: at.latitude + FALLBACK_FENCE_DEGREES, longitude: at.longitude - FALLBACK_FENCE_DEGREES),
           TrackPoint(latitude: at.latitude - FALLBACK_FENCE_DEGREES, longitude: at.longitude + FALLBACK_FENCE_DEGREES))
}

struct PlanUpload {
    let enabled: Bool
    let emphasised: Bool
    let label: String
    let shown: Bool
    let onClick: () -> Void

    var done: Bool { label == UPLOADED }
}

struct PlanBar {
    let upload: PlanUpload
    let stats: [PlanStat]
}

private struct PlanRead: Sendable {
    let planRead: Bool
    let all: [MissionItem]
    let items: [MissionItem]
    let itemCount: Int
    let shape: [String]
    let link: Bool
    let fences: [FencePolygon]
    let rally: [RallyPoint]
    let gcs: TrackPoint?
    let circles: [FenceCircle]
    let firmware: FirmwareFence?
    let breach: BreachReturn?
    let surveys: [Survey]
    let landings: [LandingPattern]
    let stats: [Int: SurveyStats]
    let drawn: Bool

    static func read() -> PlanRead {
        let plan = PlanBridge.rawItems()
        if plan != nil { MapBridge.markReachable() }
        let all = allMissionItems(plan)
        let items = all.filter(\.placed)
        let fenceView = FenceBridge.read()
        let fences = fencePolygons(fenceView)
        let rally = rallyPoints(fenceView)
        let circles = fenceCircles(fenceView)
        let surveys = SurveyBridge.surveysFrom(plan)
        return PlanRead(
            planRead: plan != nil,
            all: all,
            items: items,
            itemCount: planItemCount(plan),
            shape: planShape(plan),
            link: linksStartToHome(plan),
            fences: fences,
            rally: rally,
            gcs: operatorPoint(OperatorBridge.read()),
            circles: circles,
            firmware: firmwareFence(fenceView),
            breach: breachReturn(fenceView),
            surveys: surveys,
            landings: landingPatterns(all),
            stats: surveyStatsFor(all),
            drawn: planIsDrawn(items, surveys, fences, circles, rally)
        )
    }
}

struct PlanMapContent: View {
    let mapStyle: String
    var onCentre: ((Double, Double) -> Void)? = nil
    var itemPanel: ((Int, TrackPoint?, @escaping () -> Void) -> AnyView)? = nil
    var header: ((PlanBar) -> AnyView)? = nil
    var primary: ((PlanUpload) -> AnyView)? = nil
    var routeSettings: (() -> AnyView)? = nil
    var fitKey: Int = 0
    var onTemplates: (() -> Void)? = nil

    @Environment(\.theme) private var theme
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.LocalRootSize) private var root

    @State private var follow = false
    @State private var shownStyle: String?
    @State private var fitRequest = 0
    @State private var edits = 0
    @State private var fitOnly: [TrackPoint]?
    @State private var positioning: (MapHit, TrackPoint)?
    @State private var loadArmed = false
    @State private var items: [MissionItem] = []
    @State private var allItems: [MissionItem] = []
    @State private var itemCount = 0
    @State private var shape: [String] = []
    @State private var linkStartToHome = false
    @State private var fences: [FencePolygon] = []
    @State private var chosenCircles: Set<String> = []
    @State private var radiusFor: ShapeTarget?
    @State private var rally: [RallyPoint] = []
    @State private var gcsOperator: TrackPoint?
    @State private var circles: [FenceCircle] = []
    @State private var firmware: FirmwareFence?
    @State private var breach: BreachReturn?
    @State private var editingBreach = false
    @State private var surveyList: [Survey] = []
    @State private var landingList: [LandingPattern] = []
    @State private var surveyStatsMap: [Int: SurveyStats] = [:]
    @State private var selected: MapHit?
    @State private var layer = PlanLayer.Mission
    @State private var busy: String?
    @State private var centre: TrackPoint?
    @State private var zoom = 0.0
    @State private var panelHeight: CGFloat = 0
    @State private var scrollContent: CGFloat = 0
    @State private var pinnedHeight: CGFloat = 0
    @State private var headerHeight: CGFloat = 0
    @State private var uploadAsk: UploadGate?
    @State private var patternWanted: [MissionKind] = []
    @State private var visible: [TrackPoint] = []
    @State private var importInto: ShapeTarget?
    @State private var importing = false
    @State private var tracing: (ShapeTarget, [TrackPoint])?
    @State private var firstRead = true
    @State private var listOpen = false
    @State private var centreRequest = 0
    @State private var centreOn: TrackPoint?
    @State private var centredOnEntry = false
    @State private var gridAngle: Float?
    @State private var surveyAlt = ""
    @State private var surveyUnit = "m"
    @State private var circleRadiusTyped = ""
    @State private var rallyLatitudeTyped = ""
    @State private var rallyLongitudeTyped = ""
    @State private var rallyAltitudeTyped = ""

    @MapPath("view.missionKinds") private var kindsView
    @MapPath("view.plan") private var planStatus
    @MapPath(VEHICLES_VIEW) private var vehiclesJson
    @MapPath("view.missionSummary") private var missionSummaryView
    @MapPath(TERRAIN_VIEW) private var terrainView
    @MapPath(ELEVATION_PROVIDER) private var elevationProviderJson
    @MapPath("\(SHOW_MISSION_ITEM_STATUS).rawValue") private var missionStatusJson

    private var insertable: [MissionKind] { missionKinds(kindsView) }
    private var planHasItems: Bool { planStatus?["containsItems"].bool == true }
    private var globalFrame: Int? { planStatus.flatMap { $0.has("globalAltitudeFrame") ? $0["globalAltitudeFrame"].int(0) : nil } }
    private var planOffline: Bool { planStatus?["offline"].bool == true }
    private var planDirty: Bool { planStatus?["dirty"].bool == true }
    private var planSyncing: Bool { planStatus?["sync"]["state"].string == "busy" }
    private var canUndo: Bool { planStatus?["canUndo"].bool == true }
    private var homeSet: Bool { planStatus.flatMap { $0["templates"].object != nil ? $0["templates"]["homeSet"].bool : nil } != false }
    private var flown: VehicleChoice? { vehicleChoices(vehiclesJson).active }
    private var latitude: Double { flown?.latitude ?? .nan }
    private var longitude: Double { flown?.longitude ?? .nan }
    private var missionStatusShown: Bool { missionItemStatusShown(missionStatusJson) }
    private var selectedSequence: Int? { selectionSequence(selected, allItems) }
    private var entryPoint: TrackPoint? { isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : gcsOperator }

    private func addable(_ kind: String) -> Bool { kindAllows(insertable, kind) || !homeSet }

    private func addablePattern(_ kind: MissionKind) -> Bool { kind.enabled || !homeSet }

    private func placeAt() -> TrackPoint? {
        if let centre, isPlottable(centre.latitude, centre.longitude) { return centre }
        return isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : nil
    }

    var body: some View {
        GeometryReader { geometry in
            let sidePanel = root.width >= SIDE_PANEL_MIN_WIDTH_DP
            let mapStart: CGFloat = sidePanel ? SIDE_PANEL_WIDTH : 0
            let railStart = mapStart + 8
            let bottomPanel = sidePanel ? 0 : panelHeight
            ZStack(alignment: .topLeading) {
                map(sidePanel: sidePanel)
                if let header {
                    header(PlanBar(upload: upload, stats: planStats(itemCount, allItems, missionSummaryView)))
                        .frame(maxWidth: .infinity, alignment: .topLeading)
                        .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { headerHeight = $0 }
                        .padding(.leading, mapStart)
                }
                notices(left: railStart + RAIL_WIDTH + 8, width: geometry.size.width)
                rail
                    .padding(.leading, railStart)
                    .padding(.top, headerHeight + 8)
                    .padding(.bottom, bottomPanel + 8)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                if let primary {
                    primary(upload)
                        .padding(.trailing, 16)
                        .padding(.bottom, bottomPanel + 16)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                }
                dialogs
                controls(sidePanel: sidePanel)
                if listOpen {
                    AircastSheet(onDismissRequest: { listOpen = false }) {
                        let rows = itemRows(allItems, surveyStatsMap)
                        ItemListHeading(rows: rows, summary: missionSummaryText(missionSummaryView))
                            .padding(.horizontal, 24)
                            .padding(.vertical, 8)
                        ScrollView {
                            LazyVStack(spacing: 0) {
                                ForEach(rows) { row in
                                    ItemRowView(row: row, selected: selectedWaypointIndex == row.index) {
                                        pickRow(row)
                                        listOpen = false
                                    }
                                }
                            }
                            .padding(.bottom, 24)
                        }
                    }
                }
            }
        }
        .background(theme.colors.surface)
        .fileImporter(isPresented: $importing, allowedContentTypes: [.item], allowsMultipleSelection: true) { result in
            let target = importInto
            importInto = nil
            guard let target, case .success(let urls) = result, !urls.isEmpty else { return }
            Task { @MainActor in
                if let message = await offMain({ importShapeFiles(urls, target) }) { say(message) }
            }
        }
        .task(id: scenePhase != .background) {
            guard scenePhase != .background else { return }
            while !Task.isCancelled {
                await refresh()
                await pause(PLAN_POLL_MS)
            }
        }
        .onChange(of: mapStyle, initial: true) { shownStyle = mapStyle }
        .onReceive(PlanFocus.requests.receive(on: DispatchQueue.main)) { index in
            guard let index else { return }
            selected = .Waypoint(index: index)
            PlanFocus.requests.value = nil
        }
        .onChange(of: selected) { if let next = layerOf(selected) { layer = next } }
        .onChange(of: entryPoint != nil, initial: true) {
            if centersOnVehicleAtEntry(centredOnEntry, entryPoint != nil, fitRequest) {
                centredOnEntry = true
                centreOn = entryPoint
                centreRequest += 1
            }
        }
        .onChange(of: fitKey) { if fitKey != 0 { firstRead = true } }
        .onChange(of: SelectionKey(selected: selected, sequence: selectedSequence), initial: true) {
            guard let sequence = selectedSequence ?? (selected == nil ? 0 : nil) else { return }
            offMain { PlanBridge.selectSequence(sequence) }
        }
        .task(id: gridIndex) {
            gridAngle = nil
            guard let index = gridIndex else { return }
            gridAngle = await offMain { gridAngleShown(SurveyBridge.gridAngle(index)) }
        }
        .task(id: surveyHitItem) {
            surveyAlt = ""
            surveyUnit = "m"
            guard let item = surveyHitItem else { return }
            let shown = await offMain { SurveyBridge.altitude(item) }
            let unit = await offMain { SurveyBridge.altitudeUnits(item) }
            surveyAlt = altitudeFieldText(shown, SURFACE_DISTANCE_DECIMALS)
            surveyUnit = unit.ifBlank("m")
        }
        .onChange(of: selectedCircle.map { "\($0.index):\($0.radius)" }, initial: true) {
            circleRadiusTyped = selectedCircle.map { trimmedRadius($0.radius) } ?? ""
        }
        .onChange(of: selectedRally.map { "\($0.index):\($0.latitude)" }, initial: true) {
            rallyLatitudeTyped = selectedRally.map { String($0.latitude) } ?? ""
        }
        .onChange(of: selectedRally.map { "\($0.index):\($0.longitude)" }, initial: true) {
            rallyLongitudeTyped = selectedRally.map { String($0.longitude) } ?? ""
        }
        .onChange(of: selectedRally.map { "\($0.index):\($0.altitude)" }, initial: true) {
            rallyAltitudeTyped = selectedRally.map { altitudeFieldText($0.altitude, RALLY_ALTITUDE_DECIMALS) } ?? ""
        }
    }

    private var upload: PlanUpload {
        PlanUpload(enabled: uploadEnabled, emphasised: uploadEnabled && !uploadBlocked, label: uploadText, shown: !planOffline, onClick: startUpload)
    }

    private var uploadText: String { uploadLabel(planOffline, planSyncing, planDirty, planHasItems) }
    private var uploadEnabled: Bool { planHasItems && !planOffline && !planSyncing }
    private var uploadBlocked: Bool { syncRefusal(vehicleSyncState(planOffline, planSyncing), "upload to") != nil }

    private var selectedWaypointIndex: Int? {
        if case .Waypoint(let index) = selected { return index }
        return nil
    }

    private var selectedWaypoint: MissionItem? {
        selectedWaypointIndex.flatMap { index in allItems.first { $0.index == index } }
    }

    private var selectedCircle: FenceCircle? {
        let index: Int? = switch selected {
        case .Circle(let index), .CircleCentre(let index): index
        default: nil
        }
        return index.flatMap { index in circles.first { $0.index == index } }
    }

    private var selectedRally: RallyPoint? {
        guard case .Rally(let index) = selected else { return nil }
        return rally.first { $0.index == index }
    }

    private var gridIndex: Int? {
        selectedSurvey(selected, surveyList).flatMap { $0.kind == KIND_SURVEY ? $0.index : nil }
    }

    private var surveyHitItem: Int? {
        if case .SurveyVertex(let item, _) = selected { return item }
        return nil
    }

    @ViewBuilder
    private func map(sidePanel: Bool) -> some View {
        let circled = liveCircles(chosenCircles, fences, surveyList)
        let collidingPatterns = collidingItems(terrainView)
        let collidingSimple = collidingItems(terrainView, "collidingSimpleItems")
        let owner = ownerOf(selected)
        VehicleMap(
            mapStyle: shownStyle ?? mapStyle,
            follow: follow,
            missionItems: items.map { item in var shown = item; shown.terrainCollision = collidingSimple.contains(item.index); return shown },
            linkStartToHome: linkStartToHome,
            fencePolygons: fences.map { fence in var shown = fence; if owner != "p\(fence.index)" { shown.editable = nil }; return shown },
            fenceCircles: circles,
            rallyPoints: rally,
            operator: gcsOperator,
            surveys: surveyList.map { survey in
                var shown = survey
                if owner != "m\(survey.index)" { shown.editable = nil }
                shown.collides = collidingPatterns.contains(survey.index)
                return shown
            },
            landings: landingList.map { landing in var shown = landing; shown.collides = collidingPatterns.contains(landing.index); return shown },
            editable: true,
            selectedWaypoint: selectedWaypointIndex,
            circledShapes: circled,
            firmwareFence: firmware,
            onAdd: { lat, lon in mapAdd(lat, lon) },
            onBlankTap: { lat, lon in blankTap(lat, lon) },
            onMove: { hit, lat, lon in
                let generation = moveGeneration()
                let surveys = surveyList, points = rally, polygons = fences, listed = allItems, rings = circles
                onBridge { writeDragStep(generation, hit, lat, lon, surveys, points, polygons, listed, rings) }
            },
            onWaypointSelected: { hit in mapSelected(hit) },
            onMoved: { hit, lat, lon in
                let surveys = surveyList, points = rally, polygons = fences, listed = allItems, rings = circles
                onBridge(done: movedText(hit, allItems)) { writeMove(hit, lat, lon, surveys, points, polygons, listed, rings) }
            },
            canDrag: { hit in dragAllowed(hit, selected, layer) },
            onCentreChanged: { at, level in
                centre = at
                zoom = level
                onCentre?(at.latitude, at.longitude)
            },
            onViewChanged: { visible = $0 },
            bottomInsetPx: sidePanel ? 0 : panelHeight,
            topInsetPx: headerHeight,
            leftInsetPx: (sidePanel ? SIDE_PANEL_WIDTH : 0) + RAIL_WIDTH + 8,
            fitRequest: fitRequest,
            fitOnly: fitOnly,
            onFitFailed: { onBridge("Fitting the plan") { false } },
            centreRequest: centreRequest,
            centreOn: centreOn,
            breachReturn: breach?.point,
            tracePoints: tracing?.1 ?? [],
            traceLine: tracing?.0.line == true,
            collisionLegs: collisionLegs(terrainView)
        )
    }

    private func mapAdd(_ lat: Double, _ lon: Double) {
        if let (target, points) = tracing {
            tracing = (target, points + [TrackPoint(latitude: lat, longitude: lon)])
            return
        }
        switch layer {
        case .Mission:
            addMissionItem(KIND_WAYPOINT, "Adding a waypoint", TrackPoint(latitude: lat, longitude: lon), insertAfter(selected, allItems))
        case .Rally:
            if planSupport(planStatus).rally { addRallyAt(lat, lon) }
        case .Fence:
            break
        }
    }

    private func blankTap(_ lat: Double, _ lon: Double) {
        let at = TrackPoint(latitude: lat, longitude: lon)
        if let (target, points) = tracing {
            tracing = (target, points + [at])
        } else if selected != nil && selectedWaypointIndex == nil {
            selected = nil
        } else if layer == .Mission {
            addMissionItem(KIND_WAYPOINT, "Adding a waypoint", at, insertAfter(selected, allItems))
        } else if layer == .Rally && planSupport(planStatus).rally {
            addRallyAt(lat, lon)
        } else {
            selected = nil
        }
    }

    private func mapSelected(_ hit: MapHit?) {
        if let hit, actsOnTap(hit), !dragAllowed(hit, selected, layer) { return }
        switch hit {
        case .Midpoint(let path, let invokable, let segment):
            if path == MISSION_SPLIT_PATH {
                addMissionItem(KIND_WAYPOINT, "Adding a waypoint", legSplit(allItems.filter(\.placed), segment), segment)
            } else {
                onBridge("Adding a corner") { invokeOk("\(path).\(invokable)", segment) }
            }
        case .BreachReturn:
            editingBreach = true
        case .LoiterRotation(let index):
            if let item = allItems.first(where: { $0.index == index }) {
                let radius = item.loiterRadius
                onBridge(done: movedText(.LoiterRotation(index: index), allItems)) { PlanBridge.setLoiterRadius(index, -radius) }
            }
        case .LoiterRadius(let index):
            selected = .Waypoint(index: index)
        case .CircleRadius(let index):
            selected = .Circle(index: index)
        default:
            selected = hit
        }
    }

    private func notices(left: CGFloat, width: CGFloat) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            if let message = busy ?? (MapBridge.bridgeReady ? nil : WAITING_FOR_QGC) {
                Text(message)
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurface)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                    .background(theme.colors.surfaceContainer.opacity(0.94), in: RoundedRectangle(cornerRadius: Corner.extraLarge))
            }
            if let centre {
                ScaleBarView(latitude: centre.latitude, zoom: zoom)
                    .padding(.leading, 4)
                    .padding(.top, 8)
            }
        }
        .frame(maxWidth: max(width - left - 8, 0) * SUMMARY_MAX_FRACTION, alignment: .leading)
        .padding(.leading, left)
        .padding(.top, headerHeight + 8)
    }

    private var rail: some View {
        let support = planSupport(planStatus)
        return PlanRail {
            ForEach(PlanLayer.allCases, id: \.self) { option in
                RailButton(icon: option.icon, label: option.label, onClick: {
                    layer = option
                    if layerOf(selected) != option { selected = nil }
                }, chosen: layer == option)
            }
            RailDivider()
            switch layer {
            case .Mission:
                RailButton(icon: .add, label: "Waypoint", onClick: {
                    addMissionItem(KIND_WAYPOINT, "Adding a waypoint", placeAt(), insertAfter(selected, allItems))
                }, enabled: placeAt() != nil)
                RailButton(icon: .planGrid, label: "Pattern", onClick: { patternWanted = scanPatterns(insertable) }, enabled: !scanPatterns(insertable).isEmpty)
                if kindOffered(insertable, KIND_TAKEOFF) && takeoffMissing(items) {
                    RailButton(icon: .flightTakeoff, label: "Takeoff", onClick: {
                        addMissionItem(KIND_TAKEOFF, "Adding a takeoff", placeAt(), BEFORE_THE_REST)
                    }, enabled: addable(KIND_TAKEOFF))
                }
                RailButton(icon: .flightLand, label: kindLabel(insertable, KIND_LAND), onClick: {
                    addMissionItem(KIND_LAND, "Adding a landing", placeAt(), insertAfter(selected, allItems))
                }, enabled: itemCount > 0)
            case .Fence:
                if !support.fenceRefused {
                    RailButton(icon: .planPolygon, label: "Polygon", onClick: addPolygonFence, enabled: support.fence)
                    RailButton(icon: .planCircle, label: "Circle", onClick: addCircleFence, enabled: support.fence)
                    RailButton(icon: .home, label: "Breach", onClick: breachPressed, enabled: support.fence, chosen: breach != nil)
                }
            case .Rally:
                RailButton(icon: .add, label: "Add point", onClick: {
                    if let at = placeAt() { addRallyAt(at.latitude, at.longitude) }
                }, enabled: support.rally && placeAt() != nil)
            }
            RailDivider()
            RailButton(icon: .undo, label: "Undo", onClick: { onBridge { invokeOk(PLAN_UNDO) } }, enabled: canUndo)
            CenterMenu(
                launch: allItems.first { $0.sequence == 0 }.map { TrackPoint(latitude: $0.latitude, longitude: $0.longitude) },
                myLocation: gcsOperator,
                onCentre: { point in
                    follow = false
                    centreOn = point
                    centreRequest += 1
                },
                missionPoints: missionFitPoints(allItems),
                onFit: { points in
                    follow = false
                    fitOnly = points
                    fitRequest += 1
                },
                following: follow,
                onFollow: { follow.toggle() },
                anchor: { AnyView(RailButtonFace(icon: .planMyLocation, label: "Centre", chosen: follow)) }
            )
            MapTypeMenu { shownStyle = $0 }
        }
    }

    private func addPolygonFence() {
        let at = placeAt()
        let next = fences.count
        let shown = visible
        onBridge("Adding fence", then: { selected = .FenceVertex(polygon: next, vertex: 0) }) {
            at.map { fenceWindow(shown, $0) }.map { FenceBridge.addInclusionPolygon($0.0, $0.1) } ?? false
        }
    }

    private func addCircleFence() {
        let at = placeAt()
        let next = circles.count
        let shown = visible
        onBridge("Adding circle", then: { selected = .Circle(index: next) }) {
            at.map { fenceWindow(shown, $0) }.map { FenceBridge.addInclusionCircle($0.0, $0.1) } ?? false
        }
    }

    private func breachPressed() {
        guard breach == nil else { return editingBreach = true }
        let at = placeAt()
        onBridge("Adding breach return point") { at.map { FenceBridge.setBreachReturn($0) } ?? false }
    }

    private func addRallyAt(_ latitude: Double, _ longitude: Double) {
        let next = rally.count
        onBridge("Adding rally", then: { selected = .Rally(index: next) }) { FenceBridge.addRallyPoint(latitude, longitude) }
    }

    @ViewBuilder
    private func controls(sidePanel: Bool) -> some View {
        let chosen = selected
        let chosenItem = selectedWaypoint
        let profile = terrainProfile(terrainView)
        let corner: CGFloat = sidePanel ? 0 : 28
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 0) {
                if !sidePanel {
                    Capsule()
                        .fill(theme.colors.onSurfaceVariant.opacity(0.4))
                        .frame(width: 32, height: 4)
                        .padding(.vertical, 10)
                        .frame(maxWidth: .infinity)
                }
                if layer == .Mission && itemCount > 0 && (layerOf(chosen) == nil || layerOf(chosen) == .Mission) {
                    WaypointStripBar(
                        rows: itemRows(allItems, surveyStatsMap),
                        selected: chosenItem?.index,
                        onPick: focusItem,
                        onList: sidePanel ? nil : { listOpen = true },
                        profileShown: profileShown(layer, profile) ? missionStatusShown : nil,
                        onProfile: {
                            let shown = missionStatusShown
                            onBridge { setOk(SHOW_MISSION_ITEM_STATUS, !shown) }
                        }
                    )
                }
                if let item = chosenItem {
                    let detail = [itemPlace(item, allItems), sheetDetail(item, surveyStatsMap[item.index]).isBlank ? nil : sheetDetail(item, surveyStatsMap[item.index])]
                        .compactMap { $0 }
                        .joined(separator: " \u{00b7} ")
                    SelectionHeader(
                        title: "\(sentenceCase(item.command.ifBlank("Item"))) \(sequenceLabel(item))",
                        detail: detail.isBlank ? nil : detail,
                        onDone: { selected = nil }
                    )
                } else if let chosen {
                    SelectionHeader(title: selectionTitle(chosen, allItems), detail: selectionText(chosen, allItems, circles, fences), onDone: { selected = nil })
                }
            }
            .fixedSize(horizontal: false, vertical: true)
            .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { pinnedHeight = $0 }
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        Color.clear.frame(height: 0).id(PANEL_TOP)
                        panelBody(chosen, chosenItem, profile, sidePanel: sidePanel)
                    }
                    .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { scrollContent = $0 }
                }
                .scrollBounceBehavior(.basedOnSize)
                .frame(height: sidePanel ? nil : min(scrollContent, max(root.height * PANEL_MAX_FRACTION - pinnedHeight, 0)))
                .frame(maxHeight: sidePanel ? .infinity : nil)
                .onChange(of: selected) { proxy.scrollTo(PANEL_TOP, anchor: .top) }
            }
        }
        .padding(.horizontal, 12)
        .padding(.top, sidePanel ? 12 : 0)
        .frame(width: sidePanel ? SIDE_PANEL_WIDTH : nil)
        .frame(maxWidth: sidePanel ? nil : .infinity, alignment: .top)
        .background(theme.colors.surfaceContainerLow, in: UnevenRoundedRectangle(topLeadingRadius: corner, topTrailingRadius: corner))
        .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { panelHeight = $0 }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: sidePanel ? .topLeading : .bottom)
    }

    @ViewBuilder
    private func panelBody(_ chosen: MapHit?, _ chosenItem: MissionItem?, _ profile: TerrainProfile, sidePanel: Bool) -> some View {
        if chosen == nil && layer == .Mission && missionStatusShown && profileShown(layer, profile) && itemCount > 0 {
            TerrainProfileView(profile: profile, notice: elevationProviderJson?["value"].string ?? "", selectedSequence: selectedSequence) { sequence in
                if let item = allItems.first(where: { $0.sequence == sequence }) { selected = .Waypoint(index: item.index) }
            }
        }
        if let item = chosenItem {
            let tiles = surveyTiles(item, surveyStatsMap[item.index])
            if !tiles.isEmpty {
                HStack(spacing: 8) {
                    ForEach(Array(tiles.enumerated()), id: \.offset) { _, tile in StatTile(label: tile.0, value: tile.1) }
                }
                .padding([.leading, .trailing, .bottom], 12)
            }
            WaypointSettings(item: item, globalFrame: globalFrame) { label, work in onBridge(label) { work() } }
                .id(item.index)
            if let note = addingAfterText(chosen, allItems) {
                PaletteNote(text: note)
            }
        } else if chosen != nil {
            EmptyView()
        } else if layer == .Mission && itemCount == 0 {
            EmptyMissionStrip(homeSet: homeSet, onTemplates: onTemplates, onDownload: planOffline ? nil : { requestDownload() })
        } else if layer == .Mission {
            routeSettings?()
        } else if layer == .Rally {
            rallyList
        } else {
            fenceList
        }
        if sidePanel && layer == .Mission && worthListing(allItems) {
            let rows = itemRows(allItems, surveyStatsMap)
            ItemListHeading(rows: rows, summary: missionSummaryText(missionSummaryView))
                .padding(.horizontal, 12)
                .padding(.vertical, 4)
            ForEach(rows) { row in
                ItemRowView(row: row, selected: selectedWaypointIndex == row.index) { pickRow(row) }
            }
        }
        traceRow
        selectionTools
        if let item = chosenItem, let itemPanel {
            itemPanel(item.index, item.placed ? TrackPoint(latitude: item.latitude, longitude: item.longitude) : nil) { removeItem(item) }
        }
    }

    @ViewBuilder
    private var rallyList: some View {
        let support = planSupport(planStatus)
        if support.rallyRefused {
            PaletteNote(text: RALLY_NOT_SUPPORTED)
        } else if rally.isEmpty {
            PaletteNote(text: NO_RALLY_POINTS)
        }
        if !rally.isEmpty { FenceHeading(text: "Rally points") }
        ForEach(rallyRows(rally), id: \.index) { row in
            FenceListRow(row: row, chosen: false, onSelect: { selected = .Rally(index: row.index) }) {
                let count = rally.count
                onBridge("Removing \(row.title.lowercased())", then: { selected = rallyAfterRemove(row.index, count) }) { FenceBridge.removeRallyPoint(row.index) }
            }
        }
    }

    @ViewBuilder
    private var fenceList: some View {
        let support = planSupport(planStatus)
        if support.fenceRefused {
            PaletteNote(text: GEOFENCE_NOT_SUPPORTED)
        } else if fences.isEmpty && circles.isEmpty {
            PaletteNote(text: NO_GEOFENCE)
        }
        let listed = support.fenceRefused ? [] : fenceRows(fences, circles)
        ForEach(Array(listed.enumerated()), id: \.offset) { at, row in
            if let heading = fenceHeading(row, at > 0 ? listed[at - 1] : nil) {
                FenceHeading(text: heading)
            }
            FenceListRow(
                row: row,
                chosen: false,
                onSelect: { selected = fenceRowHit(row) },
                onInclusion: row.inclusion == nil ? nil : { keep in onBridge("Changing \(row.title.lowercased())") { FenceBridge.setPolygonInclusion(row.index, keep) } }
            ) {
                onBridge("Removing \(row.title.lowercased())", then: { selected = fenceSelectionAfterRemove(row, selected) }) {
                    row.circle ? FenceBridge.deleteCircle(row.index) : FenceBridge.deletePolygon(row.index)
                }
            }
        }
    }

    @ViewBuilder
    private var traceRow: some View {
        if let (target, points) = tracing {
            HStack {
                Text(traceCaption(points.count, target.minimum)).font(.labelSmall)
                Button("Done") {
                    tracing = nil
                    onBridge("Tracing shape") { replaceShape(target, points) }
                }
                .buttonStyle(.borderless)
                .disabled(points.count < target.minimum)
                Button("Cancel") { tracing = nil }
                    .buttonStyle(.borderless)
            }
        }
    }

    @ViewBuilder
    private var selectionTools: some View {
        let survey = selectedSurvey(selected, surveyList)
        let shapeFence: Int? = switch selected {
        case .ShapeCentre(true, let owner), .ShapeRadius(true, let owner): owner
        default: nil
        }
        let fenceHit: (polygon: Int, vertex: Int)? = if case .FenceVertex(let polygon, let vertex) = selected { (polygon, vertex) } else { nil }
        let surveyHit: (item: Int, vertex: Int)? = if case .SurveyVertex(let item, let vertex) = selected { (item, vertex) } else { nil }
        let rallyHit: Int? = if case .Rally(let index) = selected { index } else { nil }
        let circle = selectedCircle
        if survey != nil || fenceHit != nil || shapeFence != nil || rallyHit != nil || circle != nil || selectedLanding(selected, landingList) != nil {
            PlanFlowRow(spacing: 4, lineSpacing: 4) {
                if let fence = selectedFence(selected, fences, circles), let flip = fence.flip {
                    Button(fence.keepsIn ? "Make keep-out" : "Make keep-in") {
                        onBridge(fence.keepsIn ? "Making it keep-out" : "Making it keep-in") { flip() }
                    }
                    .buttonStyle(.borderless)
                }

                if let detail = fenceDetail(selected, fences, circles) {
                    PaletteNote(text: detail)
                }

                if let note = landingText(selectedLanding(selected, landingList)) {
                    PaletteNote(text: note)
                }

                if let note = survey.flatMap({ cameraText(surveyStatsMap[$0.index]) }) {
                    PaletteNote(text: note)
                }

                if let note = layersText(survey) {
                    PaletteNote(text: note)
                }

                if let grid = survey, grid.kind == KIND_SURVEY, let shown = gridAngle {
                    VStack(alignment: .leading, spacing: 0) {
                        Text("Angle \(Int(shown))°").font(.labelSmall)
                        Slider(
                            value: Binding(get: { Double(gridAngle ?? shown) }, set: { gridAngle = Float(($0 + 0.5).rounded(.down)) }),
                            in: 0...Double(GRID_ANGLE_MAX)
                        ) { editing in
                            guard !editing, let chosen = gridAngle else { return }
                            onBridge("Setting the grid angle") { SurveyBridge.setGridAngle(grid.index, Double(chosen)) }
                        }
                    }
                    .frame(width: 220)
                }

                if let area = survey, visible.count == area.area.count {
                    Button("Size to view") {
                        let corners = insetRing(visible, SURVEY_FIT_INSET)
                        onBridge("Sizing the area", done: "Area sized to the view") { fitSurveyArea(area, corners) }
                    }
                    .buttonStyle(.borderless)
                }

                if let hit = fenceHit.map({ MapHit.FenceVertex(polygon: $0.polygon, vertex: $0.vertex) }) ?? surveyHit.map({ MapHit.SurveyVertex(item: $0.item, vertex: $0.vertex) }),
                   let at = cornerPosition(hit, fences, surveyList) {
                    Button("Edit position") { positioning = (hit, at) }
                        .buttonStyle(.borderless)
                }

                if let target = shapeTarget(fenceHit?.polygon ?? shapeFence, surveyShapeOwner(survey, surveyHit != nil)) {
                    shapeTools(target, circled: liveCircles(chosenCircles, fences, surveyList))
                }

                if let hit = fenceHit {
                    if cornerRemovable(fences.first { $0.index == hit.polygon }) {
                        Button("Remove vertex") {
                            onBridge("Removing corner") { FenceBridge.removeVertex(hit.polygon, hit.vertex) }
                            selected = nil
                        }
                        .buttonStyle(.borderless)
                    }
                    Button("Delete fence") {
                        onBridge { FenceBridge.deletePolygon(hit.polygon) }
                        selected = nil
                    }
                    .buttonStyle(.borderless)
                }

                if let hit = surveyHit {
                    if let shape = survey, shape.editable?.canRemoveVertex == true {
                        Button("Remove vertex") {
                            onBridge("Removing corner") { SurveyBridge.removeVertex(shape, hit.vertex) }
                            selected = nil
                        }
                        .buttonStyle(.borderless)
                    }
                    PlanTextField(label: "Above surface \(surveyUnit)", text: $surveyAlt, width: 150) {
                        if let shown = parsedSurfaceDistance(surveyAlt, metresPerUnit(surveyUnit)) {
                            onBridge("Setting survey altitude") { SurveyBridge.setAltitude(hit.item, shown) }
                        } else {
                            say("Not an altitude")
                        }
                    }
                }

                if let item = surveyHit?.item ?? shapeCentreSurvey {
                    Button("Delete \(patternName(item, allItems))") {
                        onRefusal { PlanBridge.removeItemRefusal(item) }
                        selected = nil
                    }
                    .buttonStyle(.borderless)
                }

                if let circle {
                    circleTools(circle)
                }

                if let index = rallyHit {
                    rallyTools(index)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var shapeCentreSurvey: Int? {
        if case .ShapeCentre(false, let owner) = selected { return owner }
        return nil
    }

    private func surveyShapeOwner(_ survey: Survey?, _ vertexHit: Bool) -> Survey? {
        let centreOrRadius: Bool = switch selected {
        case .ShapeCentre, .ShapeRadius: true
        default: false
        }
        return vertexHit || centreOrRadius ? survey : nil
    }

    @ViewBuilder
    private func shapeTools(_ target: ShapeTarget, circled: Set<String>) -> some View {
        if let shape = shapeEditable(target, fences, surveyList) {
            Text(shapeCaption(shape, circled.contains(target.path))).font(.labelSmall)
        }
        if target.line {
            Button("Line") {
                let line = defaultLine(visible)
                onBridge("Drawing line") { replaceShape(target, line) }
            }
            .buttonStyle(.borderless)
            .disabled(visible.count != 4)
        } else {
            Button("Rectangle") {
                chosenCircles.remove(target.path)
                let rectangle = defaultRectangle(visible)
                onBridge("Drawing rectangle") { replaceShape(target, rectangle) }
            }
            .buttonStyle(.borderless)
            .disabled(visible.count != 4)
            Button("Circle") {
                chosenCircles.insert(target.path)
                let ring = defaultCircle(visible)
                onBridge("Drawing circle") { replaceShape(target, ring) }
            }
            .buttonStyle(.borderless)
            .disabled(visible.count != 4)
            if circled.contains(target.path) {
                Button("Set radius\u{2026}") { radiusFor = target }
                    .buttonStyle(.borderless)
                if let centreHit = shapeCentreHit(target, fences, surveyList) {
                    Button("Edit position\u{2026}") { positioning = centreHit }
                        .buttonStyle(.borderless)
                }
            }
        }
        Button("Trace") { tracing = (target, []) }
            .buttonStyle(.borderless)
        Button("Import\u{2026}") {
            chosenCircles.remove(target.path)
            importInto = target
            importing = true
        }
        .buttonStyle(.borderless)
    }

    @ViewBuilder
    private func circleTools(_ circle: FenceCircle) -> some View {
        let bigger = grownRadius(circle)
        let smaller = shrunkRadius(circle)
        PlanTextField(label: "Radius", text: $circleRadiusTyped, suffix: circle.radiusUnits, width: 110) {
            if let wanted = typedRadius(circleRadiusTyped, circle) {
                onBridge { FenceBridge.setCircleRadius(circle.index, wanted) }
            } else {
                say("Not a radius this fence accepts")
            }
        }
        Button("Bigger") { bigger.map { wanted in onBridge { FenceBridge.setCircleRadius(circle.index, wanted) } } }
            .buttonStyle(.borderless)
            .disabled(bigger == nil)
        Button("Smaller") { smaller.map { wanted in onBridge { FenceBridge.setCircleRadius(circle.index, wanted) } } }
            .buttonStyle(.borderless)
            .disabled(smaller == nil)
        Button("Delete circle") {
            onBridge { FenceBridge.deleteCircle(circle.index) }
            selected = nil
        }
        .buttonStyle(.borderless)
    }

    @ViewBuilder
    private func rallyTools(_ index: Int) -> some View {
        if let point = rally.first(where: { $0.index == index }) {
            rallyFields(point)
        }
        Button("Delete rally point") {
            let count = rally.count
            onBridge(then: { selected = rallyAfterRemove(index, count) }) { FenceBridge.removeRallyPoint(index) }
        }
        .buttonStyle(.borderless)
    }

    @ViewBuilder
    private func rallyFields(_ point: RallyPoint) -> some View {
        PlanTextField(label: "Latitude", text: $rallyLatitudeTyped, width: 150) {
            if let entered = parsedCoordinate(rallyLatitudeTyped, LATITUDE_LIMIT) {
                onBridge("Moving rally point") { FenceBridge.moveRallyPoint(point.index, entered, point.longitude, point.altitudeMetres) }
            } else {
                say("Not a Latitude")
            }
        }
        PlanTextField(label: "Longitude", text: $rallyLongitudeTyped, width: 150) {
            if let entered = parsedCoordinate(rallyLongitudeTyped, LONGITUDE_LIMIT) {
                onBridge("Moving rally point") { FenceBridge.moveRallyPoint(point.index, point.latitude, entered, point.altitudeMetres) }
            } else {
                say("Not a Longitude")
            }
        }
        if rallyAltitudeIsEditable(point) {
            PlanTextField(label: rallyAltitudeLabel(point), text: $rallyAltitudeTyped, width: 120) {
                if let shown = parsedAltitude(rallyAltitudeTyped) {
                    onBridge("Setting altitude") { FenceBridge.setRallyAltitude(point.altitudePath, shown) }
                } else {
                    say("Not an altitude")
                }
            }
        }
    }

    @ViewBuilder
    private var dialogs: some View {
        if let target = radiusFor {
            let vertices = shapeVertices(target, fences, surveyList)
            let shape = shapeEditable(target, fences, surveyList)
            RadiusDialog(radius: circleRadius(vertices) ?? 0, unit: shape?.distanceUnit ?? "m", metresPerUnit: shape?.metresPerUnit ?? 1, onDismiss: { radiusFor = nil }) { radius in
                radiusFor = nil
                if let ring = circleAround(vertices, radius) {
                    onBridge("Changed the circle radius") { replaceShape(target, ring) }
                }
            }
        }
        if let (hit, at) = positioning {
            PositionDialog(at: at, title: positionTitle(hit), onDismiss: { positioning = nil }) { moved in
                positioning = nil
                let surveys = surveyList, points = rally, polygons = fences, listed = allItems, rings = circles
                onBridge(done: movedText(hit, allItems)) { writeMove(hit, moved.latitude, moved.longitude, surveys, points, polygons, listed, rings) }
            }
        }
        if loadArmed {
            PlanDialog(title: "Load plan from vehicle?", onDismiss: { loadArmed = false }) {
                Text(replaceWarning(allItems.filter { $0.index != HOME_ITEM }.count))
            } buttons: {
                Button("Keep mine") { loadArmed = false }
                Button("Replace", action: download)
            }
        }
        if let gate = uploadAsk {
            PlanDialog(title: uploadHeading(gate, vehicleChoices(vehiclesJson)), onDismiss: { uploadAsk = nil }) {
                Text(gate.refusal)
            } buttons: {
                Button("Cancel") { uploadAsk = nil }
                Button(gate.proceedTitle.ifBlank("Upload")) {
                    let pauses = gate.pausesFirst
                    uploadAsk = nil
                    sendPlan(pauseFirst: pauses)
                }
            }
        }
        if !patternWanted.isEmpty {
            PlanDialog(title: "Which pattern?", onDismiss: { patternWanted = [] }) {
                VStack(alignment: .leading, spacing: 8) {
                    Text(patternWanted.first { !addablePattern($0) && !$0.disabledReason.isBlank }?.disabledReason ?? "A pattern covers an area or a line with a camera run.")
                    ForEach(patternWanted, id: \.id) { kind in
                        Button(kind.label) {
                            patternWanted = []
                            addMissionItem(kind.id, "Adding \(kind.label.lowercased())", placeAt(), insertAfter(selected, allItems))
                        }
                        .buttonStyle(.borderless)
                        .disabled(!addablePattern(kind))
                    }
                }
            } buttons: {
                Button("Cancel") { patternWanted = [] }
            }
        }
        if let current = breach, editingBreach {
            BreachReturnDialog(
                breach: current,
                onDismiss: { editingBreach = false },
                onAltitude: { shown in
                    editingBreach = false
                    onBridge("Setting breach return altitude") { FenceBridge.setBreachAltitude(current.altitudePath, shown) }
                },
                onRemove: {
                    editingBreach = false
                    onBridge("Removing breach return point") { FenceBridge.clearBreachReturn() }
                }
            )
        }
    }

    private func focusItem(_ index: Int) {
        selected = .Waypoint(index: index)
        if let placed = items.first(where: { $0.index == index }) {
            centreOn = TrackPoint(latitude: placed.latitude, longitude: placed.longitude)
            centreRequest += 1
            follow = false
        }
    }

    private func pickRow(_ row: ItemRow) { focusItem(row.index) }

    private func refresh() async {
        let readAt = edits
        let next = await offMain { PlanRead.read() }
        if fitsPlanOnEntry(firstRead, next.planRead, next.drawn) {
            follow = false
            fitRequest += 1
        }
        firstRead = stillFirstRead(firstRead, next.planRead)
        if !firstRead && next.drawn { centredOnEntry = true }
        allItems = next.all
        items = next.items
        itemCount = next.itemCount
        shape = next.shape
        linkStartToHome = next.link
        fences = next.fences
        rally = next.rally
        gcsOperator = next.gcs
        if readAt == edits && !selectionSurvives(selected, next.all, next.fences, next.circles, next.rally, next.surveys, landings: next.landings, breach: next.breach != nil) {
            selected = nil
        }
        circles = next.circles
        firmware = next.firmware
        breach = next.breach
        surveyList = next.surveys
        landingList = next.landings
        surveyStatsMap = next.stats
    }

    private func say(_ message: String) {
        busy = message
        Task { @MainActor in
            await pause(FAILURE_MESSAGE_MS)
            busy = nil
        }
    }

    private func removeItem(_ item: MissionItem) {
        Task { @MainActor in
            busy = "Removing #\(item.sequence)"
            let index = item.index
            let outcome = await offMain { removeMissionItem(index) }
            selected = outcome.ok ? selectionAfterRemove(item.index, allItems.count) : nil
            busy = outcome.ok ? nil : outcome.reason
            if !outcome.ok {
                await pause(FAILURE_MESSAGE_MS)
                busy = nil
            }
        }
    }

    private func addMissionItem(_ kindId: String, _ label: String, _ at: TrackPoint?, _ index: Int = AT_END) {
        Task { @MainActor in
            guard let at else {
                busy = "Move the map to where this should go"
                await pause(FAILURE_MESSAGE_MS)
                busy = nil
                return
            }
            busy = label
            let outcome = await offMain { insertMissionItem(kindId, at.latitude, at.longitude, index) }
            guard outcome.ok else {
                busy = outcome.reason
                await pause(FAILURE_MESSAGE_MS)
                busy = nil
                return
            }
            busy = nil
            guard let added = outcome.index else { return }
            let _: Bool = await offMain {
                kindId == KIND_LAND ? placeLandingIfUnplaced(added, at.latitude, at.longitude)
                    : kindId == KIND_TAKEOFF ? placeTakeoff(added, at.latitude, at.longitude)
                    : true
            }
            edits += 1
            selected = .Waypoint(index: added)
        }
    }

    private func onRefusal(_ label: String? = nil, _ work: @escaping @Sendable () -> String?) {
        busy = label
        Task { @MainActor in
            let refusal = await offMain(work)
            busy = refusal
            if refusal != nil { await pause(FAILURE_MESSAGE_MS) }
            busy = nil
        }
    }

    private func onBridge(_ label: String? = nil, done: String? = nil, then: (() -> Void)? = nil, _ work: @escaping @Sendable () -> Bool) {
        busy = label
        Task { @MainActor in
            let ok = await offMain(work)
            if ok {
                if let then {
                    edits += 1
                    then()
                }
                busy = done
                if done != nil {
                    await pause(FAILURE_MESSAGE_MS)
                    busy = nil
                }
            } else {
                busy = "\(label ?? "That") did not work"
                await pause(FAILURE_MESSAGE_MS)
                busy = nil
            }
        }
    }

    private func startUpload() {
        if let refusal = syncRefusal(vehicleSyncState(planOffline, planSyncing), "upload to") {
            return say(refusal)
        }
        Task { @MainActor in
            let view = await offMain { freshPlanView() }
            switch uploadStep(uploadGate(view), notReady: notReadyToSend(view)) {
            case .Refuse(let reason):
                PlanFocus.notReady(view)
                say(reason)
            case .Confirm(let gate):
                uploadAsk = gate
            case .Send:
                sendPlan()
            }
        }
    }

    private func sendPlan(pauseFirst: Bool = false) {
        busy = "Uploading to vehicle"
        Task { @MainActor in
            let outcome = await offMain { () -> UploadOutcome in
                if pauseFirst { PlanBridge.pauseVehicle() }
                return uploadOutcome(PlanBridge.sendToVehicle())
            }
            busy = uploadMessage(outcome)
            await pause(FAILURE_MESSAGE_MS)
            busy = nil
        }
    }

    private func download() {
        loadArmed = false
        busy = "Downloading from vehicle"
        selected = nil
        Task { @MainActor in
            let outcome = await offMain { uploadOutcome(PlanBridge.loadFromVehicle()) }
            busy = downloadMessage(outcome)
            await pause(FAILURE_MESSAGE_MS)
            busy = nil
        }
    }

    private func requestDownload() {
        if let refusal = syncRefusal(vehicleSyncState(planOffline, planSyncing), "download from") {
            say(refusal)
        } else if loadStep(planDirty, loadArmed) == .Confirm {
            loadArmed = true
        } else {
            download()
        }
    }
}

private struct SelectionKey: Equatable {
    let selected: MapHit?
    let sequence: Int?
}

private struct FenceHeading: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.titleSmall)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 4)
            .padding(.top, 8)
            .padding(.bottom, 4)
    }
}

private struct PaletteNote: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        Text(text)
            .font(.bodySmall)
            .foregroundStyle(theme.colors.onSurfaceVariant)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 4)
            .padding(.vertical, 4)
    }
}

private struct ItemListHeading: View {
    let rows: [ItemRow]
    let summary: String
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(itemCountText(rows.filter { $0.index != HOME_ITEM }.count)).font(.titleMedium)
            if !summary.isBlank {
                Text(summary).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

func hexColour(_ hex: String) -> Color {
    let digits = hex.removingPrefix("#")
    guard let value = UInt32(digits, radix: 16) else { return .gray }
    return digits.count == 8 ? Color(hex: value & 0xFFFFFF).opacity(Double(value >> 24) / 255) : Color(hex: value)
}

private struct ItemRowView: View {
    let row: ItemRow
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack(spacing: 16) {
                ZStack {
                    if row.readyForSave {
                        Circle().fill(hexColour(row.colour))
                    } else {
                        Circle().stroke(theme.aircast.warning, lineWidth: 1)
                    }
                    Text(row.number)
                        .font(.labelLarge)
                        .lineLimit(1)
                        .foregroundStyle(row.readyForSave ? theme.colors.surface : theme.aircast.warning)
                }
                .frame(width: ITEM_MARKER_SIZE, height: ITEM_MARKER_SIZE)
                VStack(alignment: .leading, spacing: 2) {
                    Text(sentenceCase(row.name)).font(.titleMedium).lineLimit(1)
                    if !row.detail.isBlank {
                        Text(row.detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(2)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if selected { Text("Selected").font(.labelMedium) }
                Image(.chevronRight).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurface)
            .padding(.leading, 16)
            .padding(.trailing, 12)
            .padding(.vertical, 8)
            .frame(minHeight: 72)
            .background(selected ? theme.colors.secondaryContainer : theme.colors.surfaceContainerLow)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

let MAP_TYPES_VIEW = "view.mapTypes"

struct MapTypes: Equatable {
    var current: String
    var types: [String]
    var path: String
}

func mapTypes(_ view: JSON?) -> MapTypes? {
    guard let view, view.has("types") else { return nil }
    return MapTypes(current: view["current"].string, types: view["types"].array.map(\.string), path: view["path"].string)
}

private struct MapTypeMenu: View {
    let onStyle: (String) -> Void
    @State private var open = false
    @State private var listed: MapTypes?

    var body: some View {
        let types = listed
        RailButton(icon: .layers, label: "Map", onClick: {
            Task { @MainActor in
                listed = await offMain { mapTypes(MapBridge.read(MAP_TYPES_VIEW)) }
                open = true
            }
        })
        .confirmationDialog("Map type", isPresented: $open) {
            ForEach(types?.types ?? [], id: \.self) { type in
                Button(type == types?.current ? "\(type) ✓" : type) {
                    guard let path = types?.path else { return }
                    Task { @MainActor in
                        let style = await offMain { () -> String in
                            setOk(path, type)
                            return qgcRasterStyle(currentMapType())
                        }
                        onStyle(style)
                    }
                }
            }
        }
    }
}

func missionFitPoints(_ items: [MissionItem]) -> [TrackPoint] {
    items.filter { isPlottable($0.latitude, $0.longitude) }.map { TrackPoint(latitude: $0.latitude, longitude: $0.longitude) }
}

func parsedCoordinate(_ latitude: String, _ longitude: String) -> TrackPoint? {
    guard let lat = parsedCoordinate(latitude, LATITUDE_LIMIT), let lon = parsedCoordinate(longitude, LONGITUDE_LIMIT) else { return nil }
    return TrackPoint(latitude: lat, longitude: lon)
}

func cornerPosition(_ hit: MapHit, _ fences: [FencePolygon], _ surveys: [Survey]) -> TrackPoint? {
    switch hit {
    case .FenceVertex(let polygon, let vertex):
        return fences.first { $0.index == polygon }.flatMap { $0.vertices.indices.contains(vertex) ? $0.vertices[vertex] : nil }
    case .SurveyVertex(let item, let vertex):
        return surveys.first { $0.index == item }.flatMap { $0.area.indices.contains(vertex) ? $0.area[vertex] : nil }
    default:
        return nil
    }
}

private struct RadiusDialog: View {
    let radius: Double
    let unit: String
    let metresPerUnit: Double
    let onDismiss: () -> Void
    let onSet: (Double) -> Void
    @State private var text = ""

    var body: some View {
        let parsed = circleRadiusMetres(text, metresPerUnit)
        PlanDialog(title: "Set radius", onDismiss: onDismiss) {
            PlanTextField(label: "Radius (\(unit))", text: $text)
        } buttons: {
            Button("Cancel", action: onDismiss)
            Button("Set") { parsed.map(onSet) }.disabled(parsed == nil)
        }
        .onChange(of: [radius, metresPerUnit], initial: true) { text = String(format: "%.1f", radius / metresPerUnit) }
    }
}

private struct PositionDialog: View {
    let at: TrackPoint
    let title: String
    let onDismiss: () -> Void
    let onMove: (TrackPoint) -> Void
    @State private var latitude = ""
    @State private var longitude = ""

    var body: some View {
        let parsed = parsedCoordinate(latitude, longitude)
        PlanDialog(title: title, onDismiss: onDismiss) {
            VStack(alignment: .leading, spacing: 8) {
                PlanTextField(label: "Latitude", text: $latitude)
                PlanTextField(label: "Longitude", text: $longitude)
            }
        } buttons: {
            Button("Cancel", action: onDismiss)
            Button("Move") { parsed.map(onMove) }.disabled(parsed == nil)
        }
        .onChange(of: [at.latitude, at.longitude], initial: true) {
            latitude = String(format: "%.7f", at.latitude)
            longitude = String(format: "%.7f", at.longitude)
        }
    }
}

struct CenterMenu: View {
    let launch: TrackPoint?
    let myLocation: TrackPoint?
    let onCentre: (TrackPoint) -> Void
    var launchLabel: String = "Launch"
    var missionPoints: [TrackPoint]? = nil
    var onFit: ([TrackPoint]?) -> Void = { _ in }
    var following: Bool? = nil
    var onFollow: () -> Void = {}
    var anchor: (() -> AnyView)? = nil
    @MapPath(VEHICLES_VIEW) private var fleetJson
    @Environment(\.theme) private var theme
    @State private var asking = false
    @State private var latitude = ""
    @State private var longitude = ""

    private var vehicle: TrackPoint? {
        vehicleChoices(fleetJson).choices.first { $0.active }
            .flatMap { isPlottable($0.latitude, $0.longitude) ? TrackPoint(latitude: $0.latitude, longitude: $0.longitude) : nil }
    }

    var body: some View {
        Menu {
            if let on = following {
                Toggle("Follow vehicle", isOn: Binding(get: { on }, set: { _ in onFollow() }))
                    .disabled(vehicle == nil)
            }
            if let points = missionPoints {
                Button("Mission") { onFit(points) }
                Button("All items") { onFit(nil) }
            }
            Button(launchLabel) { launch.map(onCentre) }
                .disabled(launch == nil)
            Button("Vehicle") { vehicle.map(onCentre) }
                .disabled(vehicle == nil)
            Button("My location") { myLocation.map(onCentre) }
                .disabled(myLocation == nil)
            Button("Coordinates\u{2026}") {
                latitude = ""
                longitude = ""
                asking = true
            }
        } label: {
            if let anchor {
                anchor()
            } else {
                Text("Center map")
                    .foregroundStyle(theme.colors.onSecondaryContainer)
                    .padding(.horizontal, Space.s4)
                    .padding(.vertical, Space.s2)
                    .background(theme.colors.secondaryContainer, in: Capsule())
            }
        }
        .background {
            if asking {
                let parsed = parsedCoordinate(latitude, longitude)
                PlanDialog(title: "Center map on coordinate", onDismiss: { asking = false }) {
                    VStack(alignment: .leading, spacing: 8) {
                        PlanTextField(label: "Latitude", text: $latitude)
                        PlanTextField(label: "Longitude", text: $longitude)
                    }
                } buttons: {
                    Button("Cancel") { asking = false }
                    Button("Center") {
                        asking = false
                        parsed.map(onCentre)
                    }
                    .disabled(parsed == nil)
                }
            }
        }
    }
}

private struct StatTile: View {
    let label: String
    let value: String
    @Environment(\.theme) private var theme

    var body: some View {
        let unit = value.contains(" ") ? String(value.split(separator: " ", omittingEmptySubsequences: false).last ?? "") : ""
        let number = value.removingSuffix(unit).trimmed
        VStack(alignment: .leading, spacing: 0) {
            Text(label).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            HStack(alignment: .lastTextBaseline, spacing: 4) {
                Text(number).font(.titleLarge).minimumScaleFactor(0.5)
                if !unit.isBlank {
                    Text(unit).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant).fixedSize()
                }
            }
            .lineLimit(1)
        }
        .padding(.trailing, 20)
        .padding(.vertical, 4)
    }
}

enum PlanLayer: CaseIterable {
    case Mission, Fence, Rally

    var label: String {
        switch self {
        case .Mission: "Mission"
        case .Fence: "Fence"
        case .Rally: "Rally"
        }
    }

    var icon: Icon {
        switch self {
        case .Mission: .route
        case .Fence: .shield
        case .Rally: .planFlag
        }
    }
}

func ownerOf(_ hit: MapHit?) -> String? {
    switch hit {
    case .Waypoint(let index), .LandingPlace(let index, _), .LoiterRadius(let index), .LoiterRotation(let index): "m\(index)"
    case .SurveyVertex(let item, _): "m\(item)"
    case .ShapeCentre(let fence, let owner), .ShapeRadius(let fence, let owner): fence ? "p\(owner)" : "m\(owner)"
    case .FenceVertex(let polygon, _): "p\(polygon)"
    case .Circle(let index), .CircleCentre(let index), .CircleRadius(let index): "c\(index)"
    case .Rally(let index): "r\(index)"
    case .BreachReturn: "breach"
    default: nil
    }
}

func dragAllowed(_ hit: MapHit, _ selected: MapHit?, _ layer: PlanLayer) -> Bool {
    switch hit {
    case .Midpoint(let path, _, _):
        if let owner = midpointOwner(path) { return owner == ownerOf(selected) }
        if case .Waypoint = selected { return true }
        return false
    case .BreachReturn:
        return layer == .Fence
    default:
        return layerOf(hit) == layer && ownerOf(hit) != nil && ownerOf(hit) == ownerOf(selected)
    }
}

func profileShown(_ layer: PlanLayer, _ profile: TerrainProfile) -> Bool { layer == .Mission && !profile.points.isEmpty }

func actsOnTap(_ hit: MapHit) -> Bool {
    switch hit {
    case .Midpoint, .LoiterRotation, .BreachReturn: true
    default: false
    }
}

func midpointOwner(_ path: String) -> String? {
    func owner(_ prefix: String) -> String {
        String(path.removingPrefix(prefix).prefix { $0 != "." })
    }
    if path.hasPrefix("\(FENCE_POLYGONS).") { return "p\(owner("\(FENCE_POLYGONS)."))" }
    if path.hasPrefix("\(PLAN_ITEMS).") { return "m\(owner("\(PLAN_ITEMS)."))" }
    return nil
}

func layerOf(_ hit: MapHit?) -> PlanLayer? {
    switch hit {
    case nil: nil
    case .Rally: .Rally
    case .FenceVertex, .Circle, .CircleCentre, .CircleRadius, .BreachReturn: .Fence
    case .ShapeCentre(let fence, _), .ShapeRadius(let fence, _): fence ? .Fence : .Mission
    case .Midpoint: nil
    default: .Mission
    }
}

private struct FenceListRow: View {
    let row: FenceRow
    let chosen: Bool
    let onSelect: () -> Void
    var onInclusion: ((Bool) -> Void)? = nil
    let onRemove: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(row.title).font(.bodyLarge)
                if !row.detail.isBlank {
                    Text(row.detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if !row.radius.isBlank {
                Text(row.radius).font(.bodyMedium).padding(.horizontal, 8)
            }
            if let onInclusion, let inclusion = row.inclusion {
                Toggle("Inclusion", isOn: Binding(get: { inclusion }, set: onInclusion))
                    .labelsHidden()
                    .padding(.horizontal, 8)
            }
            Button(action: onRemove) {
                Image(.close)
            }
            .buttonStyle(.borderless)
            .accessibilityLabel("Remove \(row.title)")
        }
        .padding(.vertical, 4)
        .background(chosen ? theme.colors.secondaryContainer : Color.clear, in: RoundedRectangle(cornerRadius: Corner.small))
        .contentShape(Rectangle())
        .onTapGesture(perform: onSelect)
    }
}

struct PlanTextField: View {
    let label: String
    @Binding var text: String
    var suffix: String = ""
    var placeholder: String = ""
    var width: CGFloat? = nil
    var enabled: Bool = true
    var isError: Bool = false
    var onDone: () -> Void = {}
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.labelSmall)
                .foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            HStack(spacing: 4) {
                TextField(placeholder, text: $text)
                    .font(.bodySmall)
                    .keyboardType(.numbersAndPunctuation)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .submitLabel(.done)
                    .onSubmit(onDone)
                if !suffix.isBlank {
                    Text(suffix).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(isError ? theme.colors.error : theme.colors.outline, lineWidth: 1))
        }
        .frame(width: width)
        .disabled(!enabled)
        .opacity(enabled ? 1 : 0.38)
    }
}

struct PlanChip: View {
    let label: String
    let selected: Bool
    var enabled: Bool = true
    var small: Bool = false
    var translucent: Bool = false
    let action: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: action) {
            HStack(spacing: 4) {
                if selected { Image(.check).font(.labelSmall) }
                Text(label).font(small ? .labelSmall : .labelLarge)
            }
            .padding(.horizontal, 12)
            .frame(minHeight: 32)
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
            .background(
                selected
                    ? (translucent ? theme.colors.primaryContainer.opacity(0.92) : theme.colors.secondaryContainer)
                    : (translucent ? theme.colors.surface.opacity(0.88) : Color.clear),
                in: RoundedRectangle(cornerRadius: Corner.small)
            )
            .overlay(RoundedRectangle(cornerRadius: Corner.small).stroke(selected ? Color.clear : theme.colors.outline, lineWidth: 1))
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : 0.38)
    }
}

struct PlanMenuField: View {
    let label: String
    let value: String
    var width: CGFloat? = nil
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            HStack {
                Text(value).font(.bodySmall).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                Image(.arrowDropDown).font(.labelSmall)
            }
            .foregroundStyle(theme.colors.onSurface)
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(theme.colors.outline, lineWidth: 1))
        }
        .frame(width: width)
        .frame(maxWidth: width == nil ? .infinity : nil)
    }
}

struct PlanFlowRow: Layout {
    var spacing: CGFloat = 8
    var lineSpacing: CGFloat = 8
    var alignment: HorizontalAlignment = .leading

    private func rows(_ maxWidth: CGFloat, _ sizes: [CGSize]) -> [[Int]] {
        sizes.indices.reduce(into: [[Int]]()) { rows, index in
            let used: CGFloat = rows.last.map { row in lineWidth(row, sizes) + spacing } ?? 0
            if let last = rows.indices.last, used + sizes[index].width <= maxWidth {
                rows[last].append(index)
            } else {
                rows.append([index])
            }
        }
    }

    private func sizes(_ proposal: ProposedViewSize, _ subviews: Subviews) -> [CGSize] {
        subviews.map { $0.sizeThatFits(ProposedViewSize(width: proposal.width, height: nil)) }
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let measured = sizes(proposal, subviews)
        let lines = rows(proposal.width ?? .infinity, measured)
        let widths: [CGFloat] = lines.map { row in lineWidth(row, measured) }
        let heights: [CGFloat] = lines.map { row in lineHeight(row, measured) }
        let gaps = CGFloat(max(lines.count - 1, 0)) * lineSpacing
        return CGSize(width: proposal.width ?? widths.max() ?? 0, height: heights.reduce(0, +) + gaps)
    }

    private func lineWidth(_ row: [Int], _ measured: [CGSize]) -> CGFloat {
        let content: CGFloat = row.map { measured[$0].width }.reduce(0, +)
        return content + CGFloat(max(row.count - 1, 0)) * spacing
    }

    private func lineHeight(_ row: [Int], _ measured: [CGSize]) -> CGFloat {
        row.map { measured[$0].height }.max() ?? 0
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let measured = sizes(proposal, subviews)
        let lines = rows(bounds.width, measured)
        _ = lines.reduce(bounds.minY) { top, row in
            let height = lineHeight(row, measured)
            let slack = bounds.width - lineWidth(row, measured)
            let start = bounds.minX + (alignment == .center ? slack / 2 : 0)
            _ = row.reduce(start) { left, index in
                subviews[index].place(at: CGPoint(x: left, y: top + (height - measured[index].height) / 2), proposal: ProposedViewSize(measured[index]))
                return left + measured[index].width + spacing
            }
            return top + height + lineSpacing
        }
    }
}
