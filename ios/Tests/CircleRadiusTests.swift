import XCTest
@testable import Aircast

final class CircleRadiusTests: XCTestCase {
    private func circle(_ radius: Double, minimum: Double? = nil, maximum: Double? = nil, metres: Double? = nil) -> FenceCircle {
        FenceCircle(index: 0, inclusion: true, centre: TrackPoint(0.0, 0.0), radius: radius, radiusMinimum: minimum, radiusMaximum: maximum, radiusMetres: metres ?? radius)
    }

    func testACircleTheFactDoesNotBoundGrowsAndShrinksFreely() throws {
        XCTAssertEqual(150.0, try XCTUnwrap(grownRadius(circle(100.0))), accuracy: 1e-9)
        XCTAssertEqual(100.0 / 1.5, try XCTUnwrap(shrunkRadius(circle(100.0))), accuracy: 1e-9)
    }

    func testAStepPastTheCeilingLandsOnTheCeilingRatherThanBeyondIt() throws {
        XCTAssertEqual(120.0, try XCTUnwrap(grownRadius(circle(100.0, maximum: 120.0))), accuracy: 1e-9)
    }

    func testACircleAlreadyAtTheCeilingCannotGrowSoTheControlGoesDead() {
        XCTAssertNil(grownRadius(circle(120.0, maximum: 120.0)))
        XCTAssertNil(grownRadius(circle(200.0, maximum: 120.0)))
    }

    func testAStepPastTheFloorLandsOnTheFloor() throws {
        XCTAssertEqual(30.0, try XCTUnwrap(shrunkRadius(circle(40.0, minimum: 30.0))), accuracy: 1e-9)
    }

    func testACircleAlreadyAtTheFloorCannotShrink() {
        XCTAssertNil(shrunkRadius(circle(30.0, minimum: 30.0)))
        XCTAssertNil(shrunkRadius(circle(10.0, minimum: 30.0)))
    }

    func testAnUnboundedCircleStillRefusesToShrinkToNothing() {
        XCTAssertNil(shrunkRadius(circle(0.0)))
    }

    func testTheBoundsAreReadFromTheServedCircle() throws {
        let decoded = fenceCircles(JSON.parse(#"{"circles":[{"index":0,"inclusion":true,"radius":100.0,"centre":{"latitude":1.0,"longitude":2.0},"radiusMinimum":30.0,"radiusMaximum":120.0}]}"#))
        XCTAssertEqual(30.0, try XCTUnwrap(decoded[0].radiusMinimum), accuracy: 1e-9)
        XCTAssertEqual(120.0, try XCTUnwrap(decoded[0].radiusMaximum), accuracy: 1e-9)
    }

    func testACircleServedWithoutBoundsDecodesToNoneRatherThanToZero() {
        let decoded = fenceCircles(JSON.parse(#"{"circles":[{"index":0,"inclusion":true,"radius":100.0,"centre":{"latitude":1.0,"longitude":2.0},"radiusMinimum":null,"radiusMaximum":null}]}"#))
        XCTAssertNil(decoded[0].radiusMinimum)
        XCTAssertNil(decoded[0].radiusMaximum)
    }

    func testTheBoundsAreInTheSameUnitsAsTheRadiusNotMetres() throws {
        let feet = circle(328.084, minimum: 30.0, maximum: 1200.0, metres: 100.0)
        XCTAssertEqual(492.126, try XCTUnwrap(grownRadius(feet)), accuracy: 1e-3, "the ceiling is 1200 ft and 328 * 1.5 fits under it")
        XCTAssertEqual(218.723, try XCTUnwrap(shrunkRadius(feet)), accuracy: 1e-3, "the floor is 30 ft and 328 / 1.5 clears it")
        XCTAssertNil(grownRadius(circle(1200.0, maximum: 1200.0, metres: 365.76)), "already at the ceiling in its own units")
        XCTAssertEqual(30.0, try XCTUnwrap(shrunkRadius(circle(35.0, minimum: 30.0, metres: 10.67))), accuracy: 1e-3, "clamped to the floor rather than to 3.28 times it")
    }

    func testAMetricRigCannotTellTheDifferenceWhichIsWhyThisSurvived() throws {
        let metric = circle(100.0, minimum: 30.0, maximum: 120.0, metres: 100.0)
        let imperial = circle(328.084, minimum: 98.425, maximum: 393.701, metres: 100.0)
        XCTAssertEqual(120.0, try XCTUnwrap(grownRadius(metric)), accuracy: 1e-3)
        XCTAssertEqual(393.701, try XCTUnwrap(grownRadius(imperial)), accuracy: 1e-3, "the same fence in feet: the ceiling is 393.7 ft, not 393.7 times 3.28")
    }

    func testACircleWhoseMetresNeverResolvedStillBoundsByItsOwnNumbers() throws {
        XCTAssertEqual(150.0, try XCTUnwrap(grownRadius(circle(100.0, maximum: 150.0, metres: 0.0))), accuracy: 1e-3)
    }
}
