import XCTest
@testable import Aircast

final class FirmwareScreenTests: XCTestCase {
    func testTheCoresFirmwareViewsReadAsTheScreenShowsThem() throws {
        let ports = JSON.parse(#"{"kind":"object","class":"FirmwarePorts","ports":[{"port":"/dev/bus/usb/001/002","description":"PX4 BL FMU v5.x","bootloader":true},{"port":"","description":"x","bootloader":false}]}"#)
        XCTAssertEqual(firmwarePorts(ports), [FirmwarePort(port: "/dev/bus/usb/001/002", description: "PX4 BL FMU v5.x", bootloader: true)])
        let job = JSON.parse(#"{"kind":"object","class":"FirmwareUpgrade","phase":"programming","busy":true,"cancellable":false,"progress":0.25,"messages":["Erasing previous program...","Erase complete"],"error":null}"#)
        XCTAssertEqual(firmwareJob(job), FirmwareJob(phase: "programming", busy: true, cancellable: false, progress: 0.25, messages: ["Erasing previous program...", "Erase complete"], error: ""))
        XCTAssertNil(firmwareJob(JSON.parse(#"{"kind":"null"}"#)), "a refusal that is not the firmware view reads as nothing to show")
        XCTAssertEqual(firmwarePhaseText("connecting"), "Waiting for the bootloader")
        XCTAssertEqual(firmwarePhaseText("idle"), "")
        let choosing = JSON.parse(#"{"class":"FirmwareUpgrade","phase":"choosing","busy":true,"cancellable":true,"choices":[{"name":"CUAVv5 - 4.6.2","url":"https://f/c.apj"}]}"#)
        XCTAssertEqual(try XCTUnwrap(firmwareJob(choosing)).choices.map { [$0.first, $0.second] }, [["CUAVv5 - 4.6.2", "https://f/c.apj"]])
        XCTAssertEqual(firmwarePhaseText("choosing"), "Choose the firmware build")
    }

    func testAReleaseIsSentAsTheCoresTokenAndAFileAsItsPath() {
        XCTAssertEqual(firmwareChoice("ardupilot:heli:dev", nil), "ardupilot:heli:dev")
        XCTAssertEqual(firmwareChoice(FIRMWARE_FROM_FILE, "/cache/firmware-fw.px4"), "/cache/firmware-fw.px4")
        XCTAssertNil(firmwareChoice(FIRMWARE_FROM_FILE, nil))
        XCTAssertTrue(FIRMWARE_SOURCES.contains { $0.first == "ardupilot:plane:stable" && $0.second == "ArduPilot Plane, stable" })
    }

    func testOnlyImagesTheBootloaderTakesAreAccepted() {
        XCTAssertTrue(firmwareFileAccepted("px4_fmu-v5_default.px4"))
        XCTAssertTrue(firmwareFileAccepted("arducopter.APJ"))
        XCTAssertTrue(firmwareFileAccepted("fw.bin"))
        XCTAssertFalse(firmwareFileAccepted("notes.txt"))
    }

    func testBetaAndDeveloperBuildsCarryQgcsWarningsLikeFirmwareUpgrade() {
        XCTAssertEqual(firmwareWarning("px4:beta"), BETA_WARNING)
        XCTAssertEqual(firmwareWarning("ardupilot:copter:dev"), DEV_WARNING)
        XCTAssertNil(firmwareWarning("px4:stable"))
    }

    func testWithoutAdvancedSettingsOnlyTheStandardBuildsAreOfferedAsFirmwareUpgradeShows() {
        let plain = firmwareSources(false).map(\.first)
        XCTAssertTrue(plain.allSatisfy { $0.hasSuffix(":stable") })
        XCTAssertTrue(plain.contains("px4:stable"))
        XCTAssertFalse(plain.contains(FIRMWARE_FROM_FILE))
        XCTAssertTrue(firmwareSources(true).map(\.first).contains(FIRMWARE_FROM_FILE))
        XCTAssertTrue(firmwareSources(true).map(\.first).contains("px4:dev"))
        XCTAssertEqual(firmwareWarning("px4:dev"), DEV_WARNING)
        XCTAssertEqual(sourceAfterAdvanced("ardupilot:plane:dev", false), "ardupilot:plane:stable")
        XCTAssertEqual(sourceAfterAdvanced(FIRMWARE_FROM_FILE, false), DEFAULT_FIRMWARE_SOURCE)
        XCTAssertEqual(sourceAfterAdvanced("px4:beta", true), "px4:beta")
    }

    func testTheBootloaderButtonNeedsAdvancedSettingsAndAnArduPilotVehicle() {
        XCTAssertTrue(bootloaderOffered(true, true))
        XCTAssertFalse(bootloaderOffered(false, true))
        XCTAssertFalse(bootloaderOffered(true, false))
    }

    func testTheLastStackAndArduPilotVehicleAreRememberedLikeFirmwareUpgradeQml() {
        XCTAssertEqual(rememberedSource(12, 2), "px4:stable")
        XCTAssertEqual(rememberedSource(3, 2), "ardupilot:plane:stable")
        XCTAssertEqual(rememberedSource(3, nil), "ardupilot:copter:stable")
        XCTAssertEqual(sourceSettings("ardupilot:sub:beta").map { "\($0.first)=\($0.second)" }, ["defaultFirmwareType=3", "apmVehicleType=4"])
        XCTAssertEqual(sourceSettings("px4:dev").map { "\($0.first)=\($0.second)" }, ["defaultFirmwareType=12"])
        XCTAssertTrue(sourceSettings("sik:stable").isEmpty)
    }

    func testFlashingNamesTheBoardLikeFirmwareUpgradeQml() {
        XCTAssertEqual(flashingLabel([FirmwarePort(port: "/dev/bus/usb/1", description: "Pixhawk 6C", bootloader: true)], "/dev/bus/usb/1"), "Flashing - Pixhawk 6C")
        XCTAssertEqual(flashingLabel([], "/dev/x"), "Flashing - /dev/x")
    }

    func testARecognisedSelectionIsKeptElseTheFirstPixhawkThenASiKRadioLikePreselectIndex() {
        let radio = FirmwarePort(port: "r", description: "SiK", bootloader: false, boardType: "SiK Radio")
        let fmu = FirmwarePort(port: "p", description: "Pixhawk 6C", bootloader: false, boardType: "Pixhawk")
        let other = FirmwarePort(port: "o", description: "FTDI", bootloader: false)
        XCTAssertEqual(preselectedPort([other, radio, fmu], ""), "p")
        XCTAssertEqual(preselectedPort([other, radio], "o"), "r")
        XCTAssertEqual(preselectedPort([radio, fmu], "r"), "r")
        XCTAssertNil(preselectedPort([other], ""))
    }

    func testPx4BuildsCarryTheReleaseNameLikeUpdatePX4VersionDisplay() {
        XCTAssertEqual(sourceLabel("px4:stable", "PX4 Pro, stable", "v1.17.0 - Stable Release", ""), "PX4 Pro v1.17.0 - Stable Release")
        XCTAssertEqual(sourceLabel("px4:beta", "PX4 Pro, beta", "v1.15.4", ""), "PX4 Pro, beta")
        XCTAssertEqual(sourceLabel("ardupilot:copter:stable", "ArduPilot Copter, stable", "v1.15.4", "x"), "ArduPilot Copter, stable")
    }

    func testLeavingThePageDropsABoardSearchButNeverInterruptsAWrite() {
        XCTAssertEqual(
            ["connecting", "choosing", "erasing", "programming", "verifying", "idle"].map(cancelledOnLeave),
            [true, true, false, false, false, false],
            "FirmwareUpgradeController's destructor allows connections again, which ends the bootloader search"
        )
        XCTAssertFalse(cancelledOnLeave(nil))
    }
}
