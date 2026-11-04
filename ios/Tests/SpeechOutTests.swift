import AVFoundation
import XCTest
@testable import Aircast

// The audio session is iOS-only; a Mac mixes speech with other audio without one.
#if os(iOS)
final class SpeechOutTests: XCTestCase {
    func testCalloutsMixWithOtherAudioWithoutLeavingItDucked() {
        XCTAssertTrue(SpeechOut.sessionOptions.contains(.mixWithOthers))
        XCTAssertFalse(SpeechOut.sessionOptions.contains(.duckOthers))
    }
}
#endif
