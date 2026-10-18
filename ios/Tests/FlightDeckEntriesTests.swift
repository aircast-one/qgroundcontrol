import XCTest
@testable import Aircast

@MainActor
final class FlightDeckEntriesTests: XCTestCase {
    private func offer(_ id: String, state: String = "ready", carriesValue: Bool = true) -> GuidedOffer {
        GuidedOffer(id: id, title: "", offer: state, reason: "", prompt: "", destructive: false, carriesValue: carriesValue)
    }

    private final class Opened {
        var ids: [String] = []
    }

    private func deck(_ offers: [GuidedOffer], armed: Bool = false, openChecklist: (() -> Void)? = nil, opened: Opened = Opened(), readiness: Readiness? = nil, dropBay: Int = 0) -> FlightDeckContext {
        FlightDeckContext(
            offers: Dictionary(offers.map { ($0.id, $0) }, uniquingKeysWith: { _, last in last }),
            armed: armed,
            confirm: { _ in },
            openValue: { opened.ids.append($0.offerId) },
            report: { _ in },
            openChecklist: openChecklist,
            readiness: readiness,
            dropBay: dropBay
        )
    }

    func testAGripperGetsADropOnTheDeckThatLandsOnAHoldAndNamesTheBay() throws {
        let release = GuidedOffer(id: RELEASE, title: "Drop", offer: "ready", reason: "", prompt: "", destructive: false, carriesValue: false)
        let single = try XCTUnwrap(flightDeckEntries(deck([release], armed: true)).first { $0.id == RELEASE })
        XCTAssertEqual(single.label, holdLabel("Drop"))
        XCTAssertTrue(single.onHold != nil && single.enabled)
        XCTAssertEqual(flightDeckEntries(deck([release], armed: true, dropBay: 2)).first { $0.id == RELEASE }?.label, holdLabel("Drop 2"))
    }

    func testTheDeckShowsTheOffersTheVehicleShowsAndTheChecklistOnlyOnTheGroundWhenOffered() {
        let offers = [offer("arm"), offer("takeoff"), offer("rtl", state: "hidden"), offer("land", state: "disabled")]
        XCTAssertEqual(flightDeckEntries(deck(offers)).map(\.id), ["arm", "takeoff", "land"])
        XCTAssertEqual(flightDeckEntries(deck(offers, openChecklist: {})).map(\.id), ["arm", "takeoff", "land", CHECKLIST])
        XCTAssertEqual(flightDeckEntries(deck(offers, armed: true, openChecklist: {})).map(\.id), ["takeoff", "land"])
    }

    func testValueActionsOpenTheSharedGuidedValueFlow() {
        let opened = Opened()
        let entries = flightDeckEntries(deck([offer("takeoff"), offer("changeSpeed"), offer("changeAltitude"), offer(PAUSE)], opened: opened))
        ["takeoff", "changeSpeed", "changeAltitude", PAUSE].forEach { id in entries.first { $0.id == id }?.onClick() }
        XCTAssertEqual(opened.ids, ["takeoff", "changeSpeed", "changeAltitude", PAUSE])
    }

    func testAVehicleThatIsNotFullyReadyCannotBeHeldStraightIntoTheAirTheTapOpensTheWarningFirst() throws {
        let ready = try XCTUnwrap(flightDeckEntries(deck([offer("takeoff")])).first { $0.id == "takeoff" })
        XCTAssertEqual(ready.label, HOLD_TO_TAKE_OFF)
        XCTAssertTrue(ready.onHold != nil)
        let warned = try XCTUnwrap(flightDeckEntries(deck([offer("takeoff")], readiness: Readiness(text: "GPS turned off.", blocks: false))).first { $0.id == "takeoff" })
        XCTAssertEqual(warned.label, TAKE_OFF)
        XCTAssertFalse(warned.onHold != nil)
    }
}
