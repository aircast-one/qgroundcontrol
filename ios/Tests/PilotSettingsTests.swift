import XCTest
@testable import Aircast

private func parameter(_ name: String, _ description: String = "Engineer description") -> Fact {
    Fact(
        path: "vehicle.parameterManager.getParameter(-1,\(name))",
        name: name,
        description: description,
        units: "m",
        valueString: "30",
        value: .number(30),
        enumStrings: [],
        enumIndex: -1,
        isBool: false,
        isString: false,
        readOnly: false,
        minString: "",
        maxString: "",
        minIsDefaultForType: true,
        maxIsDefaultForType: true,
        defaultValueString: ""
    )
}

final class PilotSettingsTests: XCTestCase {
    private let returnHome = pilotSettings(.Safety).first { $0.label == "Return-to-home altitude" }!

    func testAPilotSettingUsesWhicheverFirmwareParameterTheAircraftReportsUnderThePilotsWord() {
        let px4 = firstReported(returnHome) { $0 == "RTL_RETURN_ALT" ? parameter($0) : nil }
        let ardupilot = firstReported(returnHome) { $0 == "RTL_ALT" ? parameter($0) : nil }
        XCTAssertEqual(px4?.name, "RTL_RETURN_ALT")
        XCTAssertEqual(ardupilot?.name, "RTL_ALT")
        XCTAssertEqual(px4?.heading, "Return-to-home altitude")
    }

    func testTheEngineerDescriptionMovesBehindHelpOnceTheRowHasAPilotLabel() {
        XCTAssertEqual(firstReported(returnHome) { parameter($0) }?.detail, "Engineer description")
    }

    func testTheLookupStopsAtTheFirstParameterTheAircraftReports() {
        let asked = AskedNames()
        _ = firstReported(returnHome) { name in
            asked.add(name)
            return parameter(name)
        }
        XCTAssertEqual(asked.names, Array(returnHome.parameters.prefix(1)))
    }

    func testNothingShowsWhenTheAircraftReportsNoneOfTheParameters() {
        XCTAssertNil(firstReported(returnHome) { _ in nil })
    }

    func testOnlySafetyAndControlCarryAPilotList() {
        XCTAssertFalse(pilotSettings(.Safety).isEmpty)
        XCTAssertFalse(pilotSettings(.Control).isEmpty)
        XCTAssertEqual(pilotSettings(.Camera), [])
    }

    func testSettingsRowsReadInPilotWordsAndUnknownRowsKeepTheirs() {
        XCTAssertEqual(pilotWorded(parameter("goToLocationRequiresConfirmInGuided")).heading, "Confirm fly-to first")
        XCTAssertEqual(pilotWorded(parameter("somethingElse")).shortLabel, "")
    }

    func testDesktopOnlySetupPagesFoldAwayFromTheOnesThatOpenHere() {
        let components = ["Airframe", "Sensors", "PID Tuning"].enumerated().map { index, name in
            SetupComponent(index: index, name: name, needsAttention: false)
        }
        let (here, desktop) = splitDesktopOnly(components) { $0.name == SENSORS }
        XCTAssertEqual(here.map(\.name), [SENSORS])
        XCTAssertEqual(desktop.map(\.name), ["Airframe", "PID Tuning"])
    }

    func testSafetyListsCompassImuAndGyroInDjisOrderWithTheirHealth() {
        let view = JSON.parse(
            #"""
            {"class":"Calibration","routines":[
                {"id":"gyro","title":"Gyroscope","status":"Calibrated"},
                {"id":"levelHorizon","title":"Level Horizon","status":""},
                {"id":"compass","title":"Compass","status":"Not calibrated"},
                {"id":"accelerometer","title":"Accelerometer","status":"Calibrated"}]}
            """#
        )
        let checks = sensorChecks(calibrationState(view))
        XCTAssertEqual(checks.map(\.id), ["compass", "accelerometer", "gyro"])
        XCTAssertEqual(checks.map(sensorHealthy), [false, true, true])
        XCTAssertEqual(sensorChecks(nil).map(\.id), [])
    }

    func testFailsafeChoicesReadInDjisWords() {
        XCTAssertEqual(pilotChoice("Hold mode"), "Hover")
        XCTAssertEqual(pilotChoice("Return mode"), "Return home")
        XCTAssertEqual(pilotChoice("Terminate"), "Stop motors")
        XCTAssertEqual(pilotChoice("Enabled always RTL"), "Enabled always RTL")
    }

    func testSearchFindsAPilotSettingByItsWordsAndByEitherFirmwaresParameterName() {
        XCTAssertEqual(pilotSearchHits("return").map(\.label), ["Return-to-home altitude"])
        XCTAssertEqual(pilotSearchHits("rtl_alt").map(\.label), ["Return-to-home altitude"])
        XCTAssertEqual(pilotSearchHits("MPC_XY_VEL_MAX").map(\.label), ["Max horizontal speed"])
        XCTAssertEqual(pilotSearchHits("return").map(\.section), ["Safety \u{203a} Return to home"])
        XCTAssertTrue(pilotSearchHits("  ").isEmpty)
    }

    func testArduCopterSpeedLimitsReadInMetresPerSecondUnderBoth45CentimetreAnd47MetreNames() {
        let limits = pilotSettings(.Control)
        let names = ["LOIT_SPEED", "LOIT_SPEED_MS", "PILOT_SPEED_UP", "PILOT_SPD_UP", "PILOT_SPEED_DN", "PILOT_SPD_DN"]
        let asked = names.map { name in limits.first { $0.parameters.contains(name) }.map { parameterPath(name, $0.quantity) } }
        XCTAssertEqual(asked, names.map { "vehicle.parameterManager.getParameter(-1,\($0),speed)" })
    }

    func testAPilotRowWritesThroughTheConvertingPathItWasReadFromSoMetresPerSecondLandAsCentimetres() {
        let read = JSON.parse(#"{"kind":"fact","name":"LOIT_SPEED","units":"m/s","valueString":"10","value":10}"#)
        let horizontal = pilotSettings(.Control).first { $0.label == "Max horizontal speed" }!
        let fact = firstReported(horizontal) { name in name == "LOIT_SPEED" ? factFromParameter(name, read, "speed") : nil }
        XCTAssertEqual(fact?.path, "vehicle.parameterManager.getParameter(-1,LOIT_SPEED,speed)")
        XCTAssertEqual(fact?.units, "m/s")
    }

    func testEveryLimitInAUnitReadsInThePilotsUnitAndFailsafeChoicesStayRaw() {
        let quantities = Dictionary(uniqueKeysWithValues: (pilotSettings(.Safety) + pilotSettings(.Control)).map { ($0.label, $0.quantity) })
        XCTAssertEqual(quantities, [
            "Return-to-home altitude": "altitude",
            "Max altitude": "altitude",
            "Max distance": "distance",
            "Signal lost": nil,
            "Remote controller lost": nil,
            "Low battery": nil,
            "Max horizontal speed": "speed",
            "Max climb speed": "speed",
            "Max descent speed": "speed",
        ])
    }

    func testOfflineSafetyStillListsItsSectionsInDjisOrderSoThePilotKnowsTheyExist() {
        XCTAssertEqual(pilotSections(pilotSettings(.Safety)).map(\.key), ["Return to home", "Flight protection", "If something goes wrong"])
    }
}

private final class AskedNames {
    private(set) var names: [String] = []
    func add(_ name: String) { names = names + [name] }
}
