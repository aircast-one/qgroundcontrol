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

    func testAStoppedMissionSaysWhereItStoppedAndNamesWhereItResumes() throws {
        let notice = try XCTUnwrap(missionComplete(JSON.parse(#"{"open":true,"id":2,"imagesTaken":0,"resumeFromWaypoint":48,"batteryWarning":true,"interrupted":true,"title":"Mission stopped at survey 2 of 3","detail":"2 drops are still to come.","resumeLabel":"Resume from the survey (2 of 3), at its waypoint 14 of 40"}"#)))
        XCTAssertEqual([notice.title, notice.resumeLabel, notice.detail], ["Mission stopped at survey 2 of 3", "Resume from the survey (2 of 3), at its waypoint 14 of 40", "2 drops are still to come."], "the button reads the core label, which names the plan row as the title does")
        XCTAssertEqual(notice.resumeFromWaypoint, 48)
        XCTAssertEqual(missionComplete(JSON.parse(#"{"open":true,"id":3,"resumeFromWaypoint":null}"#))?.title, "Flight plan complete")
    }
}
