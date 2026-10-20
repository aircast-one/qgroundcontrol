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

    private func deck(_ offers: [GuidedOffer], armed: Bool = false, openChecklist: (() -> Void)? = nil, opened: Opened = Opened(), readiness: Readiness? = nil, dropBay: Int = 0, dropLabel: String = "", confirm: @escaping (GuidedAction) -> Void = { _ in }) -> FlightDeckContext {
        FlightDeckContext(
            offers: Dictionary(offers.map { ($0.id, $0) }, uniquingKeysWith: { _, last in last }),
            armed: armed,
            confirm: confirm,
            openValue: { opened.ids.append($0.offerId) },
            report: { _ in },
            openChecklist: openChecklist,
            readiness: readiness,
            dropBay: dropBay,
            dropLabel: dropLabel
        )
    }

    func testADropConfirmedFromTheDeckWaitsForTheCoreToAnswerSoARefusedDropIsNeverReportedAsSent() throws {
        let release = GuidedOffer(id: RELEASE, title: "Drop", offer: "ready", reason: "", prompt: "", destructive: false, carriesValue: false)
        var confirmed: [GuidedAction] = []
        try XCTUnwrap(flightDeckEntries(deck([release], armed: true, confirm: { confirmed.append($0) })).first { $0.id == RELEASE }).onClick()
        XCTAssertNotNil(confirmed.first?.answer)
    }

    func testAGripperGetsADropOnTheDeckThatLandsOnAHoldAndNamesTheBay() throws {
        let release = GuidedOffer(id: RELEASE, title: "Drop", offer: "ready", reason: "", prompt: "", destructive: false, carriesValue: false)
        let single = try XCTUnwrap(flightDeckEntries(deck([release], armed: true)).first { $0.id == RELEASE })
        XCTAssertEqual(single.label, holdLabel("Drop"))
        XCTAssertTrue(single.onHold != nil && single.enabled)
        let bay2 = GuidedOffer(id: RELEASE, title: "Drop bay 2", offer: "ready", reason: "", prompt: "", destructive: false, carriesValue: false)
        XCTAssertEqual(flightDeckEntries(deck([bay2], armed: true, dropBay: 2, dropLabel: "Drop 2")).first { $0.id == RELEASE }?.label, holdLabel("Drop 2"))
    }

    func testEveryBayDroppedTheDeckStillOffersBayOneButSaysTheCoreCountsItEmpty() throws {
        let release = GuidedOffer(id: RELEASE, title: "Drop bay 1", offer: "ready", reason: "", prompt: "Bay 1 is counted empty. Drop it only if you reloaded it.", destructive: false, carriesValue: false)
        var confirmed: [GuidedAction] = []
        let emptied = try XCTUnwrap(flightDeckEntries(deck([release], armed: true, dropBay: 1, dropLabel: "Drop 1 · empty", confirm: { confirmed.append($0) })).first { $0.id == RELEASE })
        XCTAssertEqual(emptied.label, "Slide to drop 1 · empty")
        XCTAssertTrue(emptied.enabled)
        emptied.onClick()
        XCTAssertEqual(confirmed.first?.name, "Drop bay 1")
        XCTAssertEqual(confirmed.first?.confirm, "Bay 1 is counted empty. Drop it only if you reloaded it.")
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
