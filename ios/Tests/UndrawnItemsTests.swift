import XCTest
@testable import Aircast

final class UndrawnItemsTests: XCTestCase {
    private func items(_ pairs: (String, String)...) -> [JSON] {
        pairs.map { kind, name in .object(["kind": .string(kind), "name": .string(name)]) }
    }

    func testTheTwoKindsTheMapCannotDrawAreTheTwoItWarnsAbout() {
        XCTAssertEqual(
            ["Orbit", "Unknown: 1234"],
            undrawnItemNames(items(("complex", "Orbit"), ("unreadable", "Unknown: 1234"))),
            "missionitems.rs kind() returns complex when by_class does not recognise the item's class, and unreadable when QGC calls it neither simple nor complex - those are the only two outside DRAWN_KINDS, and they are exactly what has no shape to draw"
        )
    }

    func testMissionStartIsDrawnSoNoPlanIsWarnedAboutItsOwnFirstRow() {
        XCTAssertEqual(
            [],
            undrawnItemNames(items(("settings", "Mission Start"), ("takeoff", "Takeoff"))),
            "every plan carries a settings item - live capture shows kind settings, name Mission Start - so leaving it out of DRAWN_KINDS would put a warning on every plan ever opened"
        )
    }

    func testEveryDrawableKindStaysOutOfTheWarning() {
        XCTAssertTrue(DRAWN_KINDS.allSatisfy { undrawnItemNames(items(($0, "Something"))).isEmpty })
    }

    func testOneNameIsNotRepeatedWhenSeveralItemsShareIt() {
        XCTAssertEqual(["Orbit"], undrawnItemNames(items(("complex", "Orbit"), ("complex", "Orbit"))))
    }

    func testAnUndrawnItemWithNoNameIsLeftOutOfTheSentenceEntirely() {
        XCTAssertEqual(
            [],
            undrawnItemNames(items(("complex", ""))),
            "deliberate: an out-of-date AAR yields items with no class and so no name, and naming nothing is better than naming it wrongly. The cost is that a plan whose undrawn items are ALL unnamed gets no warning at all, which is recorded rather than fixed"
        )
        XCTAssertNil(undrawnItemsWarning(undrawnItemNames(items(("complex", "")))))
    }

    func testTheSentenceSaysTheItemsAreStillFlownNotMerelyUndrawn() {
        XCTAssertEqual(
            "The map cannot draw Orbit. Those items are still in the plan and will still be flown.",
            undrawnItemsWarning(["Orbit"]),
            "a pilot who reads only 'the map cannot draw X' may take it as cosmetic; the item still counts in distance and duration and still uploads"
        )
        XCTAssertNil(undrawnItemsWarning([]))
    }
}
