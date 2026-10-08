import XCTest
@testable import Aircast

final class VideoStatsPillTests: XCTestCase {
    func testThePillShowsTheCoresStatsLine() {
        XCTAssertEqual(videoStatsText(JSON.parse(#"{"text":"30 fps · 720p"}"#)), "30 fps · 720p")
        XCTAssertEqual(videoStatsText(nil), "")
    }
}
