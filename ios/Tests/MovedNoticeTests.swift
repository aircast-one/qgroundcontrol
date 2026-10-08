import XCTest
@testable import Aircast

final class MovedNoticeTests: XCTestCase {
    private let items = [MissionItem(index: 4, sequence: 72, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: 50.0, kind: "waypoint")]

    func testAMovedWaypointIsNamedByTheNumberOnItsMarker() {
        XCTAssertEqual("Moved #72", movedText(.Waypoint(index: 4), items))
    }

    func testAnItemTheListNoLongerHoldsStillSaysSomethingHappened() {
        XCTAssertEqual("Moved an item", movedText(.Waypoint(index: 99), items))
    }

    func testEveryOtherKindOfHandleNamesWhatItMoved() {
        XCTAssertEqual("Moved a fence corner", movedText(.FenceVertex(polygon: 0, vertex: 2), items))
        XCTAssertEqual("Moved a survey corner", movedText(.SurveyVertex(item: 1, vertex: 0), items))
        XCTAssertEqual("Moved a rally point", movedText(.Rally(index: 0), items))
        XCTAssertEqual("Moved a fence circle", movedText(.CircleCentre(index: 0), items))
        XCTAssertEqual("Changed a fence radius", movedText(.Circle(index: 0), items))
    }
}
