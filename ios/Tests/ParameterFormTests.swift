import XCTest
@testable import Aircast

private func changed(_ fact: Fact, _ change: (inout Fact) -> Void) -> Fact {
    var copy = fact
    change(&copy)
    return copy
}

final class ParameterFormTests: XCTestCase {
    private func control(_ extra: String = "") -> JSON {
        JSON.parse(#"{"kind":"control","path":"settings.appSettings.enforceChecklist","name":"enforceChecklist","label":"Enforce checklist","control":"toggle","value":true"# + extra + "}")
    }

    func testAControlServedWithoutTheGateIsWritableBecauseEveryOlderCoreOmitsIt() {
        let fact = factFromControl(control())!
        XCTAssertTrue(
            fact.enabled,
            "enabled and disabledReason are additive; every control from a core that predates them "
                + "arrives with neither, and a head that read a missing gate as false would grey out "
                + "the whole of Settings"
        )
        XCTAssertTrue(fact.acceptsWrite)
    }

    func testASectionsIconIsReadFromTheAssetFolderTheBuildCopiesItInto() {
        XCTAssertEqual(sectionImageAsset("Battery.svg"), "SetupSections/Battery")
    }

    func testAnIndentedSmallFontHelpRowKeepsBothCues() {
        let help = factFromControl(control(#","indent":true,"smallFont":true"#))!
        XCTAssertTrue(help.indent && help.smallFont)
        let plain = factFromControl(control())!
        XCTAssertFalse(plain.indent || plain.smallFont)
    }

    func testAControlTheCoreSaysIsInertIsNotOfferedAndSaysWhy() {
        let fact = factFromControl(control(#","enabled":false,"disabledReason":"Turn on Use preflight checklist first""#))!
        XCTAssertFalse(fact.acceptsWrite)
        XCTAssertEqual(inertNote(fact), "Turn on Use preflight checklist first")
    }

    func testReadOnlyAndInertAreDifferentAnswersAndSayDifferentThings() {
        let readOnly = factFromControl(control(#","readOnly":true"#))!
        XCTAssertEqual(inertNote(readOnly), "Read-only")
        let inert = factFromControl(control(#","enabled":false"#))!
        XCTAssertEqual(
            inertNote(inert),
            "Has no effect yet",
            "the flag decides, never the presence of the string - a reason drawn whenever it is "
                + "non-empty can explain why a row is off while it is on"
        )
    }

    func testAFactResponseBecomesAFactAddressedByItsParameterPath() {
        let json = JSON.parse(#"{"kind":"fact","name":"RTL_ALT","shortDescription":"Return altitude","units":"cm","valueString":"1500","value":1500,"typeIsBool":false,"readOnly":false}"#)
        let fact = factFromParameter("RTL_ALT", json)!
        XCTAssertEqual(fact.path, parameterPath("RTL_ALT"))
        XCTAssertEqual(fact.name, "RTL_ALT")
        XCTAssertEqual(fact.description, "Return altitude")
        XCTAssertEqual(fact.units, "cm")
    }

    func testAParameterWithNoDescriptionIsLabelledByItsName() {
        let fact = factFromParameter("MNT_ANGMIN_PAN", JSON.parse(#"{"kind":"fact","name":"MNT_ANGMIN_PAN","shortDescription":"","valueString":"0"}"#))!
        XCTAssertEqual(fact.name, "MNT_ANGMIN_PAN")
        XCTAssertEqual(fact.title, "MNT_ANGMIN_PAN")
    }

    func testADescriptionStillWinsOverTheName() {
        XCTAssertEqual(factFromParameter("RTL_ALT", JSON.parse(#"{"kind":"fact","name":"RTL_ALT","shortDescription":"Return altitude"}"#))!.title, "Return altitude")
    }

    func testAParameterTheVehicleDoesNotHaveIsSkipped() {
        XCTAssertNil(factFromParameter("NOPE", JSON.parse(#"{"kind":"value","value":null}"#)))
        XCTAssertNil(factFromParameter("NOPE", JSON.parse(#"{"kind":"null"}"#)))
        XCTAssertNil(factFromParameter("NOPE", JSON.parse("{}")))
        XCTAssertNil(
            factFromParameter("RTL_RETURN_ALT", JSON.parse(#"{"kind":"fact","name":"","shortDescription":"","valueString":"0","value":0}"#)),
            "the core answers a parameter it does not have with an unnamed 0, which must not pass for a value"
        )
    }

    func testAnEmptyErrorFromQgcMeansTheValueIsAcceptable() {
        XCTAssertNil(validationMessage(.string("")))
        XCTAssertNil(validationMessage(.string("   ")))
    }

    func testQgcsOwnWordingIsPassedThroughUnchanged() {
        XCTAssertEqual(validationMessage(.string("Value must be within 0 and 100")), "Value must be within 0 and 100")
    }

    func testABridgeCallThatReturnedNothingDoesNotBlockTheWrite() {
        XCTAssertNil(validationMessage(nil))
        XCTAssertNil(validationMessage(.null))
    }

    func testANonStringResultIsNotTreatedAsAnError() {
        XCTAssertNil(validationMessage(.bool(false)))
        XCTAssertNil(validationMessage(.number(0)))
    }

    private func ranged(
        min: String = "",
        max: String = "",
        minDefault: Bool = true,
        maxDefault: Bool = true,
        default fallback: String = "",
        vehicleReboot: Bool = false,
        qgcReboot: Bool = false
    ) -> Fact {
        Fact(
            path: "p", name: "p", description: "", units: "", valueString: "1",
            value: .number(1), enumStrings: [], enumIndex: 0,
            isBool: false, isString: false, readOnly: false,
            minString: min, maxString: max,
            minIsDefaultForType: minDefault, maxIsDefaultForType: maxDefault,
            defaultValueString: fallback,
            vehicleRebootRequired: vehicleReboot, qgcRebootRequired: qgcReboot
        )
    }

    func testALimitThatIsOnlyTheTypesOwnLimitIsNotShown() {
        XCTAssertNil(factConstraintNote(ranged(min: "0", max: "4294967295")))
    }

    func testARealConstraintIsShown() {
        XCTAssertEqual(factConstraintNote(ranged(max: "100", maxDefault: false)), "Max 100")
    }

    func testMinMaxAndDefaultReadTogether() {
        XCTAssertEqual(factConstraintNote(ranged(min: "6", max: "48", minDefault: false, maxDefault: false, default: "14")), "Min 6 · Max 48 · Default 14")
    }

    func testAVehicleRebootIsNamedAheadOfAnAppRestart() {
        XCTAssertEqual(factRebootNote(ranged(vehicleReboot: true, qgcReboot: true)), "Reboot vehicle for changes to take effect.")
    }

    func testAnAppRestartIsNamedWhenOnlyThatIsRequired() {
        XCTAssertEqual(factRebootNote(ranged(qgcReboot: true)), "Restart Aircast for this to take effect.")
    }

    func testAParameterNeedingNoRestartSaysNothing() {
        XCTAssertNil(factRebootNote(ranged()))
    }

    func testABitmaskOnASetupPageNamesItsBitsInsteadOfShowingANumber() {
        let fact = factFromControl(JSON.parse(
            #"""
            {"control":"bitmask","name":"SIMPLE","label":"Simple mode bitmask","path":"p",
             "value":5,"valueString":"5","display":"5",
             "bits":[{"label":"SwitchPos1","raw":"1","set":true},
                     {"label":"SwitchPos2","raw":"2","set":false},
                     {"label":"SwitchPos3","raw":"4","set":true}]}
            """#
        ))!
        XCTAssertTrue(fact.isBitmask)
        XCTAssertEqual(fact.bitmaskValues, [1, 2, 4])
        XCTAssertEqual(bitmaskSummary(fact), "SwitchPos1, SwitchPos3")
    }

    func testAFactTheCoreCalledAChoiceStaysAChoiceEvenWhenItCarriesBits() {
        let fact = factFromControl(JSON.parse(
            #"""
            {"control":"choice","name":"FS_OPTIONS","label":"Failsafe options","path":"p",
             "value":1,"valueString":"1","display":"Continue",
             "options":[{"label":"None","raw":"0"},{"label":"Continue","raw":"1"}],
             "bits":[{"label":"RC","raw":"1","set":true}]}
            """#
        ))!
        XCTAssertFalse(fact.isBitmask, "the core decides the control kind, the head does not re-derive it")
        XCTAssertTrue(fact.isEnum)
    }
}

final class ReadOnlyNoteTests: XCTestCase {
    private func fact(_ name: String, _ readOnly: Bool) -> Fact {
        Fact(
            path: "p/\(name)", name: name, description: "", units: "",
            valueString: "0", value: .number(0), enumStrings: [], enumIndex: -1,
            isBool: false, isString: false, readOnly: readOnly
        )
    }

    func testNothingReadOnlySaysNothing() {
        XCTAssertNil(readOnlyNote([fact("FLTMODE1", false), fact("FLTMODE2", false)]))
    }

    func testAllReadOnlySaysSoWithoutNamingThem() {
        XCTAssertEqual(
            readOnlyNote([fact("A", true), fact("B", true)]),
            "This firmware reports all of these as read-only, so they are shown for reference."
        )
    }

    func testSomeReadOnlyNamesWhichOnes() {
        XCTAssertEqual(
            readOnlyNote([fact("FLTMODE1", false), fact("FLTMODE2", true), fact("FLTMODE3", true)]),
            "This firmware reports FLTMODE2, FLTMODE3 as read-only, so they are shown but cannot be changed here."
        )
    }

    func testTheSetupPagesComeFromTheCoreGroupedAndFlagged() {
        let view = JSON.parse(
            #"""
            {"groups":[
              {"title":"Vehicle","pages":[
                {"name":"Frame","openable":true,"blockedReason":null,"parameterSections":true},
                {"name":"Motors","openable":true,"blockedReason":null,"parameterSections":false}]},
              {"title":"Support","pages":[
                {"name":"Remote Support","openable":true,"blockedReason":null,"parameterSections":false}]}]}
            """#
        )
        XCTAssertEqual(setupGroups(view).map(\.title), ["Vehicle", "Support"])
        XCTAssertEqual(setupPage(view, "Frame")?.parameterSections, true)
        XCTAssertEqual(setupPage(view, "Remote Support")?.parameterSections, false)
        XCTAssertNil(setupPage(view, "A page this firmware does not have"))
    }

    func testAControlBecomesARowWithTheCoresLabelAndOptions() {
        let fact = factFromControl(JSON.parse(
            #"""
            {"class":"Control","control":"choice","label":"Frame Class",
             "name":"FRAME_CLASS","display":"Quad","value":1,"valueString":"1","units":"",
             "path":"vehicle.parameterManager.getParameter(-1,FRAME_CLASS)","readOnly":false,
             "rebootRequired":true,"vehicleRebootRequired":true,"applicationRestartRequired":false,
             "options":[{"label":"Undefined","raw":"0"},{"label":"Quad","raw":"1"}]}
            """#
        ))!
        XCTAssertEqual(fact.title, "Frame Class")
        XCTAssertEqual(fact.enumStrings, ["Undefined", "Quad"])
        XCTAssertEqual(fact.enumIndex, 1)
        XCTAssertTrue(fact.vehicleRebootRequired)
    }

    func testAnAppSettingThatNeedsAnAppRestartDoesNotAskForAVehicleReboot() {
        let fact = factFromControl(JSON.parse(
            #"""
            {"class":"Control","control":"text","label":"Host name","name":"forwardMavlinkHostName",
             "display":"localhost:14445","value":"localhost:14445","valueString":"localhost:14445","units":"",
             "path":"settings.mavlinkSettings.forwardMavlinkHostName","readOnly":false,
             "rebootRequired":true,"vehicleRebootRequired":false,"applicationRestartRequired":true}
            """#
        ))!
        XCTAssertFalse(fact.vehicleRebootRequired)
        XCTAssertTrue(fact.qgcRebootRequired)
        XCTAssertEqual(factRebootNote(fact), "Restart Aircast for this to take effect.")
    }

    func testAControlTheVehicleDoesNotHaveIsDroppedRatherThanShownBlank() {
        XCTAssertNil(factFromControl(JSON.parse(
            #"""
            {"class":"Control","control":"number","label":"","name":"",
             "display":"0","value":0,"valueString":"0","units":"",
             "path":"vehicle.parameterManager.getParameter(-1,BATT_MONITOR)"}
            """#
        )))
    }

    func testThePagePathCarriesTheNameAndNoSeparatorsTheWatchWouldSplit() {
        XCTAssertEqual(setupPagePath("Flight Modes"), "view.setup(Flight Modes)")
        XCTAssertFalse(setupPagePath("Flight Modes").contains(","))
    }
}

final class ControlBoundsTests: XCTestCase {
    private func shown(_ value: String) -> String {
        value == "null" ? "null" : String(format: "\"%.2f\"", Double(value)!)
    }

    private func control(_ minimum: String, _ maximum: String, _ fallback: String) -> Fact {
        factFromControl(JSON.parse(
            #"{"kind":"object","class":"Control","control":"number","name":"FENCE_ALT_MAX","label":"Fence maximum altitude","path":"p","units":"m","value":100.0,"valueString":"100.00","options":[],"bits":[],"readOnly":false,"minimum":"#
                + minimum + #","maximum":"# + maximum + #","defaultValue":"# + fallback
                + #","minimumText":"# + shown(minimum) + #","maximumText":"# + shown(maximum) + #","defaultText":"# + shown(fallback) + "}"
        ))!
    }

    func testAControlThatDeclaresBoundsCarriesThemIntoTheFact() {
        XCTAssertEqual(factConstraintNote(control("10.0", "1000.0", "100.0")), "Min 10 · Max 1000 · Default 100")
    }

    func testABoundTheFactDoesNotDeclareIsNotPrintedAsTheTypesExtreme() {
        let fact = control("10.0", "null", "100.0")
        XCTAssertEqual(factConstraintNote(fact), "Min 10 · Default 100")
        XCTAssertTrue(fact.maxIsDefaultForType)
        XCTAssertFalse(fact.minIsDefaultForType)
    }

    func testAControlWithNoBoundsAtAllHasNoNoteRatherThanAnEmptyOne() {
        XCTAssertNil(factConstraintNote(control("null", "null", "null")))
    }

    func testABoundThatCanGoNegativeAsksForAKeyboardThatCanTypeAMinus() {
        XCTAssertEqual(factKeyboard(control("-180.0", "180.0", "0.0")), .numbersAndPunctuation)
        XCTAssertEqual(factKeyboard(control("0.0", "180.0", "0.0")), .decimalPad)
    }

    func testAStreamAddressGetsTheUrlKeyboardAndNoAutocorrect() {
        let address = changed(control("null", "null", "null")) {
            $0.name = "whepUrl"
            $0.isString = true
        }
        XCTAssertEqual(factKeyboard(address), .URL)
        XCTAssertTrue(isAddress(address))
        XCTAssertFalse(isAddress(changed(address) { $0.name = "primaryCameraName" }))
    }

    func testAnAspectRatioReadsAndAcceptsWidthColonHeight() {
        XCTAssertEqual(ratioText(1.777777), "16:9")
        XCTAssertEqual(ratioText(1.333333), "4:3")
        XCTAssertNil(ratioText(1.9))
        XCTAssertEqual(ratioValue("16:9"), "1.777778")
        XCTAssertEqual(ratioValue(" 4 : 3 "), "1.333333")
        XCTAssertNil(ratioValue("1.85"))
        XCTAssertNil(ratioValue("16:0"))
    }

    func testATuningSliderReadsItsSetupLabelAndTheLiveValueLikeThePenpotTuningFrame() {
        let fact = changed(control("0.0", "1.0", "0.5")) {
            $0.description = "Climb Sensitivity"
            $0.shortLabel = "Acceleration (vertical) controller P gain"
        }
        XCTAssertEqual(sliderTitle(fact), "Climb sensitivity")
        XCTAssertEqual(sliderValue(0.1349, 3, ""), "0.135")
        XCTAssertEqual(sliderValue(38, 0, "%"), "38 %")
    }
}
