import XCTest
@testable import Aircast

final class MockLinkFieldsTests: XCTestCase {
    func testMockLinksAreStartedFromAddLinkNotASettingsPageOfTheirOwn() {
        XCTAssertTrue(settingsPages(JSON.parse(#"{"pages":[{"title":"Mock Link","sections":[]}]}"#)).isEmpty)
        XCTAssertEqual(addableLinkTypes(JSON.parse(#"{"linkTypeIds":["serial","udp","tcp","mock"]}"#)), [.Udp, .Tcp, .Serial, .Mock])
    }

    func testStartSendsTheVehicleKeyAndOptionsInTheOrderTheCoreReadsThem() {
        let choices = MockLinkChoices(sendStatusText: true, camera: true, proximity: true, freshParams: true, vehicle: 2, videoStream: 3)
        XCTAssertEqual(JSON(mockLinkArguments(choices)), JSON.parse(#"["apmCopter",true,true,false,true,true,3]"#))
    }

    func testFreshFirmwareParametersOnlyReachAnArduPilotVehicle() {
        let px4 = MockLinkChoices(freshParams: true, vehicle: 0)
        XCTAssertEqual(mockLinkArguments(px4)[5] as? Bool, false)
        XCTAssertEqual(withMockVehicle(MockLinkChoices(freshParams: true, vehicle: 2), 0).freshParams, false)
        XCTAssertEqual(mockLinkArguments(MockLinkChoices(freshParams: true, vehicle: 1))[5] as? Bool, false, "a PX4 plane has no ArduPilot parameter set to start fresh from")
        XCTAssertEqual(MOCK_VEHICLES.map(\.0), ["px4", "px4Plane", "apmCopter", "apmPlane", "apmSub", "apmRover", "generic"])
        XCTAssertEqual(MOCK_VIDEO_STREAMS.count, 6)
    }

    func testANewSimulatedVehicleStartsOnTheVehicleTheCorePickedFromThePreferredFirmware() {
        let picked = { (view: JSON?) in MOCK_VEHICLES[preferredMockChoices(view).vehicle].1 }
        XCTAssertEqual(picked(JSON.parse(#"{"mockVehicle":"apmCopter"}"#)), "APM ArduCopter Vehicle")
        XCTAssertEqual(picked(JSON.parse(#"{"mockVehicle":"apmPlane"}"#)), "APM ArduPlane Vehicle")
        XCTAssertEqual([picked(JSON.parse(#"{"mockVehicle":"px4"}"#)), picked(nil), picked(JSON.parse("{}"))], ["PX4 Vehicle", "PX4 Vehicle", "PX4 Vehicle"])
    }
}
