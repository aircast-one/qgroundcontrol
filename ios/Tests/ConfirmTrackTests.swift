import XCTest
@testable import Aircast

final class ConfirmTrackTests: XCTestCase {
    func testSentStandsUntilTheServedActionsChange() {
        XCTAssertTrue(sentIsStillShowing("Land", "{A}", "{A}"))
        XCTAssertFalse(sentIsStillShowing("Land", "{A}", "{B}"))
    }

    func testNothingWasSentMeansNothingIsShown() {
        XCTAssertFalse(sentIsStillShowing(nil, "{A}", "{A}"))
        XCTAssertFalse(sentIsStillShowing("Land", nil, "{A}"))
    }

    func testASendRecordedAgainstNoReadingDoesNotStick() {
        XCTAssertFalse(sentIsStillShowing("Land", nil, nil))
    }

    func testTheNoticeNamesTheActionThatWasSent() {
        XCTAssertEqual(sentText("Return"), "Sent · Return")
        XCTAssertEqual(sentText("Emergency Stop"), "Sent · Emergency Stop")
    }

    func testTheLandConfirmReadsTheHeightItLandsFromAndNothingWhenTheVehicleHasNotSaid() {
        let from = landFrom(JSON.parse(#"{"items":[{"missing":false,"value":"42.0","units":"m"}]}"#))
        XCTAssertEqual(from?.0, "42.0")
        XCTAssertEqual(from?.1, "m")
        XCTAssertNil(landFrom(JSON.parse(#"{"items":[{"missing":true,"value":"—","units":""}]}"#)))
        XCTAssertNil(landFrom(nil))
    }

    func testStartingAMissionNamesTheMissionTheDroneWillFly() {
        let summary = JSON.parse(#"{"rows":[{"id":"distance","label":"Distance","value":"1.40 km"},{"id":"time","label":"Time","value":"6 min"}]}"#)
        let uploaded = JSON.parse(#"{"hasMissionItems":true,"dirty":false,"file":"ridge.plan","status":"Uploaded · 12 items"}"#)
        XCTAssertEqual(missionIdentity(uploaded, summary), MissionIdentity(title: "ridge", facts: "Uploaded · 12 items · 1.40 km · 6 min", warning: nil))
        let edited = JSON.parse(#"{"hasMissionItems":true,"dirty":true,"file":"ridge.plan","status":"Edited · 13 items"}"#)
        XCTAssertEqual(missionIdentity(edited, summary).title, MISSION_ON_DRONE)
        XCTAssertTrue(missionIdentity(edited, summary).warning!.hasPrefix("ridge has changes"))
        XCTAssertEqual(missionIdentity(JSON.parse(#"{"hasMissionItems":false}"#), summary).title, MISSION_ON_DRONE)
        XCTAssertEqual(holdLabel("Start mission"), "Hold to start mission")
        XCTAssertEqual(holdLabel("Land"), "Hold to land")
        XCTAssertEqual(holdLabel(""), "Hold to confirm")
        XCTAssertFalse(MISSION_ACTIONS.contains("land"))
    }
}
