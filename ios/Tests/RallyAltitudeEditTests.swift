import XCTest
@testable import Aircast

final class RallyAltitudeEditTests: XCTestCase {
    private let imperial = JSON.parse(#"{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":164.0,"altitudeUnits":"ft","altitudeText":"164 ft","altitudeMetres":50.0,"altitudePath":"plan.rallyPointController.points.0.textFieldFacts.2"}]}"#)

    private func only(_ view: JSON) throws -> RallyPoint {
        let points = rallyPoints(view)
        XCTAssertEqual(1, points.count)
        return try XCTUnwrap(points.first)
    }

    func testTheEditedNumberIsTheOperatorsUnitNeverTheMetresUsedForADrag() throws {
        let point = try only(imperial)
        XCTAssertEqual(164.0, point.altitude, accuracy: 1e-9)
        XCTAssertEqual(50.0, point.altitudeMetres, accuracy: 1e-9)
        XCTAssertNotEqual(point.altitude, point.altitudeMetres)
        XCTAssertEqual("164", altitudeFieldText(point.altitude, RALLY_ALTITUDE_DECIMALS))
    }

    func testTheLabelCarriesTheServedUnitNotAHardcodedMetre() throws {
        XCTAssertEqual("Alt ft", rallyAltitudeLabel(try only(imperial)))
    }

    func testTheFieldWritesToTheFactPathWhichConvertsNotToTheCoordinate() throws {
        let point = try only(imperial)
        XCTAssertEqual("plan.rallyPointController.points.0.textFieldFacts.2", point.altitudePath)
        XCTAssertTrue(point.altitudePath.hasSuffix("textFieldFacts.2"))
    }

    func testAPointWhoseAltitudeIsNotAFactOffersNoField() throws {
        let notAFact = JSON.parse(#"{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":null,"altitudeUnits":"","altitudeMetres":50.0,"altitudePath":""}]}"#)
        XCTAssertFalse(rallyAltitudeIsEditable(try only(notAFact)))
    }

    func testACoreTooOldToServeThePathOffersNoFieldRatherThanWritingNowhere() throws {
        let old = JSON.parse(#"{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitudeMetres":50.0}]}"#)
        XCTAssertFalse(rallyAltitudeIsEditable(try only(old)))
    }

    func testAnAltitudeWithNowhereToWriteItOffersNoField() throws {
        let noPath = JSON.parse(#"{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":164.0,"altitudeUnits":"ft","altitudeMetres":50.0}]}"#)
        XCTAssertFalse(rallyAltitudeIsEditable(try only(noPath)), "the path is the write target; a field that renders without one edits nothing and reports success")
    }

    func testAServedPointIsEditable() throws {
        XCTAssertTrue(rallyAltitudeIsEditable(try only(imperial)))
    }

    func testAMetricPointLabelsItselfInMetresWithoutAFallback() throws {
        let metric = JSON.parse(#"{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":50.0,"altitudeUnits":"m","altitudeMetres":50.0,"altitudePath":"plan.rallyPointController.points.0.textFieldFacts.2"}]}"#)
        let point = try only(metric)
        XCTAssertEqual("Alt m", rallyAltitudeLabel(point))
        XCTAssertEqual(point.altitude, point.altitudeMetres, accuracy: 1e-9)
    }
}
