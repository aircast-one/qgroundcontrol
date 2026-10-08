import XCTest
@testable import Aircast

final class RallyAltitudeTests: XCTestCase {
    private let served = JSON.parse(#"{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitudeMetres":50.0},{"index":1,"latitude":47.398,"longitude":8.546,"altitudeMetres":120.0}]}"#)

    func testARallyPointCarriesTheHeightTheBridgeWillWriteBack() {
        let points = rallyPoints(served)
        XCTAssertEqual(50.0, points[0].altitudeMetres, accuracy: 1e-9)
        XCTAssertEqual(120.0, points[1].altitudeMetres, accuracy: 1e-9)
    }

    func testACoreTooOldToServeTheHeightWritesZeroRatherThanCrashing() {
        let old = JSON.parse(#"{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545}]}"#)
        XCTAssertEqual(0.0, rallyPoints(old)[0].altitudeMetres, accuracy: 1e-9)
    }

    func testTheCoordinateWrittenCarriesTheAltitudeInMetres() {
        let json = coordinateJson(47.397, 8.545, 50.0)
        XCTAssertEqual(50.0, json["altitude"], "altitude 50.0 is what the bridge reads as metres")
        XCTAssertEqual(47.397, json["latitude"])
    }

    func testThePayloadADragWritesKeepsThePointsOwnHeight() {
        XCTAssertEqual(["latitude": 47.398, "longitude": 8.546, "altitude": 120.0], rallyMovePayload(47.398, 8.546, 120.0))
    }

    func testTheHeightComesFromThePointBeingDraggedNotTheFirstOne() {
        let points = rallyPoints(served)
        XCTAssertEqual(120.0, rallyAltitudeFor(points, 1), accuracy: 1e-9)
        XCTAssertEqual(50.0, rallyAltitudeFor(points, 0), accuracy: 1e-9)
        XCTAssertEqual(0.0, rallyAltitudeFor(points, 7), accuracy: 1e-9)
    }

    func testACallerThatDoesNotCareStillGetsTheOldZero() {
        XCTAssertEqual(0.0, coordinateJson(47.397, 8.545)["altitude"])
    }
}
