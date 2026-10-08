import XCTest
@testable import Aircast

final class TerrainWarningTests: XCTestCase {
    private func clearance(collides: Bool = false, metres: Double? = nil, text: String = "", complete: Bool = true) -> Clearance {
        Clearance(collides: collides, metres: metres, text: text, complete: complete)
    }

    func testARouteBelowTheGroundIsNamedWithHowFarBelow() {
        XCTAssertEqual("The route goes 150.0 m below the ground.", terrainWarning(clearance(collides: true, metres: -150.0, text: "150.0 m")))
    }

    func testARouteBelowTheGroundIsStillNamedWhenTheGroundIsOnlyPartlyKnown() {
        XCTAssertEqual("The route goes 12.0 m below the ground.", terrainWarning(clearance(collides: true, metres: -12.0, text: "12.0 m", complete: false)))
    }

    func testARouteThatClearsIsNotAnnouncedBecauseClearingIsTheOrdinaryCase() {
        XCTAssertNil(terrainWarning(clearance(metres: 50.0, text: "50.0 m")))
    }

    func testAClearanceMeasuredOverGroundThatIsOnlyPartlyKnownSaysNothing() {
        XCTAssertNil(terrainWarning(clearance(metres: 4.0, text: "4.0 m", complete: false)))
    }

    func testNoTerrainAtAllSaysNothingRatherThanGuessingEitherWay() {
        XCTAssertNil(terrainWarning(clearance(metres: nil, complete: false)))
        XCTAssertNil(terrainWarning(nil))
    }

    func testACollisionWithNoFigureStillWarnsRatherThanFallingSilent() {
        XCTAssertEqual("The route goes below the ground.", terrainWarning(clearance(collides: true, metres: nil, text: "")))
    }

    func testTheClearanceIsReadFromTheViewTheCoreServes() {
        let read = clearanceOf(JSON.parse(#"{"points":[],"hasCollision":true,"minClearanceMetres":-8.5,"clearanceText":"8.5 m","clearanceComplete":true}"#))
        XCTAssertEqual(true, read?.collides)
        XCTAssertEqual(-8.5, read?.metres ?? 0, accuracy: 1e-9)
        XCTAssertEqual("8.5 m", read?.text)
    }

    func testANullClearanceFromTheCoreReadsAsUnknownNotAsZero() {
        let view = JSON.parse(#"{"points":[],"hasCollision":false,"minClearanceMetres":null,"clearanceText":null,"clearanceComplete":false}"#)
        XCTAssertNil(clearanceOf(view)?.metres)
        XCTAssertEqual("", clearanceOf(view)?.text)
    }
}
