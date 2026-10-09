import Foundation

let PLAN_ROOT = "plan"
let PLAN_ITEMS = "\(PLAN_ROOT).missionController.visualItems"
let PLAN_VIEW = "view.missionItems(geometry)"

let KIND_LAND = "land"
let KIND_WAYPOINT = "waypoint"
let KIND_TAKEOFF = "takeoff"
let KIND_SURVEY = "survey"
let KIND_CORRIDOR = "corridor"
let KIND_STRUCTURE = "structure"

struct MissionKind: Equatable, Hashable {
    var id: String
    var label: String
    var title: String
    var enabled: Bool
    var disabledReason: String
}

func missionKinds(_ view: JSON?) -> [MissionKind] {
    guard let kinds = view?["kinds"].arrayOrNil else { return [] }
    return kinds.filter { $0.object != nil }.map { kind in
        MissionKind(
            id: kind["id"].string,
            label: kind["complexName"].string,
            title: kind["title"].string,
            enabled: kind["enabled"].bool(true),
            disabledReason: kind["disabledReason"].string
        )
    }
}

func scanPatterns(_ kinds: [MissionKind]) -> [MissionKind] {
    kinds.filter { !$0.label.isBlank }
}

func kindOffered(_ kinds: [MissionKind], _ id: String) -> Bool {
    kinds.isEmpty || kinds.contains { $0.id == id }
}

func kindAllows(_ kinds: [MissionKind], _ id: String) -> Bool {
    kinds.first { $0.id == id }?.enabled ?? true
}

func kindLabel(_ kinds: [MissionKind], _ id: String) -> String {
    kinds.first { $0.id == id }.flatMap { $0.title.isBlank ? nil : $0.title } ?? id.capitalizedFirst
}

func blockedReason(_ kinds: [MissionKind]) -> String? {
    kinds.first { !$0.enabled && !$0.disabledReason.isBlank }?.disabledReason
}

let MAV_CMD_NAV_RETURN_TO_LAUNCH = 20

func planItems(_ json: JSON?) -> [JSON]? { json?["items"].arrayOrNil }

func planItemCount(_ json: JSON?) -> Int {
    max((planItems(json)?.count ?? 0) - 1, 0)
}

func linksStartToHome(_ json: JSON?) -> Bool { json?["linksStartToHome"].bool == true }

func planShape(_ json: JSON?) -> [String] {
    guard let items = planItems(json) else { return [] }
    let endsAfter = routeEndsAfter(items)
    let listed = items.dropFirst().filter { $0.object != nil }.compactMap { element -> String? in
        element["kind"].string == KIND_TAKEOFF ? "takeoff"
            : element["command"].int(0) == MAV_CMD_NAV_RETURN_TO_LAUNCH ? "RTL" : nil
    }
    let named = listed.enumerated().filter { listed.firstIndex(of: $0.element) == $0.offset }.map(\.element)
    let stranded = items.indices.dropFirst().filter { $0 > endsAfter }.count
    return named + (stranded > 0 ? ["\(stranded) after RTL"] : [])
}

struct MissionItem: Equatable {
    var index: Int
    var sequence: Int
    var latitude: Double
    var longitude: Double
    var command: String
    var selected: Bool
    var altitude: Double = .nan
    var exit: TrackPoint? = nil
    var routed: Bool = true
    var kind: String = ""
    var commandId: Int = 0
    var placed: Bool = true
    var afterRouteEnds: Bool = false
    var altitudeText: String = ""
    var specifiesCoordinate: Bool = false
    var distance: Double = .nan
    var distanceText: String = ""
    var azimuthText: String = ""
    var headingText: String = ""
    var altitudeChange: Double = .nan
    var altitudeChangeText: String = ""
    var gradientText: String = ""
    var altitudeBandText: String = ""
    var blockedReason: String = ""
    var readyForSave: Bool = true
    var cameraShots: Int = 0
    var extraSeconds: Double = 0
    var speedChangeText: String = ""
    var foldedCommands: Int = 0
    var altitudeFrameText: String = ""
    var altitudeMode: Int = -1
    var altitudeEditUnits: String = ""
    var complexPattern: Bool = false
    var loiterRadius: Double = .nan
    var terrainCollision: Bool = false
    var heading: Double = .nan
    var gimbalYaw: Double = .nan
    var closesRoute: Bool = false
    var legBroken: Bool = false
    var abbreviation: String = ""

