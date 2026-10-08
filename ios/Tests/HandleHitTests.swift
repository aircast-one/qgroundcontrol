import XCTest
@testable import Aircast

final class HandleHitTests: XCTestCase {
    func testEachHandleKindGivesItsOwnHit() {
        XCTAssertEqual(MapHit.FenceVertex(polygon: 1, vertex: 2), handleHit(HANDLE_KIND_FENCE, 1, 2))
        XCTAssertEqual(MapHit.SurveyVertex(item: 1, vertex: 2), handleHit(HANDLE_KIND_SURVEY, 1, 2))
        XCTAssertEqual(MapHit.CircleCentre(index: 1), handleHit(HANDLE_KIND_CIRCLE, 1, 2))
    }

    func testAHandleKindNobodyKnowsIsNotAFenceVertex() {
        XCTAssertNil(handleHit("corridor", 1, 2))
        XCTAssertNil(handleHit(nil, 1, 2))
        XCTAssertNil(handleHit("", 1, 2))
    }
}

final class WithinTapTests: XCTestCase {
    func testAFingerThatBarelyMovesIsATap() {
        XCTAssertTrue(withinTap(0, 0))
        XCTAssertTrue(withinTap(TAP_SLOP_PX, -TAP_SLOP_PX))
    }

    func testAFingerThatTravelsIsAPanAndMustNotClearTheSelection() {
        XCTAssertFalse(withinTap(TAP_SLOP_PX + 1, 0))
        XCTAssertFalse(withinTap(0, TAP_SLOP_PX + 1))
    }
}
