import XCTest
@testable import Aircast

final class ActuatorsScreenTests: XCTestCase {
    func testBitsetAndTrueIfPositiveWriteTheRawValueAsQgcsWrapperFactsDo() {
        XCTAssertTrue(bitsetChecked(5.0, 2))
        XCTAssertFalse(bitsetChecked(5.0, 1))
        XCTAssertEqual(bitsetWritten(5.0, 1, true), 7)
        XCTAssertEqual(bitsetWritten(5.0, 2, false), 1)
        XCTAssertEqual(signWritten(-1500.0, true), 1500.0)
        XCTAssertEqual(signWritten(1500.0, false), -1500.0)
    }

    func testGroupsSubgroupsAndChannelConfigsReadFromTheView() throws {
        let read = try XCTUnwrap(actuatorOutputs(JSON.parse(
            #"{"class":"ActuatorOutputs","available":true,"showUi":true,"groups":[{"label":"MAIN","enable":null,"groupsVisible":true,"params":[],"#
                + #""subgroups":[{"label":"MAIN 1-4","primary":null,"params":[],"columns":[{"label":"Function","advanced":false,"visible":true}],"#
                + #""channels":[{"label":"MAIN 1","configs":[null]}]}]}]}"#
        )))
        XCTAssertEqual(read.groups.count, 1)
        let subgroup = try XCTUnwrap(read.groups.first?.subgroups.first)
        XCTAssertEqual(subgroup.columns.map(\.label), ["Function"])
        XCTAssertEqual(subgroup.channels.first?.configs.count, 1)
        XCTAssertNil(subgroup.channels.first?.configs.first ?? nil)
    }
}

final class ActuatorGeometryTests: XCTestCase {
    func testFixedGeometryCellsReadAsReadOnlyValues() throws {
        let read = try XCTUnwrap(geometry(JSON.parse(
            #"{"title":"Geometry: Tiltrotor","helpUrl":"","groups":[{"label":"Motors","count":null,"params":[],"#
                + #""channels":[{"label":"Rear Motor (Motor 3)","cells":[{"fixed":true,"label":"Position X","valueString":"-0.7500","advanced":false},null]}]}]}"#
        )))
        let channel = try XCTUnwrap(read.groups.first?.channels.first)
        XCTAssertEqual(channel.label, "Rear Motor (Motor 3)")
        XCTAssertEqual(channel.cells[0], .Fixed(label: "Position X", valueString: "-0.7500", advanced: false))
        XCTAssertNil(channel.cells[1])
    }
}

final class ActuatorMixerCellTests: XCTestCase {
    func testAnAxisCellAndARuleHiddenCellReadFromTheGeometry() throws {
        let axis = geometryCell(JSON.parse(#"{"axis":true,"options":["Custom","Upwards"],"index":1,"params":["CA_MC_R0_AX","CA_MC_R0_AY","CA_MC_R0_AZ"],"hidden":false,"disabled":false,"advanced":true}"#))
        XCTAssertEqual(axis, .Axis(options: ["Custom", "Upwards"], index: 1, params: ["CA_MC_R0_AX", "CA_MC_R0_AY", "CA_MC_R0_AZ"], advanced: true, hidden: false, disabled: false))
        let hidden = try XCTUnwrap(geometryCell(JSON.parse(#"{"fixed":true,"label":"Pitch Torque","valueString":"0.0000","advanced":false,"hidden":true}"#)))
        XCTAssertTrue(hidden.hidden)
    }

    func testTheMixerIsEditableOnlyWhileSlidersAreOffAndNoMotorAssignmentRuns() {
        XCTAssertTrue(mixerEditable(false, false))
        XCTAssertFalse(mixerEditable(true, false))
        XCTAssertFalse(mixerEditable(false, true))
    }

    func testACellWithAMissingParameterReadsAsNotAvailable() {
        let missing = geometryCell(JSON.parse(#"{"unavailable":true,"label":"Roll Torque","advanced":false,"hidden":false,"disabled":false,"channelFunction":201,"param":"CA_SV_CS0_TRQ_R"}"#))
        XCTAssertEqual(missing, .Unavailable(label: "Roll Torque", advanced: false, hidden: false))
    }
}

final class MotorAssignmentHeadTests: XCTestCase {
    func testTheAssignmentMessageLosesItsMarkupAndTheHighlightedMotorsReadAsASet() {
        XCTAssertEqual(plainMessage("a<br /><br /><b>Warning</b><br />b"), "a\n\nWarning\nb")
        let state = motorAssignment(JSON.parse(#"{"multirotor":true,"enabled":true,"active":true,"message":"m","highlighted":[1,3]}"#))
        XCTAssertEqual(state.highlighted, [1, 3])
        XCTAssertTrue(state.active)
    }
}
