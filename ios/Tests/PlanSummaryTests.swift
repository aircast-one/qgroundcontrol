import XCTest
@testable import Aircast

final class PlanSummaryTests: XCTestCase {
    private func item(_ index: Int = 0, altitude: Double = .nan) -> MissionItem {
        MissionItem(index: index, sequence: index + 1, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: altitude, altitudeText: altitude.isNaN ? "" : "\(Int(altitude)) m")
    }

    private func summary(
        items: [MissionItem] = [],
        circles: [FenceCircle] = [],
        summaryText: String = "",
        selected: MapHit? = nil,
        itemCount: Int? = nil,
        shape: [String] = [],
        offline: Bool = true
    ) -> String {
        planSummary(itemCount ?? items.count, shape, items, [], circles, [], [], summaryText, selected, offline: offline)
    }

    func testWhatIsInThePlanButNotOnTheMapIsNamed() {
        XCTAssertTrue(summary(items: [item()], itemCount: 3, shape: ["takeoff", "RTL"]).hasPrefix("3 items (takeoff, RTL)"))
    }

    func testItemsThatCannotBeDrawnAreStillInThePlan() {
        XCTAssertTrue(summary(items: [item()], itemCount: 3).hasPrefix("3 items"))
    }

    func testAnEmptyPlanSaysHowToStartOne() {
        XCTAssertEqual("Empty plan · long press to add", summary())
    }

    func testWithAnAircraftConnectedAnEmptyPlanDoesNotImplyTheAircraftHasNone() {
        XCTAssertEqual("Empty plan · Download the aircraft's, or long press to add", summary(offline: false))
    }

    func testWithNoAircraftThereIsNothingToDownloadFromAndTheOriginalLineStands() {
        XCTAssertEqual("Empty plan · long press to add", summary(offline: true))
    }

    func testOnlyWhatThePlanActuallyHoldsIsListed() {
        XCTAssertEqual("2 items", summary(items: [item(0), item(1)]))
    }

    func testCountsReadAsSingularWhenThereIsOne() {
        XCTAssertEqual("1 item", summary(items: [item()]))
    }

    func testCostIsAppendedWhenTheControllerHasWorkedItOut() {
        XCTAssertEqual("1 item · 2.00 km · 4:00", summary(items: [item()], summaryText: "2.00 km · 4:00"))
    }

    func testASelectedWaypointShowsItsAltitudeAgainstTheNumberOnItsMarker() {
        let text = summary(items: [item(3, altitude: 75.0)], selected: .Waypoint(index: 3))
        XCTAssertTrue(text.hasSuffix("#4 at 75 m"), text)
    }

    func testASelectedCircleShowsItsRadiusFromEitherHandleSpelledByTheCore() {
        let circles = [FenceCircle(index: 1, inclusion: true, centre: TrackPoint(41.0, 44.0), radius: 136.0, detailText: "446 ft radius")]
        XCTAssertTrue(summary(circles: circles, selected: .Circle(index: 1)).hasSuffix("446 ft radius"))
        XCTAssertTrue(summary(circles: circles, selected: .CircleCentre(index: 1)).hasSuffix("446 ft radius"))
    }

    func testASelectionWithNothingToSayAddsNothing() {
        XCTAssertEqual("1 item", summary(items: [item()], selected: .Waypoint(index: 0)))
    }
}
