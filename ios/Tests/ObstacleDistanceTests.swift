import XCTest
@testable import Aircast

final class ObstacleDistanceTests: XCTestCase {
    func testAReadingWhoseAgeTheSensorNeverReportedIsStillDrawnDeliberately() {
        let ageless = JSON.parse(#"{"available":true,"stale":null,"nearest":{"distanceText":"4 m","sectorText":"ahead","close":true}}"#)
        XCTAssertEqual(
            obstacleWarning(ageless)?.label,
            "4 m ahead",
            "obstacle.rs stale is since.map(..), so null means the sensor never said how old the reading is; a reading must never be suppressed"
        )
    }

    func testAReadingTheSensorCalledStaleIsWithheld() {
        let old = JSON.parse(#"{"available":true,"stale":true,"nearest":{"distanceText":"4 m","sectorText":"ahead","stale":true}}"#)
        XCTAssertNil(obstacleWarning(old), "the head reads the flag on the object whose text it is about to show")
    }

    private func view(available: Bool = true, stale: Bool = false, nearest: String? = nil) -> JSON {
        let shown = nearest ?? #"{"distanceText":"3.2 m","sectorText":"right","close":false,"stale":\#(stale)}"#
        return JSON.parse(#"{"available":\#(available),"stale":\#(stale),"nearest":\#(shown)}"#)
    }

    func testAReadingIsTheCoresDistanceAndTheCoresSector() throws {
        let warning = try XCTUnwrap(obstacleWarning(view()))
        XCTAssertEqual(warning.label, "3.2 m right")
        XCTAssertTrue(!warning.close)
    }

    func testTheDistanceIsWhateverTheCoreSpelledSoItFollowsTheOperatorsUnits() {
        let feet = view(nearest: #"{"distanceText":"10.5 ft","sectorText":"ahead","close":true}"#)
        XCTAssertEqual(obstacleWarning(feet)?.label, "10.5 ft ahead")
    }

    func testACloseReadingSaysSoForTheHeadToColour() {
        let near = view(nearest: #"{"distanceText":"0.2 m","sectorText":"ahead","close":true}"#)
        XCTAssertEqual(obstacleWarning(near)?.close, true)
    }

    func testNothingIsShownWhenTheSensorIsAbsentStaleOrReportsNoObstacle() {
        XCTAssertNil(obstacleWarning(nil))
        XCTAssertNil(obstacleWarning(view(available: false)))
        XCTAssertNil(obstacleWarning(view(stale: true)))
        XCTAssertNil(obstacleWarning(view(nearest: "null")))
    }

    func testAReadingWithNoDistanceTextIsNotAWarning() {
        XCTAssertNil(obstacleWarning(view(nearest: #"{"sectorText":"right"}"#)))
    }
}
