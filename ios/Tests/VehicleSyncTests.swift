import XCTest
@testable import Aircast

final class VehicleSyncTests: XCTestCase {
    func testAConnectedIdleVehicleIsReady() {
        XCTAssertEqual(vehicleSyncState(false, false), .Ready)
        XCTAssertNil(syncRefusal(.Ready, "send to"))
    }

    func testNoVehicleIsRefusedRatherThanReportedAsSent() {
        XCTAssertEqual(vehicleSyncState(true, false), .Offline)
        XCTAssertEqual(syncRefusal(.Offline, "send to"), "No vehicle to send to")
    }

    func testASyncAlreadyRunningIsRefused() {
        XCTAssertEqual(vehicleSyncState(false, true), .Busy)
        XCTAssertEqual(syncRefusal(.Busy, "send to"), "Already syncing, wait for it to finish")
    }

    func testBeingOfflineOutranksBeingBusy() {
        XCTAssertEqual(vehicleSyncState(true, true), .Offline)
    }

    func testTheRefusalNamesTheActionItRefused() {
        XCTAssertEqual(syncRefusal(.Offline, "load from"), "No vehicle to load from")
    }
}

final class UploadGateTests: XCTestCase {
    private func view(_ upload: String) -> JSON { JSON.parse(#"{"upload":\#(upload)}"#) }

    func testACleanPlanUploadsWithoutAsking() {
        XCTAssertEqual(uploadStep(uploadGate(view(#"{"canSend":true,"canProceed":false}"#))), .Send)
    }

    func testAVehicleFlyingThisMissionIsPausedFirstNotSilentlyOverwritten() {
        let gate = uploadGate(view(#"{"canSend":false,"canProceed":true,"pausesFirst":true,"heading":"Upload this plan?","proceedTitle":"Pause and upload","refusal":"The vehicle is flying this mission."}"#))
        guard case .Confirm(let confirmed) = uploadStep(gate) else { return XCTFail("expected a confirmation") }
        XCTAssertTrue(confirmed.pausesFirst)
        XCTAssertEqual(confirmed.proceedTitle, "Pause and upload")
    }

    func testAFirmwareMismatchWarnsAndOffersToGoAhead() {
        let gate = uploadGate(view(#"{"canSend":false,"canProceed":true,"pausesFirst":false,"proceedTitle":"Upload anyway"}"#))
        guard case .Confirm(let confirmed) = uploadStep(gate) else { return XCTFail("expected a confirmation") }
        XCTAssertFalse(confirmed.pausesFirst)
    }

    func testNoVehicleIsARefusalWithTheCoresWords() {
        let gate = uploadGate(view(#"{"canSend":false,"canProceed":false,"refusal":"No vehicle is connected."}"#))
        XCTAssertEqual(uploadStep(gate), .Refuse("No vehicle is connected."))
    }

    func testAViewWithNoUploadBlockRefusesRatherThanSending() {
        guard case .Refuse = uploadStep(nil) else { return XCTFail("expected a refusal") }
        guard case .Refuse = uploadStep(uploadGate(JSON.parse("{}"))) else { return XCTFail("expected a refusal") }
    }

    func testUploadLabelFollowsPlanToolBarIndicators() {
        XCTAssertEqual(uploadLabel(false, true, false, true), "Uploading…")
        XCTAssertEqual(uploadLabel(false, false, false, true), "Uploaded")
        XCTAssertEqual(uploadLabel(false, false, true, true), "Upload")
        XCTAssertEqual(uploadLabel(true, false, false, true), "Upload")
        XCTAssertEqual(uploadLabel(false, false, false, false), "Upload")
    }
}
