import SwiftUI

private let FLY_POLL_MS = 2000
private let CAMERA_STORE = "fly-map-camera"
private let CAMERA_KEY = "camera"

struct SavedCamera: Equatable {
    var centre: TrackPoint
    var zoom: Double
}

func readCamera() -> SavedCamera? {
    let parts = UserDefaults(suiteName: CAMERA_STORE)?.string(forKey: CAMERA_KEY)?.split(separator: ",").compactMap { Double($0) } ?? []
    guard parts.count == 3 else { return nil }
    let camera = SavedCamera(centre: TrackPoint(latitude: parts[0], longitude: parts[1]), zoom: parts[2])
    return isPlottable(camera.centre.latitude, camera.centre.longitude) && camera.zoom > 1.0 ? camera : nil
}

private func writeCamera(_ camera: SavedCamera) {
    UserDefaults(suiteName: CAMERA_STORE)?.set("\(camera.centre.latitude),\(camera.centre.longitude),\(camera.zoom)", forKey: CAMERA_KEY)
}

func centresOnOperator(_ alreadyCentred: Bool, _ operator: TrackPoint?, _ vehiclePlaced: Bool) -> Bool {
    !alreadyCentred && `operator` != nil && !vehiclePlaced
}

private struct FlownPlan {
    var items: [MissionItem] = []
    var linkStartToHome = false
    var fences: [FencePolygon] = []
    var circles: [FenceCircle] = []
    var firmwareFence: FirmwareFence? = nil
    var breachReturn: TrackPoint? = nil
    var rally: [RallyPoint] = []
    var surveys: [Survey] = []
    var `operator`: TrackPoint? = nil
    var operatorHeading = Double.nan
    var shots: [TrackPoint] = []
    var traffic: [TrafficMark] = []
    var gimbals: [GimbalAzimuth] = []
    var roi: TrackPoint? = nil
    var goto: GotoLocation? = nil
    var orbit: OrbitCircle? = nil
    var current: Int? = nil
    var others: [OtherMission] = []
    var vehiclePlaced = false
}

private let FLY_MISSION_ITEMS = "view.flyMissionItems(geometry)"

private enum FlyMapStyle {
    nonisolated(unsafe) static var last = ""
}

func otherMissions(_ view: JSON?) -> [OtherMission] {
    guard let others = view?["others"].arrayOrNil else { return [] }
    return others.filter { $0.object != nil }.map { OtherMission(items: missionItems($0), linkStartToHome: linksStartToHome($0)) }
}

func missionArrived<T, K: Equatable>(_ before: [T], _ after: [T], _ key: (T) -> K) -> Bool {
    !after.isEmpty && after.map(key) != before.map(key)
}

private struct ItemShape: Equatable {
    let sequence: Int
    let latitude: Double
    let longitude: Double
    let command: String
}

private func shape(_ item: MissionItem) -> ItemShape {
    ItemShape(sequence: item.sequence, latitude: item.latitude, longitude: item.longitude, command: item.command)
}

private struct FlyCamera {
    var centreRequest: Int
    var centreOn: TrackPoint?
    var centreZoom: Double?
    var mainZoom: Double
    var zoomedForPip = false

    init(_ saved: SavedCamera?) {
        centreRequest = saved == nil ? 0 : 1
        centreOn = saved?.centre
        centreZoom = saved?.zoom
        mainZoom = saved?.zoom ?? 0
    }
}

private func flownPlan() -> FlownPlan {
    let raw = MapBridge.read(FLY_MISSION_ITEMS)
    let fences = FenceBridge.readFlown()
    let gcs = OperatorBridge.read()
    if raw != nil { MapBridge.markReachable() }
    return FlownPlan(
        items: missionItems(raw),
        linkStartToHome: linksStartToHome(raw),
        fences: fencePolygons(fences),
        circles: fenceCircles(fences),
        firmwareFence: firmwareFence(fences),
        breachReturn: breachReturn(fences)?.point,
        rally: rallyPoints(fences),
        surveys: SurveyBridge.surveysFrom(raw),
        operator: operatorPoint(gcs),
        operatorHeading: operatorHeading(gcs),
        shots: shotPoints(VideoBridge.read()),
        traffic: TrafficBridge.read(),
        gimbals: GimbalBridge.read(),
        roi: RoiBridge.read(),
        goto: GotoBridge.read(),
        orbit: OrbitBridge.read(),
        current: raw.map { $0["selected"].int(-1) }.flatMap { $0 > 0 ? $0 : nil },
        others: otherMissions(raw),
        vehiclePlaced: vehicleChoices(MapBridge.read(VEHICLES_VIEW)).choices.contains { isPlottable($0.latitude, $0.longitude) }
    )
}

