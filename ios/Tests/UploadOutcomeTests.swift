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
}
