import XCTest
@testable import Aircast

final class ParameterToolsTests: XCTestCase {
    func testToolsComeFromTheCoreInTheEditorsOrder() {
        let tools = parameterTools(JSON.parse(
            #"{"tools":[{"path":"parameterTools.refresh","label":"Refresh","confirm":false,"confirmTitle":"","confirmMessage":""},"#
                + #"{"path":"parameterTools.resetDefaults","label":"Reset all to firmware's defaults","confirm":true,"confirmTitle":"Reset All","confirmMessage":"m"},"#
                + #"{"path":"vehicle.rebootVehicle","label":"Reboot Vehicle","confirm":true,"confirmTitle":"Reboot Vehicle","confirmMessage":"Select Ok to reboot vehicle."}]}"#
        ))
        XCTAssertEqual(tools.map(\.label), ["Refresh", "Reset all to firmware's defaults", "Reboot Vehicle"])
        XCTAssertFalse(tools[0].confirm)
        XCTAssertEqual(confirmLabel(tools[1]), "Reset")
        XCTAssertEqual(confirmLabel(tools[2]), "Ok")
    }

    func testTheModifiedFilterKeepsOnlyParametersChangedFromStock() {
        XCTAssertTrue(parameterShown("RTL_ALT", [], "", true, ["RTL_ALT"]))
        XCTAssertFalse(parameterShown("RTL_SPEED", [], "", true, ["RTL_ALT"]))
        XCTAssertTrue(parameterShown("RTL_SPEED", [], "rtl", false, []))
    }

    func testTheModifiedFilterOnlyAppliesToPx4LikeParameterEditor() {
        XCTAssertTrue(modifiedFilterOn(true, true))
        XCTAssertFalse(modifiedFilterOn(true, false))
        XCTAssertFalse(modifiedFilterOn(false, true))
    }

    func testAReviewListsTheRowsAndWarnsAboutAnotherVehicle() throws {
        let review = try XCTUnwrap(parameterReview(JSON.parse(
            #"{"otherVehicle":true,"multipleComponents":false,"rows":["#
                + #"{"name":"RTL_ALT","fileValue":"2000","vehicleValue":"1500","units":"cm","cannotSend":false},"#
                + #"{"name":"MP_ONLY","fileValue":"4","vehicleValue":"","units":"","cannotSend":true}]}"#
        )))
        XCTAssertEqual(review.rows.map(\.name), ["RTL_ALT", "MP_ONLY"])
        XCTAssertEqual(reviewWarnings(review), ["The parameters in the file are from a different vehicle."])
        XCTAssertEqual(diffLine(review.rows[0]), "Vehicle 1500 · File 2000 · cm")
        XCTAssertEqual(diffLine(review.rows[1]), "Vehicle N/A — not on Vehicle · File 4")
    }

    func testTheSummaryCountsEveryClauseLikeParameterDiffDialog() throws {
        let review = try XCTUnwrap(parameterReview(JSON.parse(
            #"{"parsed":9,"unchanged":3,"readOnly":1,"rows":["#
                + #"{"name":"A","cannotSend":false},{"name":"B","cannotSend":false,"noVehicleValue":true},"#
                + #"{"name":"C","cannotSend":true,"noVehicleValue":true}]}"#
        )))
        XCTAssertEqual(
            reviewSummary(review),
            "Loaded 9 parameters from file: 2 will be changed (including 1 not currently on the Vehicle), 3 already match the Vehicle, "
                + "1 read-only parameter will not be sent, 1 not found on the Vehicle and cannot be sent."
        )
        XCTAssertEqual(sendableCount(review), 2)
        XCTAssertEqual(reviewSummary(ParameterReview(rows: [], otherVehicle: false, multipleComponents: false, parsed: 1, unchanged: 1)), "Loaded 1 parameter from file: 1 already matches the Vehicle.")
        XCTAssertEqual(reviewSummary(ParameterReview(rows: [], otherVehicle: false, multipleComponents: false, parsed: 2, readOnly: 2)), "Loaded 2 parameters from file: 2 read-only parameters will not be sent.")
        XCTAssertEqual(reviewSummary(ParameterReview(rows: [], otherVehicle: false, multipleComponents: false, parsed: 0)), "Loaded 0 parameters from file.")
    }

    func testCheckAllTogglesOnlyTheRowsThatCanBeSent() throws {
        let review = try XCTUnwrap(parameterReview(JSON.parse(#"{"rows":[{"name":"A","cannotSend":false},{"name":"C","cannotSend":true}]}"#)))
        XCTAssertEqual(checkedAll(review, ["1:A"], false), [])
        XCTAssertEqual(checkedAll(review, [], true), ["1:A"])
    }

    func testAFileRowIsKeyedByComponentAndAParameterNewToTheVehicleSaysSo() {
        let gimbal = ParameterDiffRow(json: .object([:]), name: "MNT_TYPE", fileValue: "1", vehicleValue: "", units: "", cannotSend: false, componentId: 154, noVehicleValue: true)
        var autopilot = gimbal
        autopilot.componentId = 1
        autopilot.noVehicleValue = false
        autopilot.vehicleValue = "0"
        XCTAssertNotEqual(gimbal.key, autopilot.key)
        XCTAssertEqual(diffLine(gimbal), "Vehicle N/A — new to Vehicle · File 1")
    }

    func testTheCountLineReadsLikeThePenpotHeader() {
        XCTAssertEqual(parameterCountLine(1204, 3), "1,204 parameters \u{00b7} 3 changed")
        XCTAssertEqual(parameterCountLine(1, 0), "1 parameter")
    }
}
