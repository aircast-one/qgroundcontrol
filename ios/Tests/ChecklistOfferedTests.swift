import XCTest
@testable import Aircast

final class ChecklistOfferedTests: XCTestCase {
    func testAnArmedVehicleIsPastTheChecksRatherThanFailingThem() {
        XCTAssertEqual(
            checklistOffered(true),
            "The checks are for before the flight",
            "QGC disables its own Checklist action on an armed vehicle - "
                + "PreFlightCheckListShowAction.qml gates enabled on !armed - and asking whether the "
                + "props are mounted is not a question for an aircraft already flying on them"
        )
    }

    func testADisarmedVehicleIsOfferedTheChecksAsBefore() {
        XCTAssertNil(checklistOffered(false))
    }

    func testAnOperatorWhoTurnedTheChecklistOffInSettingsIsNotOfferedIt() {
        XCTAssertEqual(preflightOffered(JSON.parse(#"{"offered":false}"#)), false)
        XCTAssertEqual(preflightOffered(JSON.parse(#"{"offered":true}"#)), true)
    }

    func testACoreTooOldToServeTheFieldKeepsOfferingTheChecks() {
        XCTAssertEqual(
            preflightOffered(JSON.parse(#"{"airframe":"Quadrotor"}"#)),
            true,
            "offered arrived in 98cd88175, so a head running against an older core sees no key at "
                + "all - defaulting that to false would silently withdraw the checklist from every "
                + "operator rather than from the ones who asked for it to go"
        )
        XCTAssertEqual(preflightOffered(nil), true)
    }

    func testBeingPastTheChecksAndNeverHavingWantedThemAreDifferentAnswers() {
        XCTAssertEqual(
            checklistOffered(true),
            "The checks are for before the flight",
            "armed dims a row that is still drawn, because the state will pass; offered removes the "
                + "row, because the operator withdrew it - one answer for both would either nag "
                + "someone who opted out or leave them no reason for a dimmed row"
        )
        XCTAssertEqual(preflightOffered(JSON.parse(#"{"offered":true}"#)), true)
    }
}