struct FlyMap: View {
    var cameraBottomPx: CGFloat = 0
    var topInsetPx: CGFloat = 0
    var bottomInsetPx: CGFloat = 0
    var logoEndInsetPx: CGFloat? = nil
    var pip: Bool = false
    var onMapClick: ((Double, Double) -> Void)? = nil
    var onMissionItemClick: ((Int) -> Void)? = nil
    var onRoiClick: ((TrackPoint) -> Void)? = nil
    var onTrafficClick: (() -> Void)? = nil
    var clickMarker: TrackPoint? = nil

    @Environment(\.theme) private var theme
    @Environment(\.scenePhase) private var scenePhase
    @MapBool("view.control(settings.flyViewSettings.keepMapCenteredOnVehicle)") private var keepCentered
    @State private var style = FlyMapStyle.last
    @State private var plan = FlownPlan()
    @State private var centre: TrackPoint?
    @State private var zoom = 0.0
    @State private var fitRequest = 0
    @State private var camera = FlyCamera(readCamera())

    var body: some View {
        ZStack {
            VehicleMap(
                mapStyle: style,
                follow: true,
                keepCentered: keepCentered || pip,
                missionItems: plan.items,
                linkStartToHome: plan.linkStartToHome,
                fencePolygons: plan.fences,
                fenceCircles: plan.circles,
                rallyPoints: plan.rally,
                operator: plan.operator,
                operatorHeading: plan.operatorHeading,
                surveys: plan.surveys,
                shots: plan.shots,
                editable: false,
                selectedWaypoint: plan.current,
                firmwareFence: plan.firmwareFence,
                onCentreChanged: { at, level in centreChanged(at, level) },
                bottomInsetPx: bottomInsetPx,
                topInsetPx: topInsetPx,
                logoEndInsetPx: logoEndInsetPx,
                cameraBottomPx: cameraBottomPx,
                pip: pip,
                fitRequest: fitRequest,
                centreRequest: camera.centreRequest,
                centreOn: camera.centreOn,
                centreZoom: camera.centreZoom,
                onMapClick: onMapClick,
                onMissionItemClick: onMissionItemClick,
                traffic: plan.traffic,
                onTrafficClick: onTrafficClick,
                gimbals: plan.gimbals,
                breachReturn: plan.breachReturn,
                proximityRadar: true,
                obstacleOverlay: true,
                roi: plan.roi,
                onRoiClick: onRoiClick,
                goto: plan.goto,
                clickMarker: clickMarker,
                orbit: plan.orbit,
                otherMissions: plan.others
            )
            if let at = centre, zoom > 0, !pip {
                ScaleBarView(latitude: at.latitude, zoom: zoom)
                    .padding(logoEndInsetPx.map { EdgeInsets(top: 0, leading: 0, bottom: bottomInsetPx + SCALE_ABOVE_LOGO, trailing: $0) }
                        ?? EdgeInsets(top: 0, leading: Space.s1, bottom: Space.s1, trailing: 0))
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: logoEndInsetPx == nil ? .bottomLeading : .bottomTrailing)
                    .allowsHitTesting(false)
            }
        }
        .background(theme.colors.surface)
        .onChange(of: Keys(pip, centre != nil), initial: true) { zoomForPip() }
        .task(id: scenePhase == .active) {
            guard scenePhase == .active else { return }
            while !Task.isCancelled {
                let mapStyle = await offMain { planMapStyle() }
                if mapStyle != style {
                    style = mapStyle
                    FlyMapStyle.last = mapStyle
                }
                let next = await offMain { flownPlan() }
                if missionArrived(plan.items, next.items, shape) { fitRequest += 1 }
                if centresOnOperator(FlightMapPosition.operatorCentred, next.operator, next.vehiclePlaced) {
                    FlightMapPosition.operatorCentred = true
                    camera.centreOn = next.operator
                    camera.centreZoom = nil
                    camera.centreRequest += 1
                }
                plan = next
                try? await Task.sleep(for: .milliseconds(FLY_POLL_MS))
            }
        }
    }

    private func zoomForPip() {
        guard let at = centre, pip != camera.zoomedForPip else { return }
        camera.zoomedForPip = pip
        guard let level = pipZoom(camera.mainZoom, pip) else { return }
        camera.centreOn = at
        camera.centreZoom = level
        camera.centreRequest += 1
    }

    private func centreChanged(_ at: TrackPoint, _ level: Double) {
        centre = at
        zoom = level
        FlightMapPosition.latest = at
        guard level > 1.0 && !pip else { return }
        camera.mainZoom = level
        writeCamera(SavedCamera(centre: at, zoom: level))
    }
}

private let SCALE_ABOVE_LOGO: CGFloat = 46
private let SCALE_BAR_HEIGHT: CGFloat = 32

let MAP_SCALE_CLEARANCE: CGFloat = SCALE_ABOVE_LOGO + SCALE_BAR_HEIGHT

func pipZoom(_ mainZoom: Double, _ pip: Bool) -> Double? {
    mainZoom <= 0 ? nil : !pip ? mainZoom : mainZoom > 3 ? mainZoom - 3 : nil
}
