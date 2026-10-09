import XCTest
@testable import Aircast

final class ResumeFailedPromptTests: XCTestCase {
    func testAFailedResumeUploadNamesTheIndexToRetry() {
        XCTAssertEqual(resumeFailedIndex(JSON.parse(#"{"resumeFailedIndex":4}"#)), 4)
        XCTAssertNil(resumeFailedIndex(JSON.parse(#"{"resumeFailedIndex":null}"#)))
        XCTAssertNil(resumeFailedIndex(nil))
    }

    func testADismissalIsForgottenOnlyOnceTheCoreSaysNothingFailedNotWhileTheViewIsStillLoading() {
        XCTAssertFalse(resumeCleared(nil))
        XCTAssertFalse(resumeCleared(JSON.parse(#"{"resumeFailedIndex":4}"#)))
        XCTAssertTrue(resumeCleared(JSON.parse(#"{"resumeFailedIndex":null}"#)))
    }

    func testADismissedFailureStaysHiddenUntilAnotherIndexFails() {
        XCTAssertEqual(resumePending(4, nil), 4)
        XCTAssertNil(resumePending(4, 4))
        XCTAssertEqual(resumePending(5, 4), 5)
        XCTAssertNil(resumePending(nil, 4))
    }

    func testTheResumePromptIsTheTopmostAppDialogLikeTheLastComposedAlertDialog() {
        XCTAssertEqual(appAlert(true, [], resumeFailed: 3), .ResumeFailed(3))
        XCTAssertEqual(appAlert(true, [], resumeFailed: nil), .Gimbal)
    }
}
