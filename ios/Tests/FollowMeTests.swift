import XCTest
@testable import Aircast

final class FollowMeTests: XCTestCase {
    private func view(
        mode: String = #""followMe""#,
        enabled: Bool = true,
        wouldSend: Bool = true,
        reason: String = "null",
        vehicles: String = #"[{"id":1,"following":true,"refusal":""}]"#
    ) -> JSON {
        JSON.parse(#"{"kind":"object","class":"FollowMe","mode":\#(mode),"enabled":\#(enabled),"wouldSend":\#(wouldSend),"reason":\#(reason),"vehicles":\#(vehicles),"count":1}"#)
    }

    func testAnotherViewIsNotAFollowReading() {
        XCTAssertNil(followMeReading(nil))
        XCTAssertNil(followMeReading(JSON.parse(#"{"kind":"object","class":"Orbit"}"#)))
    }

    func testTheDefaultSettingDoesNotAnnounceThatAFeatureNobodyAskedForIsNotHappening() {
        XCTAssertNil(followMeLabel(followMeReading(view(mode: #""followMe""#, wouldSend: false, reason: #""noVehicleInFollowMode""#))))
    }

    func testTheCoreCannotReportThatReasonUnderAlwaysSoTheModeDoesNotNeedTesting() {
        XCTAssertNil(followMeLabel(followMeReading(view(mode: #""always""#, wouldSend: false, reason: #""noVehicleInFollowMode""#))))
    }

    func testOnceAVehicleIsInFollowMeModeEveryOtherTroubleIsStillReported() {
        XCTAssertEqual(
            followMeLabel(followMeReading(view(mode: #""followMe""#, wouldSend: false, reason: #""noFix""#))),
            "Not following you \u{2014} this phone has no position yet"
        )
    }

    func testAVehicleTakingTheStreamSaysSoPlainly() {
        XCTAssertEqual(followMeLabel(followMeReading(view())), "Following you")
    }

    func testASwitchedOffFeatureShowsNothingAtAllRatherThanAComplaint() {
        let off = view(mode: #""never""#, enabled: false, wouldSend: false, reason: #""modeNever""#)
        XCTAssertNil(followMeLabel(followMeReading(off)))
    }

    func testSwitchedOnAndNotSendingNamesTheThingStandingInTheWay() {
        func stuck(_ token: String) -> String? {
            followMeLabel(followMeReading(view(mode: #""always""#, enabled: false, wouldSend: false, reason: #""\#(token)""#)))
        }
        XCTAssertEqual(stuck("fixStale"), "Not following you — this phone's position has stopped updating")
        XCTAssertEqual(stuck("fixInvalid"), "Not following you — this phone's position is not valid")
        XCTAssertEqual(stuck("allVehiclesRefused"), "Not following you — the vehicle refused the position")
    }

    func testATokenThisHeadHasNeverSeenStillProducesASentence() {
        XCTAssertEqual(
            followMeLabel(followMeReading(view(enabled: false, wouldSend: false, reason: #""somethingNew""#))),
            "Not following you — the position is not being sent"
        )
    }

    func testMoreThanOneFollowerIsWorthCountingOneIsNot() {
        let pair = view(vehicles: #"[{"id":1,"following":true},{"id":2,"following":true}]"#)
        XCTAssertEqual(followMeLabel(followMeReading(pair)), "Following you · 2 vehicles")
    }

    func testEnabledIsTheTimerRunningNotTheOperatorAskingSoTheChipCannotKeyOnIt() {
        let asked = view(mode: #""always""#, enabled: false, wouldSend: false, reason: #""fixStale""#, vehicles: #"[{"id":1,"following":false,"refusal":"notInFollowMode"}]"#)
        XCTAssertEqual(followMeLabel(followMeReading(asked)), "Not following you — this phone's position has stopped updating")
    }

    func testNoVehicleAtAllIsNothingToSayHoweverTheSettingReads() {
        let alone = view(enabled: false, wouldSend: false, reason: #""noVehicles""#, vehicles: "[]")
        XCTAssertNil(followMeLabel(followMeReading(alone)))
    }
}
