import XCTest
@testable import Aircast

final class PlanSummaryTests: XCTestCase {
    private func item(_ index: Int = 0, altitude: Double = .nan) -> MissionItem {
        MissionItem(index: index, sequence: index + 1, stop: index - 1, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: altitude, altitudeText: altitude.isNaN ? "" : "\(Int(altitude)) m")
    }

    func testASelectedWaypointShowsItsAltitudeAgainstTheNumberOnItsMarker() {
        XCTAssertEqual(
            "Waypoint 2 at 75 m",
            selectionText(.Waypoint(index: 3), [item(3, altitude: 75.0)], [], []),
            "the map marker and the list row are labelled with the waypoint number, so naming the index or the mission sequence here would point at a different item"
        )
    }

    func testASelectedCircleShowsItsRadiusFromEitherHandleSpelledByTheCore() {
        let circles = [FenceCircle(index: 1, inclusion: true, centre: TrackPoint(41.0, 44.0), radius: 136.0, detailText: "446 ft radius")]
        XCTAssertEqual("446 ft radius", selectionText(.Circle(index: 1), [], circles, []))
        XCTAssertEqual("446 ft radius", selectionText(.CircleCentre(index: 1), [], circles, []))
    }

    func testASelectionWithNothingToSayAddsNothing() {
        XCTAssertNil(selectionText(.Waypoint(index: 0), [item()], [], []))
    }
}