    static func == (lhs: MissionItem, rhs: MissionItem) -> Bool {
        lhs.doubles.map(boxedBits) == rhs.doubles.map(boxedBits)
            && lhs.index == rhs.index && lhs.sequence == rhs.sequence && lhs.command == rhs.command
            && lhs.selected == rhs.selected && lhs.exit == rhs.exit && lhs.routed == rhs.routed && lhs.kind == rhs.kind
            && lhs.commandId == rhs.commandId && lhs.placed == rhs.placed && lhs.afterRouteEnds == rhs.afterRouteEnds
            && lhs.altitudeText == rhs.altitudeText && lhs.specifiesCoordinate == rhs.specifiesCoordinate
            && lhs.distanceText == rhs.distanceText && lhs.azimuthText == rhs.azimuthText && lhs.headingText == rhs.headingText
            && lhs.altitudeChangeText == rhs.altitudeChangeText && lhs.gradientText == rhs.gradientText
            && lhs.altitudeBandText == rhs.altitudeBandText && lhs.blockedReason == rhs.blockedReason
            && lhs.readyForSave == rhs.readyForSave && lhs.cameraShots == rhs.cameraShots
            && lhs.speedChangeText == rhs.speedChangeText && lhs.foldedCommands == rhs.foldedCommands
            && lhs.altitudeFrameText == rhs.altitudeFrameText && lhs.altitudeMode == rhs.altitudeMode
            && lhs.altitudeEditUnits == rhs.altitudeEditUnits && lhs.complexPattern == rhs.complexPattern
            && lhs.terrainCollision == rhs.terrainCollision && lhs.closesRoute == rhs.closesRoute
            && lhs.legBroken == rhs.legBroken && lhs.abbreviation == rhs.abbreviation
    }

    private var doubles: [Double] {
        [latitude, longitude, altitude, distance, altitudeChange, extraSeconds, loiterRadius, heading, gimbalYaw]
    }
}

private func boxedBits(_ value: Double) -> UInt64 { value.isNaN ? Double.nan.bitPattern : value.bitPattern }

private func isReturn(_ element: JSON) -> Bool {
    element["endsRoute"].bool && element["command"].int(0) == MAV_CMD_NAV_RETURN_TO_LAUNCH
}

private func isLanding(_ element: JSON) -> Bool {
    element["endsRoute"].bool && element["command"].int(0) != MAV_CMD_NAV_RETURN_TO_LAUNCH
}

func routeEndsAfter(_ items: [JSON]?) -> Int {
    (items ?? []).firstIndex(where: isReturn) ?? Int.max
}

func legsAfterLanding(_ items: [JSON]?) -> Set<Int> {
    let list = items ?? []
    let flown = list.indices.filter { list[$0]["flownLeg"].bool }
    return Set(zip(flown, flown.dropFirst()).filter { from, _ in isLanding(list[from]) }.map(\.1))
}

func placed(_ element: JSON, _ key: String) -> TrackPoint? {
    let at = element[key]
    guard at.object != nil else { return nil }
    let latitude = at["latitude"].double(.nan)
    let longitude = at["longitude"].double(.nan)
    return isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : nil
}

func allMissionItems(_ json: JSON?) -> [MissionItem] {
    guard let items = planItems(json) else { return [] }
    let endsAfter = routeEndsAfter(items)
    let broken = legsAfterLanding(items)
    return items.enumerated().compactMap { index, element in
        guard element.object != nil else { return nil }
        let at = placed(element, "coordinate")
        let exit = placed(element, "exitCoordinate").flatMap { out in
            at == nil || out.latitude != at?.latitude || out.longitude != at?.longitude ? out : nil
        }
        return MissionItem(
            index: index,
            sequence: element["sequence"].int(index),
            latitude: at?.latitude ?? .nan,
            longitude: at?.longitude ?? .nan,
            command: element["name"].string,
            selected: element["selected"].bool,
            altitude: element["altitude"].double(.nan),
            exit: exit,
            routed: element["flownLeg"].bool && index <= endsAfter,
            kind: element["kind"].string,
            commandId: element["command"].int(0),
            placed: at != nil,
            afterRouteEnds: index > endsAfter,
            altitudeText: element["altitudeText"].string,
            specifiesCoordinate: element["specifiesCoordinate"].bool,
            distance: element["distance"].double(.nan),
            distanceText: element["distanceText"].string,
            azimuthText: element["azimuthText"].string,
            headingText: element["headingText"].string,
            altitudeChange: element["altitudeChange"].double(.nan),
            altitudeChangeText: element["altitudeChangeText"].string,
            gradientText: element["gradientText"].string,
            altitudeBandText: element["altitudeBandText"].string,
            blockedReason: element["blockedReason"].string,
            readyForSave: !element["blocked"].bool && !element["awaitingTerrain"].bool,
            cameraShots: element["cameraShots"].int(0),
            extraSeconds: element["extraSeconds"].double(0),
            speedChangeText: element["speedChangeText"].string,
            foldedCommands: element["foldedCommands"].int(0),
            altitudeFrameText: element["altitudeFrameText"].string,
            altitudeMode: element["altitudeMode"].int(-1),
            altitudeEditUnits: element["altitudeEditUnits"].string,
            complexPattern: !element["simple"].bool(true),
            loiterRadius: element["loiterRadius"].double(.nan),
            heading: element["heading"].double(.nan),
            gimbalYaw: element["gimbalYaw"].double(.nan),
            closesRoute: element["closesRoute"].bool,
            legBroken: broken.contains(index),
            abbreviation: element["abbreviation"].string
        )
    }
}

