import XCTest
@testable import Aircast

final class FlightModesTests: XCTestCase {
    private let served = #"""
        {"available":true,"canSet":true,"current":"Stabilize",
         "currentSummary":"You fly it; it keeps itself level.",
         "everyday":[
           {"name":"Stabilize","summary":"You fly it; it keeps itself level.","current":true,
            "advanced":false,"needsConfirm":false},
           {"name":"Loiter","summary":"Holds position.","current":false,
            "advanced":false,"needsConfirm":false}],
         "folded":[
           {"name":"Acro","summary":"Rate mode, no self-levelling.","current":false,
            "advanced":true,"needsConfirm":false}]}
        """#

    func testTheEverydayModesAreSeparateFromTheFoldedOnes() throws {
        let modes = try XCTUnwrap(flightModesView(JSON.parse(served)))
        XCTAssertEqual(modes.everyday.map(\.name), ["Stabilize", "Loiter"])
        XCTAssertEqual(modes.folded.map(\.name), ["Acro"])
    }

    func testTheCurrentModeIsMarkedSoTheListCanTickIt() throws {
        let modes = try XCTUnwrap(flightModesView(JSON.parse(served)))
        XCTAssertEqual(modes.current, "Stabilize")
        XCTAssertTrue(modes.everyday.first { $0.name == "Stabilize" }?.current == true)
    }

    func testTheConfirmFlagIsReadFromTheModeNotGuessedFromItsName() throws {
        let flying = try XCTUnwrap(flightModesView(JSON.parse(#"""
            {"available":true,"canSet":true,"current":"Loiter",
             "everyday":[
               {"name":"RTL","summary":"Flies home.","current":false,"advanced":false,"needsConfirm":true},
               {"name":"Loiter","summary":"Holds position.","current":true,"advanced":false,"needsConfirm":false}],
             "folded":[]}
            """#)))
        XCTAssertTrue(flying.everyday.first { $0.name == "RTL" }?.needsConfirm == true)
        XCTAssertFalse(flying.everyday.first { $0.name == "Loiter" }?.needsConfirm == true)
        XCTAssertFalse(try XCTUnwrap(flightModesView(JSON.parse(served))).folded.first { $0.name == "Acro" }?.needsConfirm == true)
    }

    func testAVehicleReportingNoModesOffersNoPicker() {
        XCTAssertNil(flightModesView(nil))
        XCTAssertNil(flightModesView(JSON.parse(#"{"available":false}"#)))
    }

    func testTheCoresUnknownModeLineIsCarriedToTheMenu() throws {
        let view = JSON.parse(#"{"available":true,"current":"Custom 7","unknownModeNotice":"The vehicle is in Custom 7, which this version of the app doesn't know. Choose a mode below to change it."}"#)
        XCTAssertTrue(try XCTUnwrap(flightModesView(view)).unknownModeNotice.hasPrefix("The vehicle is in Custom 7"))
        XCTAssertEqual(try XCTUnwrap(flightModesView(JSON.parse(#"{"available":true}"#))).unknownModeNotice, "")
    }

    private func modes(_ current: String, _ summary: String) -> FlightModesView? {
        flightModesView(JSON.parse(#"{"kind":"object","class":"FlightModes","available":true,"canSet":true,"current":\#(current),"currentSummary":\#(summary),"everyday":[],"folded":[]}"#))
    }

    func testTheHeadingNamesTheModeAndWhatItDoes() {
        XCTAssertEqual(modeHeading(modes("\"Stabilize\"", "\"You fly it by hand, it only levels itself\"")), "Stabilize — You fly it by hand, it only levels itself")
    }

    func testAModeTheCoreHasNoSentenceForGetsNoHeadingRatherThanADanglingDash() {
        XCTAssertNil(modeHeading(modes("\"Stabilize\"", "\"\"")))
        XCTAssertNil(modeHeading(modes("\"\"", "\"Something\"")))
        XCTAssertNil(modeHeading(nil))
    }

    func testHidingAModeAppendsItAndShowingItTakesItOutTheWayQgcWritesTheSetting() {
        XCTAssertEqual(hiddenModesAfter(["Manual", "Offboard"], "Acro", true), "Manual,Offboard,Acro")
        XCTAssertEqual(hiddenModesAfter(["Manual", "Offboard"], "Manual", false), "Offboard")
        XCTAssertEqual(hiddenModesAfter(["Manual"], "Manual", false), "")
        XCTAssertEqual(hiddenModesAfter(["Manual"], "Manual", true), "Manual")
    }

    func testEachModeInTheMenuCarriesAnIconLikeThePenpotModePicker() {
        XCTAssertEqual(
            ["Stabilize", "Altitude Hold", "Position Hold", "Auto", "Smart RTL", "Circle"].map(flightModeIcon),
            [.gamepad, .height, .myLocation, .route, .home, .flight]
        )
    }

    func testADividerStartsTheReturnModesLikeTheQgcModeSections() throws {
        let modes = try XCTUnwrap(flightModesView(JSON.parse(#"""
            {"available":true,"modes":[
              {"name":"Loiter","section":"normal"},{"name":"Guided"},
              {"name":"RTL","section":"return"},{"name":"Land","section":"return"}]}
            """#))).all
        XCTAssertEqual(modes.indices.map { startsSection(modes, $0) }, [false, false, true, false])
    }
}
