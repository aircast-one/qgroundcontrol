import XCTest
@testable import Aircast

final class PlanStripTests: XCTestCase {
    private func item(_ index: Int, _ altitude: Double = .nan) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: true, altitude: altitude, altitudeText: altitude.isNaN ? "" : "\(Int(altitude)) m")
    }

    private let summary = JSON.parse(#"{"rows":[{"label":"Distance","value":"1.2 km"},{"label":"Time","value":"00:04:30"},{"label":"Furthest from launch","value":"640 m"}]}"#)

    func testTheStripReadsItemsTheCoresDistanceAndTimeAndTheHighestAltitude() {
        let items = [item(0, 488.0), item(1, 30.0), item(2, 80.0), item(3, 50.0)]
        XCTAssertEqual(
            [PlanStat(label: "Items", value: "3"), PlanStat(label: "Distance", value: "1.2 km"), PlanStat(label: "Time", value: "04:30"), PlanStat(label: "Max alt", value: "80 m")],
            planStats(3, items, summary),
            "the planned home sits at ground height above sea level, so counting it would report the launch site's elevation as the plan's ceiling"
        )
    }

    func testAFlightUnderAnHourDropsTheEmptyHoursSoTheTimeFitsTheStrip() {
        let long = JSON.parse(#"{"rows":[{"label":"Time","value":"01:04:30"}]}"#)
        XCTAssertEqual(PlanStat(label: "Time", value: "01:04:30"), planStats(0, [], long)[1])
    }

    func testAStatTheCoreHasNotWorkedOutIsLeftOffRatherThanShownAsZero() {
        XCTAssertEqual([PlanStat(label: "Items", value: "1")], planStats(1, [item(1)], nil))
        XCTAssertNil(highestAltitude([item(0, 488.0)]))
    }

    func testASelectionThatIsNotAMissionItemIsNamedForWhatItIs() {
        let items = [MissionItem(index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: "Survey", selected: true)]
        XCTAssertEqual(
            ["Fence corner", "Circular fence", "Rally point 3", "Breach return point", "Survey corner"],
            [MapHit.FenceVertex(polygon: 0, vertex: 1), .CircleRadius(index: 0), .Rally(index: 2), .BreachReturn, .SurveyVertex(item: 2, vertex: 0)].map { selectionTitle($0, items) }
        )
    }
}
