import XCTest
@testable import Aircast

final class ActiveVehicleTests: XCTestCase {
    private let two = JSON.parse(#"""
        {"count":2,"activeId":1,"ambiguous":true,"vehicles":[
          {"id":1,"name":"Quadrotor 1","link":"TCP 127.0.0.1:5771","active":true,
           "armed":false,"flying":false,"flightMode":"Stabilize","contactLost":false},
          {"id":2,"name":"Quadrotor 2","link":"TCP 127.0.0.1:5772","active":false,
           "armed":true,"flying":true,"flightMode":"Guided","contactLost":true}
        ]}
        """#)

    private let one = JSON.parse(#"""
        {"count":1,"activeId":1,"ambiguous":false,"vehicles":[
          {"id":1,"name":"Quadrotor 1","link":"TCP 127.0.0.1:5771","active":true,
           "armed":false,"flying":false,"flightMode":"Stabilize","contactLost":false}
        ]}
        """#)

    func testTheHeaderNamesTheAircraftOnlyWhenMoreThanOneIsConnected() {
        XCTAssertEqual(activeVehicleTitle(vehicleChoices(one), "Stabilize · Disarmed"), "Stabilize · Disarmed")
        XCTAssertEqual(activeVehicleTitle(vehicleChoices(two), "Stabilize · Disarmed"), "Quadrotor 1 · Stabilize · Disarmed")
    }

    func testAVehicleOutOfContactSaysSoInsteadOfWhatItWasLastDoing() throws {
        let lost = try XCTUnwrap(vehicleChoices(two).choices.first { $0.id == 2 })
        XCTAssertEqual(vehicleChoiceLine(lost), "No contact · TCP 127.0.0.1:5772")
        XCTAssertTrue(lost.contactLost)
    }

    func testOneRadioCarryingTheWholeFleetPrintsItsNameUnderNobody() {
        let one = VehicleChoice(id: 1, name: "Quadrotor 1", state: "Stabilize · Disarmed", link: "SiK", contactLost: false, active: true, latitude: 0, longitude: 0)
        var two = one
        two.id = 2
        two.name = "Quadrotor 2"
        two.active = false
        XCTAssertFalse(linkDistinguishes([one, two]))
        XCTAssertEqual(vehicleChoiceLine(one, linkDistinguishes([one, two])), "Stabilize · Disarmed")
    }

    func testTwoRadiosKeepTheirNamesBecauseThatIsTheQuestionBeingAsked() {
        let a = VehicleChoice(id: 1, name: "A", state: "Disarmed", link: "TCP 5771", contactLost: false, active: true, latitude: 0, longitude: 0)
        var b = a
        b.id = 2
        b.name = "B"
        b.link = "TCP 5772"
        b.active = false
        b.contactLost = true
        XCTAssertTrue(linkDistinguishes([a, b]))
        XCTAssertEqual(vehicleChoiceLine(b, true), "No contact · TCP 5772")
    }

    func testAVehicleInContactSaysWhatItIsDoingAndWhichLinkCarriesIt() throws {
        let live = try XCTUnwrap(vehicleChoices(two).choices.first { $0.id == 1 })
        XCTAssertEqual(vehicleChoiceLine(live), "Stabilize · Disarmed · TCP 127.0.0.1:5771")
        XCTAssertTrue(live.active)
    }

    func testAVehicleThatHasNotReportedAModeStillSaysWhetherItIsArmed() {
        let early = JSON.parse(#"{"ambiguous":true,"vehicles":[{"id":4,"name":"Quadrotor 4","active":false,"contactLost":false,"flightMode":"","armed":true,"flying":false}]}"#)
        XCTAssertEqual(vehicleChoices(early).choices.map(\.state), ["Armed"])
    }

    func testFlyingOutranksArmedBecauseAnArmedVehicleOnTheGroundIsADifferentThing() {
        XCTAssertEqual(vehicleChoices(two).choices.first { $0.id == 2 }?.state, "Guided · Flying")
    }

    func testAContactLostTheCoreCouldNotAnswerIsNotReadAsALoss() {
        let unknown = JSON.parse(#"{"ambiguous":true,"vehicles":[{"id":3,"name":"Rover 3","active":false,"contactLost":null}]}"#)
        XCTAssertEqual(vehicleChoices(unknown).choices.map(\.contactLost), [false])
    }

    func testAnEmptyViewOffersNothingRatherThanAnEmptyName() {
        XCTAssertTrue(vehicleChoices(nil).choices.isEmpty)
        XCTAssertFalse(vehicleChoices(nil).ambiguous)
        XCTAssertEqual(activeVehicleTitle(vehicleChoices(nil), "No vehicle"), "No vehicle")
    }

    func testAVehicleTheCoreDidNotNameStillHasSomethingToTap() {
        XCTAssertEqual(vehicleChoices(JSON.parse(#"{"ambiguous":true,"vehicles":[{"id":7,"active":false}]}"#)).choices.map(\.name), ["Vehicle 7"])
    }

    func testAVehicleThatStoppedAnsweringIsNamedEvenWhileAnotherIsBeingFlown() {
        let lost = lostVehicles(vehicleChoices(two))
        XCTAssertEqual(lost.map(\.id), [2])
        XCTAssertEqual(lostVehiclesText(lost), "Quadrotor 2 is not answering")
    }

    func testTheVehicleBeingFlownIsNotCountedAmongTheSilentOnes() {
        let activeLost = JSON.parse(#"""
            {"ambiguous":true,"vehicles":[
              {"id":1,"name":"Quadrotor 1","active":true,"contactLost":true},
              {"id":2,"name":"Quadrotor 2","active":false,"contactLost":false}]}
            """#)
        XCTAssertTrue(lostVehicles(vehicleChoices(activeLost)).isEmpty)
    }

    func testMoreThanOneSilentVehicleIsCountedRatherThanListed() {
        let many = JSON.parse(#"""
            {"ambiguous":true,"vehicles":[
              {"id":1,"name":"Quadrotor 1","active":true,"contactLost":false},
              {"id":2,"name":"Quadrotor 2","active":false,"contactLost":true},
              {"id":3,"name":"Quadrotor 3","active":false,"contactLost":true}]}
            """#)
        XCTAssertEqual(lostVehiclesText(lostVehicles(vehicleChoices(many))), "2 other vehicles are not answering")
    }

    func testNothingIsSaidWhenEveryVehicleIsAnswering() {
        XCTAssertNil(lostVehiclesText(lostVehicles(vehicleChoices(one))))
    }

    private func gate(_ heading: String) -> UploadGate {
        UploadGate(canSend: true, canProceed: true, pausesFirst: false, heading: heading, refusal: "", proceedTitle: "Upload")
    }

    func testTheUploadNamesTheAircraftItIsAboutToSendTo() {
        XCTAssertEqual(uploadHeading(gate(""), vehicleChoices(two)), "Upload this plan to Quadrotor 1?")
        XCTAssertEqual(uploadHeading(gate("Send the plan to the vehicle"), vehicleChoices(two)), "Send the plan to the vehicle to Quadrotor 1")
    }

    func testOneVehicleLeavesTheCoresQuestionExactlyAsItWas() {
        XCTAssertEqual(uploadHeading(gate(""), vehicleChoices(one)), "Upload this plan?")
        XCTAssertEqual(uploadHeading(gate("Replace the plan on the vehicle?"), vehicleChoices(one)), "Replace the plan on the vehicle?")
    }

    private func listOfVehicles(_ ids: Int..., activeId: Int) -> VehicleChoices {
        let listed = ids.map { #"{"id":\#($0),"name":"Quadrotor \#($0)","active":\#($0 == activeId)}"# }.joined(separator: ",")
        return vehicleChoices(JSON.parse(#"{"ambiguous":\#(ids.count > 1),"vehicles":[\#(listed)]}"#))
    }

    func testTheAircraftIsHandedOverBeforeTheOneThatLeftIsRemoved() {
        XCTAssertEqual(handoverNotice(listOfVehicles(1, 2, activeId: 1), listOfVehicles(1, 2, activeId: 2), nil), "Quadrotor 1 stopped answering. Now flying Quadrotor 2.")
        XCTAssertEqual(handoverNotice(listOfVehicles(1, 2, activeId: 1), listOfVehicles(2, activeId: 2), nil), "Quadrotor 1 is gone. Now flying Quadrotor 2.")
    }

    func testAnOperatorSwitchingVehiclesIsToldNothingHavingJustDoneIt() {
        XCTAssertNil(handoverNotice(listOfVehicles(1, 2, activeId: 1), listOfVehicles(1, 2, activeId: 2), 2))
    }

    func testARequestLastsExactlyOneChangeOfVehicle() {
        XCTAssertTrue(activeChanged(listOfVehicles(1, 2, activeId: 1), listOfVehicles(1, 2, activeId: 2)))
        XCTAssertFalse(activeChanged(listOfVehicles(1, 2, activeId: 1), listOfVehicles(1, 2, activeId: 1)))
        XCTAssertFalse(activeChanged(listOfVehicles(1, 2, activeId: 1), vehicleChoices(JSON.parse(#"{"vehicles":[{"id":2,"name":"Quadrotor 2","active":false}]}"#))))
        XCTAssertFalse(activeChanged(nil, listOfVehicles(1, activeId: 1)))
    }

    func testAHandoverBackToAVehicleTheOperatorOnceChoseIsStillAnnounced() {
        XCTAssertEqual(handoverNotice(listOfVehicles(1, 2, activeId: 1), listOfVehicles(1, 2, activeId: 2), nil), "Quadrotor 1 stopped answering. Now flying Quadrotor 2.")
    }

    func testAnOldRequestDoesNotSilenceTheNextHandover() {
        XCTAssertEqual(handoverNotice(listOfVehicles(1, 2, activeId: 2), listOfVehicles(1, 2, activeId: 1), 2), "Quadrotor 2 stopped answering. Now flying Quadrotor 1.")
    }

    func testTheOtherVehicleLeavingIsNotAHandover() {
        XCTAssertNil(handoverNotice(listOfVehicles(1, 2, activeId: 1), listOfVehicles(1, activeId: 1), nil))
    }

    func testTheLastVehicleLeavingSaysNothingBecauseNoVehicleAlreadyDoes() {
        XCTAssertNil(handoverNotice(listOfVehicles(1, activeId: 1), vehicleChoices(nil), nil))
    }

    func testTheFirstReadingAnnouncesNothing() {
        XCTAssertNil(handoverNotice(nil, listOfVehicles(1, 2, activeId: 1), nil))
    }

    func testTheGapBetweenOneVehicleLeavingAndTheNextBeingPromotedIsNotAReading() {
        let flying = listOfVehicles(1, 2, activeId: 1)
        let gap = vehicleChoices(JSON.parse(#"{"ambiguous":false,"vehicles":[{"id":2,"name":"Quadrotor 2","active":false}]}"#))
        let promoted = listOfVehicles(2, activeId: 2)
        XCTAssertEqual(rememberedChoices(flying, gap), flying)
        XCTAssertEqual(rememberedChoices(flying, promoted), promoted)
        XCTAssertEqual(handoverNotice(rememberedChoices(flying, gap), promoted, nil), "Quadrotor 1 is gone. Now flying Quadrotor 2.")
    }

    func testLosingEveryVehicleForgetsTheOneThatWasBeingFlown() {
        XCTAssertNil(rememberedChoices(listOfVehicles(1, activeId: 1), vehicleChoices(nil)))
    }

    func testTheChooserIsTitledAsTheQuestionItAnswersNotAsAState() {
        XCTAssertEqual(CHOOSER_TITLE, "Fly which aircraft?")
    }
}
