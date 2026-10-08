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
}
