import Foundation

let MISSION_CONTROLLER = "\(PLAN_ROOT).missionController"

let SHAPE_AREA = "area"
let SHAPE_LINE = "line"

struct Survey: Equatable {
    var index: Int
    var area: [TrackPoint]
    var transects: [TrackPoint]
    var cameraShots: Int
    var kind: String
    var shape: String
    var property: String
    var editable: EditableShape? = nil
    var flightLoop: [TrackPoint] = []
    var layers: Int = 0
    var layerSpanText: String = ""
    var turnaround: Bool = false
    var outline: [TrackPoint] = []
    var collides: Bool = false
}

private func points(_ array: [JSON]?) -> [TrackPoint] {
    (array ?? []).compactMap { point in
        guard point.object != nil else { return nil }
        let latitude = point["latitude"].double(.nan)
        let longitude = point["longitude"].double(.nan)
        return isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : nil
    }
}

let GRID_ANGLE_MAX: Float = 359

func gridAngleShown(_ raw: Double) -> Float {
    raw.isNaN ? 0 : min(max(Float((((raw.truncatingRemainder(dividingBy: 360)) + 360).truncatingRemainder(dividingBy: 360)).rounded()), 0), GRID_ANGLE_MAX)
}

enum SurveyBridge {
    @discardableResult static func removeVertex(_ survey: Survey, _ vertex: Int) -> Bool {
        invokeOk("\(PLAN_ITEMS).\(survey.index).\(survey.property).removeVertex", vertex)
    }

    static func surveysFrom(_ json: JSON?) -> [Survey] {
        guard let items = planItems(json) else { return [] }
        return items.enumerated().compactMap { index, element in
            let geometry = element["geometry"]
            guard element.object != nil, geometry.object != nil else { return nil }
            let shape = geometry["shape"].string
            let property = geometry["property"].string
            guard !shape.isBlank, !property.isBlank else { return nil }
            let area = points(geometry["vertices"].arrayOrNil)
            let transects = points(geometry["transects"].arrayOrNil)
            let flightLoop = points(geometry["flightLoop"].arrayOrNil)
            guard !(transects.isEmpty && flightLoop.isEmpty && area.isEmpty) else { return nil }
            return Survey(
                index: index,
                area: area,
                transects: transects,
                cameraShots: element["cameraShots"].int(0),
                kind: element["kind"].string,
                shape: shape,
                property: property,
                editable: editableShape("\(PLAN_ITEMS).\(index).\(property)", shape),
                flightLoop: flightLoop,
                layers: geometry["layers"].int(0),
                layerSpanText: geometry["layerSpanText"].string,
                turnaround: geometry["turnaround"].bool,
                outline: points(geometry["outline"].arrayOrNil)
            )
        }
    }

    static func gridAngle(_ itemIndex: Int) -> Double {
        MapBridge.read("view.control(\(PLAN_ITEMS).\(itemIndex).gridAngle)")?["value"].double(.nan) ?? .nan
    }

    private static func altitudePath(_ itemIndex: Int) -> String {
        "\(PLAN_ITEMS).\(itemIndex).cameraCalc.distanceToSurface"
    }

    static func altitude(_ itemIndex: Int) -> Double {
        MapBridge.read("view.control(\(altitudePath(itemIndex)))")?["value"].double(.nan) ?? .nan
    }

    static func altitudeUnits(_ itemIndex: Int) -> String {
        MapBridge.read("view.control(\(altitudePath(itemIndex)))")?["units"].string ?? ""
    }

    @discardableResult static func setAltitude(_ itemIndex: Int, _ shown: Double) -> Bool {
        setOk(altitudePath(itemIndex), shown)
    }

    @discardableResult static func setGridAngle(_ itemIndex: Int, _ degrees: Double) -> Bool {
        setOk("\(PLAN_ITEMS).\(itemIndex).gridAngle", degrees)
    }

    @discardableResult static func adjustVertex(_ survey: Survey, _ vertex: Int, _ latitude: Double, _ longitude: Double) -> Bool {
        invokeOk("\(PLAN_ITEMS).\(survey.index).\(survey.property).adjustVertex", vertex, coordinateJson(latitude, longitude))
    }
}

let ABSENT = "\u{2014}"

struct SurveyStats: Equatable {
    var areaText: String
    var warning: String
    var intervalText: String = ""
    var footprintText: String = ""
    var surfaceDistanceText: String = ""
    var distanceText: String = ""
    var photosText: String = ""
    var structure: StructureStats? = nil
}

struct StructureStats: Equatable {
    var layers: String
    var layerHeight: String
    var top: String
    var bottom: String
}

func statisticsRows(_ stats: SurveyStats) -> [(String, String)] {
    let shape = stats.structure.map {
        [("Layers", $0.layers), ("Layer height", $0.layerHeight), ("Top layer altitude", $0.top), ("Bottom layer altitude", $0.bottom)]
    } ?? [("Area", stats.areaText), ("Distance", stats.distanceText)]
    return shape + [("Photos", stats.photosText), ("Photo interval", stats.intervalText)]
}

private func stated(_ view: JSON, _ key: String) -> String {
    view[key].string == ABSENT ? "" : view[key].string
}

func surveyStats(_ view: JSON?) -> SurveyStats? {
    guard let view, view["available"].bool else { return nil }
    let structure = view["structure"]
    return SurveyStats(
        areaText: stated(view, "areaText"),
        warning: view["warning"].string,
        intervalText: stated(view, "intervalText"),
        footprintText: stated(view, "footprintText"),
        surfaceDistanceText: stated(view, "surfaceDistanceText"),
        distanceText: stated(view, "distanceText"),
        photosText: view["shotsText"].string,
        structure: structure.object == nil ? nil : StructureStats(layers: structure["layers"].string, layerHeight: structure["layerHeight"].string, top: structure["top"].string, bottom: structure["bottom"].string)
    )
}

func insetRing(_ corners: [TrackPoint], _ fraction: Double) -> [TrackPoint] {
    guard corners.count >= 3 else { return [] }
    let midLatitude = corners.map(\.latitude).reduce(0, +) / Double(corners.count)
    let midLongitude = corners.map(\.longitude).reduce(0, +) / Double(corners.count)
    return corners.map {
        TrackPoint(latitude: midLatitude + ($0.latitude - midLatitude) * fraction, longitude: midLongitude + ($0.longitude - midLongitude) * fraction)
    }
}

func fitSurveyArea(_ survey: Survey, _ corners: [TrackPoint]) -> Bool {
    guard !corners.isEmpty, survey.area.count == corners.count else { return false }
    return corners.enumerated().allSatisfy { vertex, at in
        SurveyBridge.adjustVertex(survey, vertex, at.latitude, at.longitude)
    }
}

func surveyStatsFor(_ items: [MissionItem]) -> [Int: SurveyStats] {
    Dictionary(uniqueKeysWithValues: items.filter(\.complexPattern).compactMap { item in
        surveyStats(MapBridge.read("view.surveyStats(\(item.index))")).map { (item.index, $0) }
    })
}
