import XCTest
@testable import Aircast

final class GuidedOffersTests: XCTestCase {
    func testTheRtlOfferCarriesTheCoresSmartRtlOption() {
        let offers = guidedOffers(JSON.parse(#"{"actions":[{"id":"rtl","offer":"ready","option":"Smart RTL"},{"id":"land","offer":"ready","option":""}]}"#))
        XCTAssertEqual(offers["rtl"]!.option, "Smart RTL")
        XCTAssertEqual(offers["land"]!.option, "")
    }

    func testTheMultiVehiclePanelIsOnUnlessTheSettingTurnsItOff() {
        XCTAssertEqual([nil, JSON.parse(#"{"value":true}"#), JSON.parse(#"{"value":false}"#)].map { multiVehiclePanelEnabled($0) }, [true, true, false])
    }

    func testAConfirmClosesOnceItsActionStopsBeingOfferedLikeGuidedActionConfirmsHideTrigger() {
        let offers = guidedOffers(JSON.parse(#"{"actions":[{"id":"land","offer":"ready"},{"id":"takeoff","offer":"hidden"}]}"#))
        XCTAssertEqual([offerWithdrawn("land", offers), offerWithdrawn("takeoff", offers), offerWithdrawn("orbit", offers), offerWithdrawn(nil, offers)], [false, true, true, false])
    }

    func testTheChecklistResultIsWrittenAsVehiclesCheckListPassedOrCheckListFailed() {
        XCTAssertEqual([checklistStateValue(true), checklistStateValue(false)], [1, 2])
    }
}
