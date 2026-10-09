import AVFoundation
import XCTest
@testable import Aircast

final class SpeechOutTests: XCTestCase {
    func testCalloutsMixWithOtherAudioWithoutLeavingItDucked() {
        XCTAssertTrue(SpeechOut.sessionOptions.contains(.mixWithOthers))
        XCTAssertFalse(SpeechOut.sessionOptions.contains(.duckOthers))
    }
}
