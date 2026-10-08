import XCTest
@testable import Aircast

final class OperatorControlTests: XCTestCase {
    private func served(_ body: String) -> JSON { JSON.parse(#"{"kind":"object","class":"OperatorControl","# + body + "}") }

    private var unavailable: JSON {
        served(#""available":false,"known":false,"inControl":null,"holderSystemId":null,"takeoverAllowed":null,"systemManager":null,"reason":"No vehicle is connected.""#)
    }

    private var silent: JSON {
        served(#""available":true,"known":false,"inControl":null,"holderSystemId":null,"takeoverAllowed":null,"requestAllowed":true,"reason":"This vehicle has not said who is flying it.""#)
    }

    private func elsewhere(takeover: Bool, request: Bool = true) -> JSON {
        served(#""available":true,"known":true,"inControl":false,"holderSystemId":42,"takeoverAllowed":\#(takeover),"requestAllowed":\#(request),"reason":"Another ground station is flying this vehicle.""#)
    }

    private var ours: JSON {
        served(#""available":true,"known":true,"inControl":true,"holderSystemId":255,"takeoverAllowed":true,"requestAllowed":true,"reason":"""#)
    }

    func testAVehicleThatIsNotThereIsNotAStationQuestion() {
        XCTAssertNil(controlStation(unavailable))
        XCTAssertNil(controlStation(nil))
    }

    func testAVehicleThatHasNeverSaidWhoIsFlyingItIsNotReportedAsTaken() throws {
        let station = try XCTUnwrap(controlStation(silent))
        XCTAssertFalse(station.known)
        XCTAssertNil(station.inControl)
        XCTAssertFalse(controlIsElsewhere(station))
        XCTAssertNil(controlLine(station), "a single-station setup would carry this sentence forever, and QGC hides the indicator entirely until the vehicle answers")
    }

    func testHoldingControlOffersNoRequestOnlyTheTakeoverCondition() throws {
        let station = try XCTUnwrap(controlStation(ours))
        XCTAssertFalse(controlIsElsewhere(station))
        XCTAssertNil(controlLine(station))
        XCTAssertNil(acquireLabel(station))
        XCTAssertEqual(inControlLine(station), "System in control: This GCS (255)")
        XCTAssertEqual(takeoverLine(station), "Takeover allowed")
        XCTAssertFalse(takeoverChangeable(station, true), "Change is off until the box differs from what the vehicle reports")
        XCTAssertTrue(takeoverChangeable(station, false))
        XCTAssertFalse(takeoverChangeable(controlStation(elsewhere(takeover: true)), false))
        XCTAssertTrue(allowTakeoverEditable(station), "QGC enables the box while this station holds control")
        XCTAssertTrue(allowTakeoverEditable(controlStation(elsewhere(takeover: true))), "or while the holder allows a takeover")
        XCTAssertFalse(allowTakeoverEditable(controlStation(elsewhere(takeover: false))))
    }

    func testTheTakeoverConditionShowsWhoeverHoldsControlAndTheRequestCountsDown() {
        XCTAssertEqual(takeoverLine(controlStation(elsewhere(takeover: false))), "Takeover NOT allowed")
        XCTAssertNil(takeoverLine(controlStation(silent)))
        XCTAssertNil(inControlLine(controlStation(elsewhere(takeover: true))))
        XCTAssertEqual(requestSentLabel(9_500), "Request sent: 9.5")
        XCTAssertEqual(requestSentLabel(-20), "Request sent: 0.0")
    }

    func testAnotherStationIsNamedByTheSystemItFliesUnder() throws {
        let station = try XCTUnwrap(controlStation(elsewhere(takeover: true)))
        XCTAssertTrue(controlIsElsewhere(station))
        XCTAssertEqual(controlLine(station), "Another ground station is flying this vehicle. (system 42)")
        XCTAssertEqual(holderLine(station), "System in control: 42", "GCSControlIndicator lists the holder as 'System in control:' and its id")
        XCTAssertNil(holderLine(controlStation(ours)))
        XCTAssertNil(holderLine(controlStation(silent)))
    }

    func testTheOfferFollowsWhetherTakeoverWasAllowed() {
        XCTAssertEqual(acquireLabel(controlStation(elsewhere(takeover: true))), "Acquire control")
        XCTAssertEqual(acquireLabel(controlStation(elsewhere(takeover: false))), "Send request")
    }

    func testAPendingRequestKeepsTheButtonButDisablesItLikeTheQml() throws {
        let pending = try XCTUnwrap(controlStation(elsewhere(takeover: false, request: false)))
        XCTAssertEqual(acquireLabel(pending), "Send request")
        XCTAssertFalse(acquireEnabled(pending, false))
        XCTAssertFalse(acquireEnabled(try XCTUnwrap(controlStation(elsewhere(takeover: false))), true))
        XCTAssertTrue(acquireEnabled(try XCTUnwrap(controlStation(elsewhere(takeover: false))), false))
    }

    func testAStationThatNeverAnsweredIsNotOfferedARequestEither() {
        XCTAssertNil(acquireLabel(controlStation(silent)))
    }

    func testTheTimeoutIsOnlySpentWhenTakeoverHasToBeAskedFor() throws {
        XCTAssertEqual(
            requestTimeoutSeconds(try XCTUnwrap(controlStation(elsewhere(takeover: true))), 10),
            0,
            "Vehicle::requestOperatorControl clamps anything outside 3..60 to the setting's default, so this 0 is what the API takes and never what the command carries - it means do not start a waiting timer, which is what the QML does with it too"
        )
        XCTAssertEqual(requestTimeoutSeconds(try XCTUnwrap(controlStation(elsewhere(takeover: false))), 10), 10)
    }

    func testAnOutstandingRequestIsNamedWhileTheOtherStationHasNotAnswered() {
        XCTAssertEqual(controlWaitLine(controlStation(elsewhere(takeover: false, request: false))), "Waiting for the other station to answer")
    }

    func testAStationThatMayStillBeAskedIsNotDescribedAsWaiting() {
        XCTAssertNil(controlWaitLine(controlStation(elsewhere(takeover: false))))
    }

    func testTheVehicleWeFlyIsNeverDescribedAsWaitingOnUs() {
        XCTAssertNil(controlWaitLine(controlStation(ours)))
        XCTAssertNil(controlWaitLine(controlStation(silent)))
    }

    func testTheWaitLineStandsBesideTheDisabledButton() throws {
        let waiting = try XCTUnwrap(controlStation(elsewhere(takeover: true, request: false)))
        XCTAssertEqual(acquireLabel(waiting), "Acquire control")
        XCTAssertFalse(acquireEnabled(waiting, false))
        XCTAssertEqual(controlWaitLine(waiting), "Waiting for the other station to answer")
    }

    func testTheSheetTitlesAndTimeoutFieldFollowGcsControlIndicator() throws {
        let asking = try XCTUnwrap(controlStation(elsewhere(takeover: false)))
        let taking = try XCTUnwrap(controlStation(elsewhere(takeover: true)))
        let holding = try XCTUnwrap(controlStation(ours))
        XCTAssertEqual(controlSectionTitle(asking), "Send control request")
        XCTAssertEqual(controlSectionTitle(holding), "Change takeover condition")
        XCTAssertNil(controlSectionTitle(try XCTUnwrap(controlStation(silent))))
        XCTAssertTrue(requestTimeoutEditable(asking))
        XCTAssertFalse(requestTimeoutEditable(taking))
        XCTAssertFalse(requestTimeoutEditable(holding))
    }
}
