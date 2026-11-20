import XCTest
@testable import Aircast

final class UploadOutcomeTests: XCTestCase {
    func testAnInvokedUploadIsReportedAsSentNeverAsAccepted() {
        XCTAssertEqual(uploadOutcome(true), .Sent)
        XCTAssertEqual(uploadMessage(.Sent), "Upload sent to vehicle")
    }

    func testADownloadIsReportedAsRequestedNeverAsReceived() {
        XCTAssertEqual(downloadMessage(.Sent), "Download requested from vehicle")
        XCTAssertEqual(downloadMessage(.NotStarted), "Download did not start")
    }

    func testACallThatNeverRanSaysSo() {
        XCTAssertEqual(uploadOutcome(false), .NotStarted)
        XCTAssertEqual(uploadMessage(.NotStarted), "Upload did not start")
    }

    func testAPauseTheAircraftRefusesStopsTheUploadAndSaysWhySoAMissionItIsStillFlyingIsNotReplaced() {
        XCTAssertEqual(refusalIn(JSON.parse(#"{"ok":false,"reason":"Pause is not available right now."}"#), "The aircraft did not pause."), "Pause is not available right now.")
        XCTAssertEqual(refusalIn(nil, "The aircraft did not pause."), "The aircraft did not pause.")
        XCTAssertNil(refusalIn(JSON.parse(#"{"ok":true}"#), "The aircraft did not pause."))
        XCTAssertEqual(pauseRefusedText("Pause is not available right now."), "The aircraft did not pause, so the plan was not uploaded. Pause is not available right now.")
    }
}
