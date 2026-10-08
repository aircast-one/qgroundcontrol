import XCTest
@testable import Aircast

final class InspectorScreenTests: XCTestCase {
    private let path = "mavlinkInspector.activeSystem.messages.3"

    private func view(_ selectedPath: String) -> JSON {
        JSON.parse(#"""
            {"messages":[{"path":"\#(selectedPath)","selected":true}],
             "fields":[
               {"name":"roll","type":"float","value":"0.001"},
               {"name":"pitch","type":"float","value":"0.002"}
             ]}
            """#)
    }

    func testFieldsKeepTheirDeclaredOrder() {
        let fields = parseInspectorFields(view(path), path)
        XCTAssertEqual(fields.map(\.name), ["roll", "pitch"])
        XCTAssertEqual(fields[0], InspectorField(name: "roll", type: "float", value: "0.001"))
    }

    func testFieldsOfThePreviousSelectionAreNotDrawnUnderTheNewMessage() {
        XCTAssertTrue(parseInspectorFields(view("mavlinkInspector.activeSystem.messages.1"), path).isEmpty)
        XCTAssertTrue(parseInspectorFields(nil, path).isEmpty)
    }
}

final class InspectorOpenMessageTests: XCTestCase {
    private func message(_ index: Int, _ name: String) -> InspectorMessage {
        InspectorMessage(
            index: index,
            id: index,
            name: name,
            rateText: "1.0 Hz",
            count: 1,
            path: "mavlinkInspector.activeSystem.messages.\(index)",
            compId: 1,
            title: name
        )
    }

    func testTheOpenMessageFollowsItsNameWhenTheModelIsRebuiltAtOtherIndices() {
        let before = [message(0, "HEARTBEAT"), message(1, "ATTITUDE")]
        let after = [message(0, "SYS_STATUS"), message(1, "HEARTBEAT"), message(2, "ATTITUDE")]

        XCTAssertEqual(openMessageIn(before, "mavlinkInspector.activeSystem.messages.1")?.index, 1)
        XCTAssertEqual(openMessageIn(after, "mavlinkInspector.activeSystem.messages.2")?.index, 2)
    }

    func testAMessageThatLeavesTheModelClosesRatherThanShowingAnotherOne() {
        XCTAssertNil(openMessageIn([message(0, "HEARTBEAT")], "ATTITUDE"))
    }

    func testNothingOpenResolvesToNothing() {
        XCTAssertNil(openMessageIn([message(0, "HEARTBEAT")], nil))
    }

    func testTheTableReadsTheCoresRateTextAndPath() {
        let messages = inspectorMessages(JSON.parse(#"""
            {"messages":[
                {"index":1,"id":30,"name":"ATTITUDE","rateText":"10.0 Hz","count":420,
                 "path":"mavlinkInspector.activeSystem.messages.1"},
                {"index":0,"id":0,"name":"HEARTBEAT","rateText":"1.0 Hz","count":42,
                 "path":"mavlinkInspector.activeSystem.messages.0"}]}
            """#))

        XCTAssertEqual(messages.map(\.name), ["ATTITUDE", "HEARTBEAT"])
        XCTAssertEqual(messages.map(\.rateText), ["10.0 Hz", "1.0 Hz"])
        XCTAssertEqual(messages.map(\.index), [1, 0])
    }

    func testNoMessagesIsAnEmptyTableNotACrash() {
        XCTAssertEqual(inspectorMessages(nil), [])
        XCTAssertEqual(inspectorMessages(JSON.parse("{}")), [])
    }

    func testTwoComponentsSendingTheSameMessageAreDistinctRows() {
        let messages = inspectorMessages(JSON.parse(#"""
            {"messages":[
                {"index":0,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
                 "compId":100,"path":"mavlinkInspector.systems.1.messages.0"},
                {"index":1,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
                 "compId":101,"path":"mavlinkInspector.systems.1.messages.1"}]}
            """#))

        XCTAssertEqual(messages.count, 2)
        XCTAssertEqual(Set(messages.map(\.path)).count, 2)
    }

    func testTheRowTitleIsTheCoresFallingBackToTheNameWhenAbsent() {
        let served = inspectorMessages(JSON.parse(#"""
            {"messages":[
                {"index":0,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
                 "compId":100,"path":"p0","title":"CAMERA_CAPTURE_STATUS (comp 100)"},
                {"index":1,"id":30,"name":"ATTITUDE","rateText":"5.0 Hz","count":99,
                 "compId":1,"path":"p1"}]}
            """#))

        XCTAssertEqual(served.first { $0.path == "p0" }?.title, "CAMERA_CAPTURE_STATUS (comp 100)")
        XCTAssertEqual(served.first { $0.path == "p1" }?.title, "ATTITUDE")
    }

    func testTheFieldsAndTheSelectionFollowTheServedPathNotARebuiltOne() {
        let path = "mavlinkInspector.systems.0.messages.3"

        XCTAssertEqual(selectedPathFor(path), "mavlinkInspector.systems.0.selected")
        XCTAssertEqual(messageIndexIn(path), 3)
    }

    func testAPathTheCoreWordsDifferentlyStillYieldsItsOwnSystem() {
        XCTAssertEqual(selectedPathFor("mavlinkInspector.systems.2.messages.11"), "mavlinkInspector.systems.2.selected")
        XCTAssertEqual(messageIndexIn("mavlinkInspector.systems.2.messages.11"), 11)
    }
}

final class InspectorRateTests: XCTestCase {
    func testNoViewOffersNoRates() {
        XCTAssertTrue(inspectorRateChoices(nil).isEmpty)
    }

    func testRateChoicesComeFromTheCoreWithTheirTitles() {
        let choices = inspectorRateChoices(JSON.parse(#"{"rateChoices":[{"rate":-1,"title":"Disabled"},{"rate":0,"title":"Default"},{"rate":10,"title":"10 Hz"}]}"#))
        XCTAssertEqual(choices.count, 3)
        XCTAssertEqual(choices[0].rate, -1)
        XCTAssertEqual(choices[0].title, "Disabled")
        XCTAssertEqual(choices[2].title, "10 Hz")
    }

    func testAChoiceWithNoTitleIsNotOffered() {
        let choices = inspectorRateChoices(JSON.parse(#"{"rateChoices":[{"rate":5,"title":""},{"rate":6,"title":"6 Hz"}]}"#))
        XCTAssertEqual(choices.count, 1)
        XCTAssertEqual(choices[0].rate, 6)
    }

    func testTheTargetRateTitleIsDecodedOntoTheMessage() {
        let view = JSON.parse(#"{"messages":[{"index":0,"id":30,"name":"ATTITUDE","targetRateTitle":"10 Hz"}]}"#)
        XCTAssertEqual(inspectorMessages(view)[0].targetRateTitle, "10 Hz")
    }
}

final class InspectorSystemTests: XCTestCase {
    func testTheInspectorNamesTheSystemItsMessagesCameFrom() {
        let view = JSON.parse(#"{"kind":"object","class":"MavlinkInspector","available":true,"systemId":1,"messages":[]}"#)
        XCTAssertEqual(inspectorSystemText(view), "System 1")
    }

    func testNoSystemYetIsNoLineRatherThanSystem0() {
        XCTAssertNil(inspectorSystemText(JSON.parse(#"{"kind":"object","class":"MavlinkInspector","available":true,"systemId":0}"#)))
        XCTAssertNil(inspectorSystemText(JSON.parse(#"{"kind":"object","class":"MavlinkInspector","available":false,"systemId":3}"#)))
        XCTAssertNil(inspectorSystemText(nil))
    }

    func testTheEmptyStateIsTheCoresSentenceNotTheHeads() {
        let noVehicle = JSON.parse(#"{"kind":"object","class":"Inspector","available":false,"messages":[],"emptyText":"Connect a vehicle to inspect its MAVLink traffic."}"#)
        XCTAssertEqual(inspectorEmptyText(noVehicle), "Connect a vehicle to inspect its MAVLink traffic.", "witnessed on device with no vehicle")
    }

    func testAVehicleWithRowsHasNothingToSayInsteadOfThem() {
        let flowing = JSON.parse(#"{"kind":"object","class":"Inspector","available":true,"systemId":1,"messages":[{"name":"HEARTBEAT"}],"emptyText":""}"#)
        XCTAssertNil(
            inspectorEmptyText(flowing),
            "witnessed on device: twelve messages and emptyText empty - a blank sentence is the core saying draw the list, not saying nothing"
        )
    }

    func testAVehicleThatHasSentNothingYetSaysSoAndTheHeadDoesNotInventIt() {
        let waiting = JSON.parse(#"{"kind":"object","class":"Inspector","available":true,"systemId":1,"messages":[],"emptyText":"Waiting for this vehicle's first message…"}"#)
        XCTAssertEqual(
            inspectorEmptyText(waiting),
            "Waiting for this vehicle's first message…",
            "the window between activeSystem existing and the first frame is too short to sample on this rig, so the head must render whatever arrives rather than carry a second sentence of its own that could drift from it"
        )
    }

    func testTheComponentFilterNarrowsTheListTheWayTheInspectorComboDoes() {
        let heartbeat = InspectorMessage(index: 0, id: 0, name: "HEARTBEAT", rateText: "", count: 1, path: "p0", compId: 1, title: "HEARTBEAT")
        let camera = InspectorMessage(index: 1, id: 262, name: "CAMERA_CAPTURE_STATUS", rateText: "", count: 1, path: "p1", compId: 100, title: "CAMERA_CAPTURE_STATUS")
        XCTAssertEqual(inspectorShown([heartbeat, camera], "", 100), [camera])
        XCTAssertEqual(inspectorShown([heartbeat, camera], "", nil), [heartbeat, camera])
        XCTAssertEqual(inspectorChoices(JSON.parse(#"{"systems":[{"id":2,"title":"System 2"}]}"#), "systems"), [InspectorChoice(id: 2, title: "System 2")])
    }
}
