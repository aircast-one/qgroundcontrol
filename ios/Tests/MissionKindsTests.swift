import XCTest
@testable import Aircast

final class MissionKindsTests: XCTestCase {
    func testAnAcceptedInsertCarriesNoComplaint() {
        let done = insertOutcome(JSON.parse(#"{"ok":true,"inserted":"survey","atSequence":4}"#))
        XCTAssertTrue(done.ok)
        XCTAssertEqual(done.reason, "")
    }

    func testARefusalIsShownInTheCoresWords() {
        let refused = insertOutcome(JSON.parse(#"{"ok":false,"reason":"The mission already takes off before this point.","refused":"takeoff"}"#))
        XCTAssertFalse(refused.ok)
        XCTAssertEqual(refused.reason, "The mission already takes off before this point.")
    }

    func testAKindTheCatalogueDoesNotHoldSaysSoRatherThanGoingQuiet() {
        let unknown = insertOutcome(JSON.parse(#"{"ok":false,"unknown":"corkscrew","reason":"the core has no corkscrew in its catalogue"}"#))
        XCTAssertFalse(unknown.ok)
        XCTAssertEqual(unknown.reason, "the core has no corkscrew in its catalogue")
    }

    func testARefusalWithNoReasonStillSaysSomething() {
        XCTAssertEqual(insertOutcome(JSON.parse(#"{"ok":false}"#)).reason, "The plan did not answer.")
        XCTAssertEqual(insertOutcome(nil).reason, "The plan did not answer.")
    }
}

final class InsertedIndexTests: XCTestCase {
    func testASuccessfulInsertSaysWhereTheItemLandedSoTheNextOneCanFollowIt() {
        XCTAssertEqual(insertOutcome(JSON.parse(#"{"ok":true,"inserted":"waypoint","index":3}"#)).index, 3)
    }

    func testAnAnswerWithoutAnIndexDoesNotInventOne() {
        XCTAssertNil(insertOutcome(JSON.parse(#"{"ok":true}"#)).index)
    }

    func testARefusalCarriesNoIndex() {
        XCTAssertNil(insertOutcome(JSON.parse(#"{"ok":false,"reason":"no"}"#)).index)
    }
}

final class RemoveRefusalTests: XCTestCase {
    func testARefusalCarriesTheCoresSentenceRatherThanAGenericFailure() {
        let refused = insertOutcome(JSON.parse(#"{"ok":false,"reason":"The first entry holds the plan's own settings and cannot be removed."}"#))
        XCTAssertEqual(refused.reason, "The first entry holds the plan's own settings and cannot be removed.")
    }

    func testARemovalThatHappenedCarriesNoReasonToShow() {
        let done = insertOutcome(JSON.parse(#"{"ok":true,"removed":3,"remaining":6}"#))
        XCTAssertTrue(done.ok)
        XCTAssertEqual(done.reason, "")
    }

    func testAnAnswerWithNoReasonStillSaysSomething() {
        XCTAssertEqual(insertOutcome(JSON.parse(#"{"ok":false}"#)).reason, "The plan did not answer.")
        XCTAssertEqual(insertOutcome(nil).reason, "The plan did not answer.")
    }

    func testTheLandButtonCarriesTheTitleTheCoreServes() {
        let kinds = missionKinds(JSON.parse(#"{"kinds":[{"id":"land","title":"Return","enabled":true}]}"#))
        XCTAssertEqual(kindLabel(kinds, "land"), "Return")
        XCTAssertEqual(kindLabel([], "land"), "Land")
    }
}
