import XCTest
@testable import Aircast

final class FieldRunsTests: XCTestCase {
    private func choice(_ name: String, _ options: [String] = ["Metric", "Imperial", "SI", "Nautical", "Other"], label: String? = nil) -> Fact {
        Fact(
            path: "settings.\(name)", name: name, description: "\(name) long description", units: "", valueString: options[0],
            value: .number(0), enumStrings: options, enumIndex: 0, isBool: false, isString: false, readOnly: false, shortLabel: label ?? name
        )
    }

    private func text(_ name: String, isString: Bool = true, units: String = "") -> Fact {
        Fact(
            path: "settings.\(name)", name: name, description: name, units: units, valueString: "", value: .string(""),
            enumStrings: [], enumIndex: -1, isBool: false, isString: isString, readOnly: false, shortLabel: name
        )
    }

    private func changed(_ fact: Fact, _ change: (inout Fact) -> Void) -> Fact {
        var copy = fact
        change(&copy)
        return copy
    }

    func testASettingWithNoEffectYetStillDrawsAsAField() {
        let host = changed(text("host", isString: false)) {
            $0.enabled = false
            $0.disabledReason = "Has no effect while the ADS-B server connection is off."
        }
        XCTAssertTrue(showsAsField(host))
        XCTAssertFalse(showsAsField(changed(host) { $0.readOnly = true }))
    }

    func testNumbersAndChoicesSitRightOfTheirLabelWhileTextTakesTheWholeRow() {
        XCTAssertTrue(valueOnTheRight(text("alt", isString: false, units: "m")))
        XCTAssertTrue(valueOnTheRight(choice("units")))
        XCTAssertFalse(valueOnTheRight(text("url")))
    }

    func testANumberFieldDropsThePaddedDecimalsButTextIsLeftAlone() {
        XCTAssertEqual(fieldText(changed(text("d", isString: false)) { $0.valueString = "300.000" }), "300")
        XCTAssertEqual(fieldText(changed(text("d", isString: false)) { $0.valueString = "2.50" }), "2.5")
        XCTAssertEqual(fieldText(changed(text("s")) { $0.valueString = "007.10" }), "007.10")
    }

    func testALabelThatNamesAnOptionIsNotShownAsTheSubtitle() {
        let fence = changed(choice("FENCE_ENABLE", ["Disabled", "Enabled"], label: "Fence enable/disable")) { $0.description = "Enabled" }
        XCTAssertEqual(fence.detail, "")
        XCTAssertEqual(fence.heading, "Fence enable/disable")
        XCTAssertEqual(changed(fence) { $0.description = "Allows the fence" }.detail, "Allows the fence")
    }

    func testALabelEndingInAColonLosesIt() {
        XCTAssertEqual(choice("RTL", label: "Return at specified altitude:").heading, "Return at specified altitude")
        XCTAssertEqual(changed(text("t")) {
            $0.description = "Time Offset (seconds):"
            $0.shortLabel = ""
        }.heading, "Time Offset (seconds)")
    }

    func testANoteThatOnlyRestatesTheLabelIsDropped() {
        let multiplier = changed(text("BATT_VOLT_MULT", isString: false)) {
            $0.shortLabel = "Voltage Multiplier"
            $0.description = "Voltage multiplier"
        }
        let labelled = { (label: String, description: String) in
            self.changed(multiplier) {
                $0.shortLabel = label
                $0.description = description
            }.detail
        }
        XCTAssertEqual(multiplier.detail, "")
        XCTAssertEqual(labelled("Battery monitoring", "Battery monitor"), "")
        XCTAssertEqual(labelled("Required arming voltage", "Minimum arming voltage"), "Minimum arming voltage")
        XCTAssertEqual(labelled("ArduPilot support host", "Ardupilot Support Host name"), "")
        XCTAssertEqual(labelled("Color scheme", "Application color scheme"), "")
        XCTAssertEqual(labelled("RTL loiter time", "Loiter time"), "")
        XCTAssertEqual(labelled("RTL final altitude", "Final land stage altitude"), "Final land stage altitude")
        XCTAssertEqual(labelled("Attitude control input time constant", "RC Roll/Pitch Feel"), "RC Roll/Pitch feel")
        XCTAssertEqual(labelled("Confirm", "Only while in Guided mode."), "Only while in Guided mode.")
    }

    func testARebootNoticeSharedByTheBlockIsSaidOnce() {
        let reboot = changed(text("a")) { $0.vehicleRebootRequired = true }
        XCTAssertEqual(sharedRebootNote([reboot, text("plain"), changed(reboot) { $0.name = "b" }]), "Reboot vehicle for changes to take effect.")
        XCTAssertNil(sharedRebootNote([reboot, text("plain")]))
        XCTAssertNil(sharedRebootNote([reboot, changed(text("c")) { $0.qgcRebootRequired = true }]))
    }

    func testARowLabelReadsInSentenceCaseKeepingAcronymsAndNames() {
        XCTAssertEqual(sentenceCase("Mute Audio Output"), "Mute audio output")
        XCTAssertEqual(sentenceCase("Use Preflight Checklist"), "Use preflight checklist")
        XCTAssertEqual(sentenceCase("Forward MAVLink To UDP Host"), "Forward MAVLink to UDP host")
        XCTAssertEqual(sentenceCase("PX4 Pro"), "PX4 Pro")
        XCTAssertEqual(sentenceCase("3DR Solo (requires restart)"), "3DR Solo (requires restart)")
        XCTAssertEqual(sentenceCase("Yuneec Mantis G"), "Yuneec Mantis G")
        XCTAssertEqual(sentenceCase("Herelink Hotspot"), "Herelink Hotspot")
        XCTAssertEqual(sentenceCase("Mapbox Token"), "Mapbox token")
        XCTAssertEqual(sentenceCase("Automatically connect to a Pixhawk board"), "Automatically connect to a Pixhawk board")
        XCTAssertEqual(sentenceCase("Time Offset (seconds)"), "Time offset (seconds)")
        XCTAssertEqual(sentenceCase("Auto-Center throttle"), "Auto-center throttle")
        XCTAssertEqual(sentenceCase("Left-Handed mode"), "Left-handed mode")
    }

    func testABlockWhoseFieldsAreAllOffForOneReasonSaysItOnce() {
        let off = changed(text("basicID")) {
            $0.enabled = false
            $0.disabledReason = "Has no effect while Basic ID broadcast is off."
        }
        let toggle = text("sendBasicID")
        XCTAssertEqual(blockInertNote([toggle, off, changed(off) { $0.name = "uaType" }]), "Has no effect while Basic ID broadcast is off.")
        XCTAssertNil(blockInertNote([toggle, off]))
    }

    func testSecondsReadAsS() {
        XCTAssertEqual(shownUnits("secs"), "s")
        XCTAssertEqual(shownUnits("Seconds"), "s")
        XCTAssertEqual(shownUnits("m/s"), "m/s")
    }
}
