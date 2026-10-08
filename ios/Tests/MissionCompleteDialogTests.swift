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
}
