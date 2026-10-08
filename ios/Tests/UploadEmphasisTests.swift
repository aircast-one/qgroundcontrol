import XCTest
@testable import Aircast

final class UploadEmphasisTests: XCTestCase {
    private func emphasised(_ offline: Bool, _ syncing: Bool) -> Bool {
        syncRefusal(vehicleSyncState(offline, syncing), "upload to") == nil
    }

    func testWithAVehicleReadyToTakeItUploadIsThePrimaryAction() {
        XCTAssertTrue(
            emphasised(false, false),
            "this is the direction that matters - the Plan tab exists to get a plan onto an aircraft, and de-emphasising the button that does it would be worse than the problem being fixed"
        )
        XCTAssertNil(syncRefusal(.Ready, "upload to"))
    }

    func testWithNoVehicleItIsNotAdvertisedAsTheThingToDo() {
        XCTAssertFalse(
            emphasised(true, false),
            "the core says canSend false with 'No vehicle is connected, so there is nowhere to send this plan' - a filled primary button is the head telling the operator to do the one thing that cannot work"
        )
    }

    func testMidSyncItIsNotAdvertisedEither() {
        XCTAssertFalse(emphasised(false, true))
    }

    func testItStaysTappableInEveryStateBecauseTheRefusalIsTheExplanation() {
        [VehicleSync.Offline, .Busy, .Ready].forEach { state in
            let refusal = syncRefusal(state, "upload to")
            XCTAssertTrue(
                refusal.map { !$0.isBlank } ?? true,
                "\(state): a greyed button on a phone cannot carry the sentence that explains it, so the button is always pressable and answers when pressed"
            )
        }
    }
}