func missionItems(_ json: JSON?) -> [MissionItem] { allMissionItems(json).filter(\.placed) }

enum PlanBridge {
    static func rawItems() -> JSON? { MapBridge.read(PLAN_VIEW) }

    @discardableResult static func loadFromVehicle() -> Bool { invokeOk("\(PLAN_ROOT).loadFromVehicle") }

    @discardableResult static func sendToVehicle() -> Bool { invokeOk("\(PLAN_ROOT).sendToVehicle") }

    @discardableResult static func pauseVehicle() -> Bool { invokeOk("vehicle.pauseVehicle") }

    @discardableResult static func setAltitude(_ index: Int, _ shown: Double) -> Bool {
        setOk("\(PLAN_ITEMS).\(index).altitude", shown)
    }

    @discardableResult static func setAltitudeMode(_ index: Int, _ raw: Int) -> Bool {
        setOk(altitudeModePath(index), raw)
    }

    @discardableResult static func removeItem(_ index: Int) -> Bool { removeMissionItem(index).ok }

    static func removeItemRefusal(_ index: Int) -> String? {
        let outcome = removeMissionItem(index)
        return outcome.ok ? nil : outcome.reason
    }

    @discardableResult static func selectSequence(_ sequence: Int) -> Bool {
        invokeOk("\(MISSION_CONTROLLER).setCurrentPlanViewSeqNum", sequence, true)
    }

    @discardableResult static func setLoiterRadius(_ index: Int, _ radius: Double) -> Bool {
        setOk("\(PLAN_ITEMS).\(index).loiterRadius", radius)
    }

    @discardableResult static func moveItem(_ index: Int, _ latitude: Double, _ longitude: Double) -> Bool {
        setOk("\(PLAN_ITEMS).\(index).coordinate", coordinateJson(latitude, longitude))
    }
}

func takeoffMissing(_ items: [MissionItem]) -> Bool {
    items.contains { $0.latitude != 0 || $0.longitude != 0 } && !items.contains { $0.kind == "takeoff" }
}

func landingPatterns(_ items: [MissionItem]) -> [LandingPattern] {
    items.filter { $0.kind == KIND_LAND }.compactMap { item in
        landingPattern(item.index, MapBridge.read("view.landingPattern(\(item.index))"))
    }
}

@discardableResult
func moveLandingPlace(_ index: Int, _ place: Int, _ latitude: Double, _ longitude: Double) -> Bool {
    let property: String? = switch place {
    case LANDING_PLACE_APPROACH: "finalApproachCoordinate"
    case LANDING_PLACE_TOUCHDOWN: "landingCoordinate"
    default: nil
    }
    guard let property else { return false }
    return setOk("\(PLAN_ITEMS).\(index).\(property)", coordinateJson(latitude, longitude))
}

@discardableResult
func placeTakeoff(_ index: Int, _ latitude: Double, _ longitude: Double) -> Bool {
    setOk("\(PLAN_ITEMS).\(index).launchCoordinate", coordinateJson(latitude, longitude))
}

@discardableResult
func placeLandingIfUnplaced(_ index: Int, _ latitude: Double, _ longitude: Double) -> Bool {
    let view = MapBridge.read("view.landingPattern(\(index))")
    guard landingPattern(index, view)?.landing == nil else { return false }
    guard moveLandingPlace(index, LANDING_PLACE_TOUCHDOWN, latitude, longitude) else { return false }
    invokeOk("\(PLAN_ITEMS).\(index).setLandingHeadingToTakeoffHeading")
    return leaveWizardMode(index)
}

@discardableResult
func leaveWizardMode(_ index: Int) -> Bool {
    setOk("\(PLAN_ITEMS).\(index).wizardMode", false)
}
