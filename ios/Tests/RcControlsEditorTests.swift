import XCTest
@testable import Aircast

final class RcControlsEditorTests: XCTestCase {
    func testEachConfiguredCameraChannelIsReservedForTheSettingThatNamesIt() {
        let channels = RcCameraChannels(tilt: 6, pan: 7, zoom: 0, light: 9, record: 10)
        XCTAssertEqual(reservedChannels(channels), [6: "Gimbal tilt", 7: "Gimbal pan", 9: "Camera light", 10: "Camera record"])
    }

    func testAChannelTwoSettingsShareIsReservedByTheLaterOneAsAssociateKeepsIt() {
        XCTAssertEqual(reservedChannels(RcCameraChannels(tilt: 6, pan: 6, zoom: 0, light: 0, record: 0)), [6: "Gimbal pan"])
        XCTAssertEqual(reservedChannels(RcCameraChannels(tilt: 0, pan: 0, zoom: 0, light: 0, record: 0)), [:])
    }
}
