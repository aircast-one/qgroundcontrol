import XCTest
@testable import Aircast

final class CalibrationViewTests: XCTestCase {
    func testTheStepLineCountsThePositionsReachedAmongTheVisibleOnes() {
        func side(_ key: String, _ stage: String, visible: Bool = true) -> CalibrationSide {
            CalibrationSide(key: key, title: key, visible: visible, stage: stage, rotate: false)
        }
        let sides = [side("down", "done"), side("up", "done"), side("left", "inProgress"), side("right", "pending"), side("nose", "pending", visible: false)]

        XCTAssertEqual(positionsText(sides), "3 of 4 positions")
        XCTAssertEqual(positionsText(sides.map { var pending = $0; pending.stage = "pending"; return pending }), "1 of 4 positions")
        XCTAssertNil(positionsText([]))
    }

    private let served = JSON.parse(
        #"""
        {"kind":"object","class":"Calibration","connected":true,"inProgress":false,
            "busy":false,"showsSides":false,"nextEnabled":false,"cancelEnabled":false,
            "progress":0.0,"progressText":"0%","helpText":"","statusText":"",
            "accelNeeded":true,"compassNeeded":true,
            "needsAttention":"The accelerometer and compass both need calibrating.",
            "sides":[{"key":"Down","title":"Level","visible":false,"stage":"waiting","rotate":false}],
            "routines":[
              {"id":"accelerometer","title":"Accelerometer","invocation":"sensorsCal.calibrateAccel",
               "arguments":[false],"blocked":false,"enabled":true,
               "description":"Hold the vehicle in each orientation it asks for.","status":"Not calibrated","warning":""},
              {"id":"compass","title":"Compass","invocation":"sensorsCal.calibrateCompass",
               "arguments":[],"blocked":true,"enabled":false,
               "description":"Calibrate the accelerometer first.","status":"Calibrate the accelerometer first.","warning":""}]}
        """#
    )

    private func with(_ json: JSON, _ changes: [String: JSON]) -> JSON {
        .object((json.object ?? [:]).merging(changes) { _, new in new })
    }

    func testTheRoutinesComeFromTheCoreWithTheCallItSaysToMake() throws {
        let state = try XCTUnwrap(calibrationState(served))

        XCTAssertEqual(state.routines.map(\.id), ["accelerometer", "compass"])
        XCTAssertEqual(state.routines[0].invocation, "sensorsCal.calibrateAccel")
        XCTAssertEqual(state.routines[0].arguments, [false])
        XCTAssertEqual(state.routines[1].arguments, [])
    }

    func testTheAccelerometerFirstRuleAndEachRoutinesStatusAreTheCoresAnswerNotTheHeads() throws {
        let state = try XCTUnwrap(calibrationState(served))

        XCTAssertFalse(state.routines[0].blocked)
        XCTAssertTrue(state.routines[1].blocked)
        XCTAssertEqual(state.routines[1].status, "Calibrate the accelerometer first.")
        XCTAssertEqual(state.routines[0].status, "Not calibrated")
    }

    func testAPx4RoutineOpensWithTheDialogTextQgcShows() throws {
        let px4 = try XCTUnwrap(calibrationState(with(served, ["px4": .bool(true), "settingsTitle": .string("Orientations")])))
        var gyro = px4.routines[0]
        gyro.id = "gyro"
        gyro.dialogHelp = "For Gyroscope calibration you will need to place your vehicle on a surface and leave it still."

        XCTAssertTrue(px4.px4)
        XCTAssertEqual(px4.settingsTitle, "Orientations")
        XCTAssertEqual(routineCopy(gyro).instruction, gyro.dialogHelp)
    }

    func testTheInstructionIsTheCoresDialogTextElseItsDescription() throws {
        let state = try XCTUnwrap(calibrationState(served))
        var routine = state.routines[1]
        routine.dialogHelp = ""
        routine.description = "Rotate the vehicle until every side is done."

        XCTAssertEqual(routineCopy(routine).instruction, "Rotate the vehicle until every side is done.")
    }

    func testSidesArriveWithTheirStageRatherThanThreeSeparateFlags() throws {
        let running = try XCTUnwrap(calibrationState(JSON.parse(
            #"""
            {"class":"Calibration","sides":[
                {"key":"Down","title":"Level","visible":true,"stage":"done","rotate":false},
                {"key":"Left","title":"Left side","visible":true,"stage":"inProgress","rotate":true}]}
            """#
        )))

        XCTAssertEqual(running.sides.map(\.stage), ["done", "inProgress"])
        XCTAssertEqual(running.sides.map(\.rotate), [false, true])
    }

    func testAPayloadThatIsNotTheCalibrationViewYieldsNothing() {
        XCTAssertNil(calibrationState(nil))
        XCTAssertNil(calibrationState(JSON.parse("{}")))
        XCTAssertNil(calibrationState(JSON.parse(#"{"class":"Vehicle"}"#)))
    }

    func testAPayloadUsingInventedNamesLeavesTheStateEmptyRatherThanPlausible() throws {
        let invented = JSON.parse(
            #"""
            {"class":"Calibration","calibrations":[{"id":"accelerometer"}],
                "orientations":[{"key":"Down"}],"inProgressNow":true}
            """#
        )
        let state = try XCTUnwrap(calibrationState(invented))

        XCTAssertEqual(state.routines, [])
        XCTAssertEqual(state.sides, [])
        XCTAssertFalse(state.inProgress)
    }

    func testAPendingCancelSaysSoLikeSensorsSetupsCalibrationCancelDialog() throws {
        XCTAssertFalse(try XCTUnwrap(calibrationState(served)).waitingForCancel)
        XCTAssertTrue(try XCTUnwrap(calibrationState(with(served, ["waitingForCancel": .bool(true)]))).waitingForCancel)
        XCTAssertEqual(CANCEL_WAIT_TEXT, "Waiting for Vehicle to response to Cancel. This may take a few seconds.")
    }

    func testSensorHealthListsNamedSensorsAndSaysHowManyAreFailing() throws {
        XCTAssertNil(sensorHealth(JSON.parse(#"{"class":"Calibration"}"#)))
        let reading = try XCTUnwrap(sensorHealth(JSON.parse(#"""
            {"class":"SensorHealth","available":true,"status":"ok",
            "sensors":[{"name":"Gyro","state":"healthy","label":"OK"},{"name":"","state":"healthy"},3],
            "failing":["Compass"," "]}
            """#)))
        XCTAssertEqual(reading.sensors, [SensorHealth(name: "Gyro", state: "healthy", label: "OK")], "a nameless or non-object sensor is dropped")
        XCTAssertEqual(reading.failing, ["Compass"], "a blank failing name is dropped")
        XCTAssertEqual(healthSummary(reading), "Compass is reporting a fault.")
        XCTAssertEqual(healthSummary(SensorHealthReading(available: true, sensors: [], failing: ["Compass", "Gyro"], status: "")), "2 sensors are reporting faults.")
        XCTAssertEqual(healthSummary(SensorHealthReading(available: true, sensors: [], failing: [], status: "")), "")
        XCTAssertEqual(healthSummary(SensorHealthReading(available: false, sensors: [], failing: ["Compass"], status: "")), "", "an unavailable reading says nothing")
        XCTAssertEqual(healthSummary(nil), "")
    }
}
