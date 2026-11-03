import XCTest
@testable import Aircast

final class VideoGimbalModeTests: XCTestCase {
    func testEachWayADragOnTheVideoCanSteerTheGimbalMapsToItsTwoSettingsAndBack() {
        XCTAssertEqual(VideoGimbalMode.allCases.map { videoGimbalMode(OnScreenGimbal(enabled: $0.onScreen, clickAndDrag: $0.clickAndDrag)) }, VideoGimbalMode.allCases)
        XCTAssertEqual(videoGimbalMode(OnScreenGimbal(enabled: false, clickAndDrag: true)), .Drag)
        XCTAssertEqual(VideoGimbalMode.allCases.map(\.label), ["Drag to move", "Tap to point", "Hold and drag"])
    }

    func testTheSwitchOnTheVideoStepsThroughEveryModeAndBackToTheFirst() {
        XCTAssertEqual(VideoGimbalMode.allCases.map(nextVideoGimbalMode), [.Point, .Hold, .Drag])
    }
}
