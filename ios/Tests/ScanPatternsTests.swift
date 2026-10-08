import XCTest
@testable import Aircast

final class ScanPatternsTests: XCTestCase {
    func testTheEmptyPlanNamesTheItemThatCanActuallyBeAdded() {
        let blocked = planSummary(0, [], [], [], [], [], [], "", nil, offline: true, canAddByHand: false)
        let open = planSummary(0, [], [], [], [], [], [], "", nil, offline: true, canAddByHand: true)
        XCTAssertEqual("Empty plan \u{00b7} add a takeoff to start", blocked)
        XCTAssertEqual("Empty plan \u{00b7} long press to add", open)
    }

    private let served = JSON.parse(#"{"kind":"object","kinds":[{"id":"waypoint","simple":true,"complexName":null,"enabled":true},{"id":"roi","simple":true,"complexName":null,"enabled":true},{"id":"survey","simple":false,"complexName":"Survey","enabled":true,"disabledReason":""},{"id":"corridor","simple":false,"complexName":"Corridor Scan","enabled":true,"disabledReason":""},{"id":"structure","simple":false,"complexName":"Structure Scan","enabled":false,"disabledReason":"This mission starts from the ground, so a takeoff has to come first."}]}"#)

    func testOnlyThePatternsAreOfferedAndByTheCoresOwnNames() {
        let patterns = scanPatterns(missionKinds(served))
        XCTAssertEqual(["survey", "corridor", "structure"], patterns.map(\.id))
        XCTAssertEqual(["Survey", "Corridor Scan", "Structure Scan"], patterns.map(\.label))
    }

    func testAPatternThePlanWillNotTakeIsOfferedDisabledWithTheCoresReason() throws {
        let patterns = scanPatterns(missionKinds(served))
        let structure = try XCTUnwrap(patterns.first { $0.id == "structure" })
        XCTAssertFalse(structure.enabled)
        XCTAssertTrue(structure.disabledReason.contains("takeoff"))
        XCTAssertEqual(true, patterns.first { $0.id == "survey" }?.enabled)
    }

    func testAFourthPatternTheCoreAddsIsOfferedWithoutThisHeadBeingChanged() {
        let later = JSON.parse(#"{"kind":"object","kinds":[{"id":"spiral","simple":false,"complexName":"Spiral Scan","enabled":true}]}"#)
        XCTAssertEqual(["Spiral Scan"], scanPatterns(missionKinds(later)).map(\.label))
    }

    func testNoAnswerOffersNothingRatherThanAListThisHeadRemembers() {
        XCTAssertEqual([], scanPatterns(missionKinds(nil)))
        XCTAssertEqual([], scanPatterns(missionKinds(JSON.parse(#"{"kind":"null"}"#))))
    }
}

final class MissionKindGateTests: XCTestCase {
    private let empty = JSON.parse(#"{"kind":"object","kinds":[{"id":"takeoff","simple":true,"enabled":true,"disabledReason":""},{"id":"land","simple":true,"enabled":false,"disabledReason":"This mission starts from the ground, so a takeoff has to come first."},{"id":"roi","simple":true,"enabled":false,"disabledReason":"This mission starts from the ground, so a takeoff has to come first."}]}"#)

    func testOnlyTheKindThePlanWillTakeIsOffered() {
        let kinds = missionKinds(empty)
        XCTAssertTrue(kindAllows(kinds, "takeoff"))
        XCTAssertFalse(kindAllows(kinds, "land"))
        XCTAssertFalse(kindAllows(kinds, "roi"))
    }

    func testAKindTheCoreNeverMentionedStaysOfferedRatherThanDisappearingOnSilence() {
        XCTAssertTrue(kindAllows(missionKinds(empty), "spiral"))
        XCTAssertTrue(kindAllows([], "land"))
    }

    func testTheReasonIsShownOnceFromTheCoreRatherThanBesideEveryDeadControl() {
        XCTAssertEqual(true, blockedReason(missionKinds(empty))?.contains("takeoff has to come"))
        XCTAssertNil(blockedReason([]))
    }

    func testAKindTheCoreLeavesOutOfTheListIsNotOfferedLikeTheRoversTakeoff() {
        let kinds = [MissionKind(id: "waypoint", label: "", title: "Waypoint", enabled: true, disabledReason: "")]
        XCTAssertEqual([true, false, true], [kindOffered(kinds, "waypoint"), kindOffered(kinds, "takeoff"), kindOffered([], "takeoff")])
    }

    func testAnItemsAltitudeReferenceFollowsThePlanWideFrameLikeSimpleItemEditor() {
        let frames: [Int?] = [GLOBAL_FRAME_MIXED, GLOBAL_FRAME_RELATIVE, 2, nil]
        XCTAssertEqual([true, false, true, true], frames.map(itemReferenceShown))
        XCTAssertEqual([true, false, false, true], frames.map(itemReferenceSelectable))
    }
}
