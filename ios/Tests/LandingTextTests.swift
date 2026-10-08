import XCTest
@testable import Aircast

final class LandingTextTests: XCTestCase {
    private func pattern(_ radius: String, clockwise: Bool = true, index: Int = 3) -> LandingPattern {
        LandingPattern(
            index: index,
            landing: TrackPoint(latitude: 41.0, longitude: 44.0),
            slopeStart: nil,
            finalApproach: nil,
            loiterRadiusMetres: 75.0,
            loiterClockwise: clockwise,
            loiterRadiusText: radius
        )
    }

    func testADrawnCircleSaysHowWideItIsAndWhichWayRound() {
        XCTAssertEqual(landingText(pattern("75.0 m")), "circles 75.0 m clockwise")
        XCTAssertEqual(landingText(pattern("75.0 m", clockwise: false)), "circles 75.0 m anticlockwise")
    }

    func testAPatternTheCoreGaveNoRadiusForSaysNothingRatherThanCirclingZero() {
        XCTAssertNil(landingText(pattern("")))
        XCTAssertNil(landingText(nil))
    }

    func testALandingPickedFromTheListIsTheSameOneAsPickedByItsTouchdown() {
        let landings = [pattern("75.0 m", index: 3)]
        XCTAssertEqual(selectedLanding(.Waypoint(index: 3), landings)?.index, 3)
        XCTAssertEqual(selectedLanding(.LandingPlace(index: 3, place: LANDING_PLACE_TOUCHDOWN), landings), selectedLanding(.Waypoint(index: 3), landings))
        XCTAssertNil(selectedLanding(.Waypoint(index: 9), landings))
        XCTAssertNil(selectedLanding(nil, landings))
    }
}
