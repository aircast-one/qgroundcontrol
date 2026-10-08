import XCTest
@testable import Aircast

final class ParametersScreenTests: XCTestCase {
    func testAParameterPathIsAddressableThroughTheBridge() {
        XCTAssertEqual(parameterPath("ACRO_BAL_PITCH"), "vehicle.parameterManager.getParameter(-1,ACRO_BAL_PITCH)")
    }

    func testThePathARowWritesToIsTheOneTheBridgeCanResolve() {
        let path = parameterPath("RTL_ALT")
        XCTAssertTrue(path.contains("."), "a bare name does not resolve")
        XCTAssertTrue(path.hasPrefix("vehicle.parameterManager."))
        XCTAssertTrue(path.hasSuffix("(-1,RTL_ALT)"))
    }

    func testAParameterIsFoundByWhatItDoesNotOnlyByItsName() {
        XCTAssertTrue(parameterMatches("BATT_FS_LOW_ACT", ["Low battery failsafe action"], "failsafe"))
        XCTAssertTrue(parameterMatches("BATT_FS_LOW_ACT", ["Low battery failsafe action"], "BATT"))
        XCTAssertFalse(parameterMatches("BATT_FS_LOW_ACT", ["Low battery failsafe action"], "compass"))
    }

    func testEveryWordMustMatchAndEachWordIsARegularExpressionAsParameterEditorControllerDoes() {
        XCTAssertTrue(parameterMatches("BATT_VOLT_MULT", ["Voltage multiplier"], "batt volt"))
        XCTAssertFalse(parameterMatches("BATT_VOLT_MULT", ["Voltage multiplier"], "batt compass"))
        XCTAssertTrue(parameterMatches("RC1_MIN", ["RC min PWM"], "^RC"))
        XCTAssertFalse(parameterMatches("SERVO_RC_ON", ["Servo"], "^RC"))
        XCTAssertTrue(parameterMatches("ATC_RATE_P", ["Roll axis rate P gain"], "RATE.*P"))
        XCTAssertTrue(parameterMatches("A(B", [""], "A(B"), "an invalid pattern is matched literally")
    }

    func testTheLongDescriptionIsSearchedTooAsParameterEditorControllerDoes() {
        XCTAssertTrue(parameterMatches("RTL_ALT", ["Return altitude", "The minimum height to climb to before returning"], "climb"))
        XCTAssertFalse(parameterMatches("RTL_ALT", ["Return altitude"], "climb"))
    }

    func testSearchingIsCaseInsensitiveOnBothHalves() {
        XCTAssertTrue(parameterMatches("RTL_ALT", ["Return to launch altitude"], "rtl_alt"))
        XCTAssertTrue(parameterMatches("RTL_ALT", ["Return to launch altitude"], "LAUNCH"))
    }

    func testAParameterWhoseDescriptionHasNotLoadedYetStillMatchesByName() {
        XCTAssertTrue(parameterMatches("RTL_ALT", [""], "RTL"))
        XCTAssertFalse(parameterMatches("RTL_ALT", [""], "launch"))
    }

    func testAnEmptySearchKeepsEveryParameter() {
        XCTAssertTrue(parameterMatches("ANY", [""], ""))
    }
}

final class ParameterSubtitleTests: XCTestCase {
    func testAParameterSaysWhatItIsAndWhatItIsMeasuredIn() {
        XCTAssertEqual(parameterSubtitle("RTL Altitude", "cm"), "RTL Altitude · cm")
    }

    func testEitherHalfStandsAloneAndNeitherLeavesAStraySeparator() {
        XCTAssertEqual(parameterSubtitle("RTL Altitude", ""), "RTL Altitude")
        XCTAssertEqual(parameterSubtitle("", "cm"), "cm")
        XCTAssertEqual(parameterSubtitle("", ""), "")
    }

    func testParametersFallIntoCategoriesAndGroupsTheWayTheQtEditorBuildsItsTree() {
        let placement: [String: (String, String)] = [
            "A_ONE": ("Advanced", "Misc"),
            "A_TWO": ("Advanced", "Attitude"),
            "B_ONE": ("Other", "Misc"),
            "C_ONE": ("Standard", "Battery"),
        ]
        let tree = parameterTree(["A_ONE", "A_TWO", "B_ONE", "C_ONE"], placement)
        XCTAssertEqual(tree.map(\.name), ["Standard", "Advanced", "Other"], "Standard first, the default category last")
        XCTAssertEqual(tree[1].groups, ["Attitude", "Misc"], "the default group last")
        XCTAssertTrue(inGroup("A_TWO", placement, "Advanced", "Attitude"))
        XCTAssertFalse(inGroup("A_ONE", placement, "Advanced", "Attitude"))
    }

    func testEveryComponentIsListedAndANonAutopilotParameterCarriesItsComponent() {
        XCTAssertEqual(parameterKeys([(1, ["B", "A"]), (154, ["MNT_TYPE"])]), ["A", "B", "154:MNT_TYPE"])
        XCTAssertEqual(parameterPath("154:MNT_TYPE"), "vehicle.parameterManager.getParameter(154,MNT_TYPE)")
        XCTAssertEqual(parameterPath("RTL_ALT"), "vehicle.parameterManager.getParameter(-1,RTL_ALT)")
        XCTAssertTrue(parameterMatches("154:MNT_TYPE", [], "^MNT"), "an anchored search matches the parameter name, not its component prefix")
    }
}
