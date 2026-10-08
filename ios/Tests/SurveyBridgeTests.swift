import XCTest
@testable import Aircast

final class SurveyBridgeTests: XCTestCase {
    private func point(_ latitude: Double, _ longitude: Double) -> String { #"{"latitude":\#(latitude),"longitude":\#(longitude)}"# }

    private var threeCorners: [String] { [point(41.0, 44.0), point(41.0, 44.1), point(41.1, 44.1)] }

    private func survey(
        corners: [String]? = nil,
        transects: [String]? = nil,
        shots: Int = 12,
        kind: String = "survey",
        shape: String = "area",
        property: String = "surveyAreaPolygon"
    ) -> String {
        let vertices = (corners ?? threeCorners).joined(separator: ",")
        let lines = (transects ?? [point(41.0, 44.0), point(41.0, 44.1)]).joined(separator: ",")
        return #"{"kind":"\#(kind)","cameraShots":\#(shots),"geometry":{"shape":"\#(shape)","property":"\#(property)","vertices":[\#(vertices)],"transects":[\#(lines)]}}"#
    }

    private func plan(_ items: String...) -> JSON { JSON.parse(#"{"kind":"object","items":["# + items.joined(separator: ",") + "]}") }

    private func found(_ json: JSON?) -> [Survey] { SurveyBridge.surveysFrom(json) }

    func testASurveyCarriesItsAreaTransectsAndShotCount() {
        let surveys = found(plan(#"{"kind":"settings"}"#, survey()))
        XCTAssertEqual(1, surveys.count)
        XCTAssertEqual(1, surveys.first?.index)
        XCTAssertEqual(3, surveys.first?.area.count)
        XCTAssertEqual(2, surveys.first?.transects.count)
        XCTAssertEqual(12, surveys.first?.cameraShots)
    }

    func testItemsThatAreNotSurveysAreIgnored() {
        XCTAssertEqual(0, found(plan(#"{"kind":"waypoint"}"#, #"{"kind":"takeoff"}"#)).count)
    }

    func testACorridorScanIsALineNotAnAreaAndSaysWhichPropertyHoldsIt() {
        let surveys = found(plan(#"{"kind":"settings"}"#, survey(kind: "corridor", shape: "line", property: "corridorPolyline")))
        XCTAssertEqual(1, surveys.count)
        XCTAssertEqual(SHAPE_LINE, surveys.first?.shape)
        XCTAssertEqual("corridorPolyline", surveys.first?.property)
        XCTAssertEqual("corridor", surveys.first?.kind)
    }

    func testAStructureScanIsAnAreaDrawnFromItsOwnProperty() {
        let surveys = found(plan(survey(kind: "structure", property: "structurePolygon")))
        XCTAssertEqual(SHAPE_AREA, surveys.first?.shape)
        XCTAssertEqual("structurePolygon", surveys.first?.property)
    }

    func testGeometryWithoutAPropertyCannotBeDraggedSoItIsNotShown() {
        let noProperty = #"{"kind":"corridor","geometry":{"shape":"line","vertices":["# + threeCorners.joined(separator: ",") + #"],"transects":[]}}"#
        XCTAssertEqual(0, found(plan(noProperty)).count)
    }

    func testASurveyTheCoreGaveNoGeometryIsNotShown() {
        XCTAssertEqual(0, found(plan(#"{"kind":"survey","cameraShots":3}"#)).count)
    }

    func testTransectsThatHaveNotArrivedYetLeaveTheAreaDrawable() {
        let surveys = found(plan(survey(transects: [])))
        XCTAssertEqual(3, surveys.first?.area.count)
        XCTAssertEqual(0, surveys.first?.transects.count)
    }

    func testUnusablePointsAreDroppedFromTheAreaAndTransects() {
        let surveys = found(plan(survey(transects: [point(41.0, 44.0), point(0.0, 0.0), point(41.2, 44.2)])))
        XCTAssertEqual(2, surveys.first?.transects.count)
    }

    func testAnAreaNeedsThreeCornersBeforeItIsDrawn() {
        let thin = survey(corners: [point(41.0, 44.0), point(41.0, 44.1)])
        XCTAssertEqual(0, surveyAreaFeatures(found(plan(thin))).shapes.count)
    }

    func testTransectsNeedTwoPointsBeforeTheyAreDrawn() {
        let single = survey(transects: [point(41.0, 44.0)])
        XCTAssertEqual(0, surveyTransectFeatures(found(plan(single))).shapes.count)
    }

    func testAnAbsentPlanYieldsNothing() {
        XCTAssertEqual(0, found(nil).count)
    }
}
