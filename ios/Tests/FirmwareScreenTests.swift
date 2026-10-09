import XCTest
@testable import Aircast

final class FirmwareScreenTests: XCTestCase {
    func testTheCoresFirmwareViewReadsAsTheReleasesTheScreenShows() {
        let view = JSON.parse(#"{"kind":"object","class":"FirmwareUpgrade","phase":"idle","busy":false,"updateAvailable":"Update available","px4StableVersion":"v1.17.0","px4BetaVersion":null}"#)
        XCTAssertEqual(firmwareReleases(view), FirmwareReleases(updateAvailable: "Update available", px4StableVersion: "v1.17.0", px4BetaVersion: ""))
        XCTAssertNil(firmwareReleases(JSON.parse(#"{"kind":"null"}"#)), "a refusal that is not the firmware view reads as nothing to show")
    }

    func testBetaAndDeveloperBuildsCarryQgcsWarningsLikeFirmwareUpgrade() {
        XCTAssertEqual(firmwareWarning("px4:beta"), BETA_WARNING)
        XCTAssertEqual(firmwareWarning("ardupilot:copter:dev"), DEV_WARNING)
        XCTAssertNil(firmwareWarning("px4:stable"))
    }

    func testWithoutAdvancedSettingsOnlyTheStablePx4ReleaseIsListed() {
        XCTAssertEqual(px4Releases(false).map(\.first), ["px4:stable"])
        XCTAssertEqual(px4Releases(true).map(\.first), ["px4:stable", "px4:beta"])
    }

    func testTheBootloaderButtonNeedsAdvancedSettingsAndAnArduPilotVehicle() {
        XCTAssertTrue(bootloaderOffered(true, true))
        XCTAssertFalse(bootloaderOffered(false, true))
        XCTAssertFalse(bootloaderOffered(true, false))
    }

    func testPx4BuildsCarryTheReleaseNameLikeUpdatePX4VersionDisplay() {
        XCTAssertEqual(sourceLabel("px4:stable", "PX4 Pro, stable", "v1.17.0 - Stable Release", ""), "PX4 Pro v1.17.0 - Stable Release")
        XCTAssertEqual(sourceLabel("px4:beta", "PX4 Pro, beta", "v1.15.4", ""), "PX4 Pro, beta")
        XCTAssertEqual(sourceLabel("ardupilot:copter:stable", "ArduPilot Copter, stable", "v1.15.4", "x"), "ArduPilot Copter, stable")
    }
}
