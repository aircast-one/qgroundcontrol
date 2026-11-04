#if os(macOS)
import XCTest
@testable import Aircast

final class FirmwareFlashMacTests: XCTestCase {
    func testTheCoresFirmwareViewsReadAsTheScreenShowsThem() {
        let ports = JSON.parse(#"{"kind":"object","class":"FirmwarePorts","ports":[{"port":"/dev/cu.usbmodem01","description":"PX4 BL FMU v5.x","bootloader":true},{"port":"","description":"x","bootloader":false}]}"#)
        XCTAssertEqual(firmwarePorts(ports), [FirmwarePort(port: "/dev/cu.usbmodem01", description: "PX4 BL FMU v5.x", bootloader: true)])
        let job = JSON.parse(#"{"kind":"object","class":"FirmwareUpgrade","phase":"programming","busy":true,"cancellable":false,"progress":0.25,"messages":["Erasing previous program...","Erase complete"],"error":null}"#)
        XCTAssertEqual(
            firmwareJob(job),
            FirmwareJob(phase: "programming", busy: true, cancellable: false, progress: 0.25, messages: ["Erasing previous program...", "Erase complete"], error: "")
        )
        XCTAssertNil(firmwareJob(JSON.parse(#"{"kind":"null"}"#)), "a refusal that is not the firmware view reads as nothing to show")
        XCTAssertEqual(firmwarePhaseText("connecting"), "Waiting for the bootloader")
        XCTAssertEqual(firmwarePhaseText("idle"), "")
        let choosing = JSON.parse(#"{"class":"FirmwareUpgrade","phase":"choosing","busy":true,"cancellable":true,"choices":[{"name":"CUAVv5 - 4.6.2","url":"https://f/c.apj"}]}"#)
        XCTAssertEqual(firmwareJob(choosing)?.choices, [FirmwareBuildChoice(name: "CUAVv5 - 4.6.2", url: "https://f/c.apj")])
        XCTAssertEqual(firmwarePhaseText("choosing"), "Choose the firmware build")
    }

    func testAReleaseIsSentAsTheCoresTokenAndAFileAsItsPath() {
        XCTAssertEqual(firmwareChoice("ardupilot:heli:dev", nil), "ardupilot:heli:dev")
        XCTAssertEqual(firmwareChoice(FIRMWARE_FROM_FILE, "/tmp/firmware-fw.px4"), "/tmp/firmware-fw.px4")
        XCTAssertNil(firmwareChoice(FIRMWARE_FROM_FILE, nil))
        XCTAssertTrue(FIRMWARE_SOURCES.contains { $0.first == "ardupilot:plane:stable" && $0.second == "ArduPilot Plane, stable" })
    }

    func testOnlyImagesTheBootloaderTakesAreAccepted() {
        XCTAssertTrue(firmwareFileAccepted("px4_fmu-v5_default.px4"))
        XCTAssertTrue(firmwareFileAccepted("arducopter.APJ"))
        XCTAssertTrue(firmwareFileAccepted("fw.bin"))
        XCTAssertFalse(firmwareFileAccepted("notes.txt"))
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

    func testTheLastStackAndArduPilotVehicleAreRememberedLikeFirmwareUpgradeQml() {
        XCTAssertEqual(rememberedSource(12, 2), "px4:stable")
        XCTAssertEqual(rememberedSource(3, 2), "ardupilot:plane:stable")
        XCTAssertEqual(rememberedSource(3, nil), "ardupilot:copter:stable")
        XCTAssertEqual(
            sourceSettings("ardupilot:sub:beta"),
            [FirmwareSourceSetting(name: "defaultFirmwareType", value: 3), FirmwareSourceSetting(name: "apmVehicleType", value: 4)]
        )
        XCTAssertEqual(sourceSettings("px4:dev"), [FirmwareSourceSetting(name: "defaultFirmwareType", value: 12)])
        XCTAssertEqual(sourceSettings("sik:stable"), [])
    }

    func testFlashingNamesTheBoardLikeFirmwareUpgradeQml() {
        XCTAssertEqual(flashingLabel([FirmwarePort(port: "/dev/cu.usbmodem1", description: "Pixhawk 6C", bootloader: true)], "/dev/cu.usbmodem1"), "Flashing - Pixhawk 6C")
        XCTAssertEqual(flashingLabel([], "/dev/x"), "Flashing - /dev/x")
    }

    func testARecognisedSelectionIsKeptElseTheFirstPixhawkThenASikRadioLikePreselectIndex() {
        let radio = FirmwarePort(port: "r", description: "SiK", bootloader: false, boardType: "SiK Radio")
        let fmu = FirmwarePort(port: "p", description: "Pixhawk 6C", bootloader: false, boardType: "Pixhawk")
        let other = FirmwarePort(port: "o", description: "FTDI", bootloader: false)
        XCTAssertEqual(preselectedPort([other, radio, fmu], ""), "p")
        XCTAssertEqual(preselectedPort([other, radio], "o"), "r")
        XCTAssertEqual(preselectedPort([radio, fmu], "r"), "r")
        XCTAssertNil(preselectedPort([other], ""))
    }

    func testAPortReadsAsItsBootloaderOrItsBoardAndPath() {
        XCTAssertEqual(firmwarePortSubtitle(FirmwarePort(port: "/dev/cu.a", description: "", bootloader: true, boardType: "Pixhawk")), "In its bootloader")
        XCTAssertEqual(firmwarePortSubtitle(FirmwarePort(port: "/dev/cu.a", description: "", bootloader: false, boardType: "Pixhawk")), "Pixhawk \u{00b7} /dev/cu.a")
        XCTAssertEqual(firmwarePortSubtitle(FirmwarePort(port: "/dev/cu.a", description: "", bootloader: false)), "/dev/cu.a")
    }

    func testLeavingThePageDropsABoardSearchButNeverInterruptsAWrite() {
        XCTAssertEqual(
            ["connecting", "choosing", "erasing", "programming", "verifying", "idle"].map { cancelledOnLeave($0) },
            [true, true, false, false, false, false],
            "FirmwareUpgradeController's destructor allows connections again, which ends the bootloader search"
        )
        XCTAssertFalse(cancelledOnLeave(nil))
    }
}
#endif
