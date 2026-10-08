import XCTest
@testable import Aircast

final class FlyDeckTests: XCTestCase {
    private func ids(_ deck: [(String, Bool)]) -> [String] { deck.map(\.0) }
    private func primaries(_ deck: [(String, Bool)]) -> [Bool] { deck.map(\.1) }

    func testGroundOffersChecklistAndTakeoffAsPrimary() {
        let deck = deckIds([CHECKLIST, "arm", "takeoff", "changeSpeed"], false)
        XCTAssertEqual(ids(deck), [CHECKLIST, "takeoff"])
        XCTAssertEqual(primaries(deck), [false, true])
    }

    func testGroundWithoutTakeoffArmsAsPrimary() {
        let deck = deckIds(["arm"], false)
        XCTAssertEqual(ids(deck), ["arm"])
        XCTAssertEqual(primaries(deck), [true])
    }

    func testFlyingOffersPauseReturnLandWithReturnPrimary() {
        let deck = deckIds(["arm", PAUSE, "rtl", "land", "changeAltitude"], true)
        XCTAssertEqual(ids(deck), [PAUSE, "rtl", "land"])
        XCTAssertEqual(primaries(deck), [false, true, false])
    }

    func testArmedOnTheGroundKeepsTheGroundDeck() {
        let deck = deckIds(["arm", "takeoff"], true)
        XCTAssertEqual(ids(deck), ["takeoff"])
        XCTAssertEqual(primaries(deck), [true])
    }

    func testTheRailLabelsDropTheHoldInstruction() {
        XCTAssertEqual(["Hold to land", "Return", "Hold to take off"].map(railLabel), ["Land", "Return", "Take off"])
    }
}
