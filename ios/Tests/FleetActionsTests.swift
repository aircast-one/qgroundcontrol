import XCTest
@testable import Aircast

final class FleetActionsTests: XCTestCase {
    private func served(_ entries: String...) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Vehicles","actions":["# + entries.joined(separator: ",") + "]}")
    }

    private func action(_ id: String, _ offer: String, _ reason: String = "") -> String {
        #"{"id":"\#(id)","title":"\#(id)","prompt":"do \#(id)","offer":"\#(offer)","reason":"\#(reason)"}"#
    }

    func testAnAbsentActionsListReadsAsNoFleetActions() {
        XCTAssertEqual(mvActions(JSON.parse(#"{"kind":"object"}"#)), [])
        XCTAssertEqual(mvActions(nil), [])
    }

    func testEveryServedActionIsOfferedInTheOrderTheCoreListedThem() {
        let actions = mvActions(served(action("mvArm", "ready"), action("mvDisarm", "blocked", "No selected vehicle is armed.")))
        XCTAssertEqual(actions.map(\.id), ["mvArm", "mvDisarm"])
        XCTAssertTrue(actions.first!.ready)
        XCTAssertFalse(actions.last!.ready)
    }

    func testAnEntryWithoutAnIdIsNotOffered() {
        XCTAssertEqual(mvActions(served(#"{"title":"Arm","offer":"ready","prompt":"","reason":""}"#)), [])
    }

    func testAReasonIsShownOnlyWhileTheActionIsBlocked() throws {
        let blocked = try XCTUnwrap(mvActions(served(action("mvPause", "blocked", "No selected vehicle supports being paused."))).first)
        XCTAssertEqual(mvReasonFor(blocked), "No selected vehicle supports being paused.")
        let ready = try XCTUnwrap(mvActions(served(action("mvPause", "ready", "No selected vehicle supports being paused."))).first)
        XCTAssertNil(mvReasonFor(ready))
    }

    func testAnEmptyReasonOnABlockedActionSaysNothingRatherThanAnEmptyLine() throws {
        XCTAssertNil(mvReasonFor(try XCTUnwrap(mvActions(served(action("mvArm", "blocked"))).first)))
    }

    func testTheHeadingListsTheSelectedVehicleIdsLikeFlyViewTopRightPanel() {
        XCTAssertEqual(fleetHeading([]), "Vehicles Selected: -")
        XCTAssertEqual(fleetHeading([1, 3]), "Vehicles Selected: 1, 3")
    }

    func testThePanelShowsForTwoOrMoreVehiclesWithTheSettingOn() {
        XCTAssertEqual(fleetPanelShown(1, true), false)
        XCTAssertEqual(fleetPanelShown(2, false), false)
        XCTAssertEqual(fleetPanelShown(2, true), true)
    }

    func testAReadyActionSaysWhatItDoes() throws {
        XCTAssertEqual(fleetActionLine(try XCTUnwrap(mvActions(served(action("mvArm", "ready"))).first)), "do mvArm")
    }

    func testABlockedActionShowsTheRefusalInPlaceOfThePrompt() throws {
        let blocked = try XCTUnwrap(mvActions(served(action("mvDisarm", "blocked", "No selected vehicle is armed."))).first)
        XCTAssertEqual(fleetActionLine(blocked), "No selected vehicle is armed.")
    }

    func testAnActionTheCoreSentNoPromptForFallsBackToItsTitle() {
        XCTAssertEqual(fleetActionLine(MvAction(id: "mvArm", title: "Arm", confirmTitle: "Arm (MV)", prompt: "", offer: "ready", reason: "")), "Arm")
    }

    func testPauseIsTheOneFleetActionThatIsNotDestructive() {
        let destructive = ["mvArm", "mvDisarm", "mvStartMission", "mvPause"]
            .map { MvAction(id: $0, title: $0, confirmTitle: $0, prompt: "", offer: "ready", reason: "") }
            .filter(fleetIsDestructive)
            .map(\.id)
        XCTAssertEqual(destructive, ["mvArm", "mvDisarm", "mvStartMission"])
    }

    func testTheActiveVehiclesIdComesOffTheViewThatAlreadyNamesIt() {
        XCTAssertEqual(activeVehicleId(JSON.parse(#"{"kind":"object","activeId":7}"#)), 7)
    }

    func testNoVehicleIsNoIdAndNeverZero() {
        XCTAssertNil(activeVehicleId(nil))
        XCTAssertNil(activeVehicleId(JSON.parse(#"{"kind":"object","activeId":null}"#)))
        XCTAssertNil(activeVehicleId(JSON.parse(#"{"kind":"object"}"#)))
        XCTAssertNil(activeVehicleId(JSON.parse(#"{"kind":"object","activeId":0}"#)))
    }
}
