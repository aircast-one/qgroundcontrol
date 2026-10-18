import XCTest
@testable import Aircast

final class MissionCompleteDialogTests: XCTestCase {
    func testAnOpenNoticeCarriesWhatTheDialogOffers() {
        let notice = missionComplete(JSON.parse(#"{"open":true,"id":3,"imagesTaken":12,"resumeFromWaypoint":4,"batteryWarning":true}"#))!
        XCTAssertEqual(notice.id, 3)
        XCTAssertEqual(notice.resumeFromWaypoint, 4)
        XCTAssertTrue(notice.batteryWarning)
        XCTAssertEqual(imagesTakenText(notice.imagesTaken), "12 Images Taken")
    }

    func testAClosedNoticeOrNoPhotosShowsNothing() {
        XCTAssertNil(missionComplete(JSON.parse(#"{"open":false,"id":3}"#)))
        XCTAssertNil(missionComplete(JSON.parse(#"{"open":true,"id":1,"resumeFromWaypoint":null}"#))!.resumeFromWaypoint)
        XCTAssertNil(imagesTakenText(0))
    }

    func testAnInterruptedMissionSaysSoAndNamesWhereItResumes() throws {
        let notice = try XCTUnwrap(missionComplete(JSON.parse(#"{"open":true,"id":2,"imagesTaken":0,"resumeFromWaypoint":5,"batteryWarning":true,"interrupted":true,"title":"Mission interrupted","detail":"2 drops are still to come.","resumeLabel":"Resume from waypoint 4"}"#)))
        XCTAssertEqual([notice.title, notice.resumeLabel], ["Mission interrupted", "Resume from waypoint 4"])
        XCTAssertTrue(notice.interrupted)
        XCTAssertEqual(missionComplete(JSON.parse(#"{"open":true,"id":3,"resumeFromWaypoint":null}"#))?.title, "Flight plan complete")
    }
}
