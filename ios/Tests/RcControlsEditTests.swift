import XCTest
@testable import Aircast

final class RcControlsEditTests: XCTestCase {
    private let served = #"""
        [{"label":"Winch","channel":7,"type":"slider","orientation":"vertical"},
         {"label":"Light","channel":9,"type":"button"}]
    """#

    func testEditingAControlKeepsTheFieldsThisHeadDoesNotUnderstand() {
        let next = rcControlsPatched(served, 0, "Hoist", 8, .Button)
        let entry = JSON.parse(next)[0]

        XCTAssertEqual(entry["label"].string, "Hoist")
        XCTAssertEqual(entry["channel"].int, 8)
        XCTAssertEqual(entry["type"].string, "button")
        XCTAssertEqual(entry["orientation"].string, "vertical")
    }

    func testAddingAndRemovingKeepTheRestOfTheListIntact() {
        let added = rcControlsAdded(served, "Zoom", 11, .Switch3)
        XCTAssertEqual(parseRcControls(added).map(\.label), ["Winch", "Light", "Zoom"])
        XCTAssertEqual(parseRcControls(added)[2].type, .Switch3)

        let removed = rcControlsRemoved(added, 1)
        XCTAssertEqual(parseRcControls(removed).map(\.label), ["Winch", "Zoom"])
        XCTAssertEqual(JSON.parse(removed)[0]["orientation"].string, "vertical")
    }

    func testAChannelAlreadyDrivingSomethingNamesWhatHasIt() {
        let reserved = [5: "Gimbal tilt"]

        XCTAssertEqual(channelOwner(served, 5, -1, reserved), "Gimbal tilt")
        XCTAssertEqual(channelOwner(served, 9, 0, reserved), "Light")
        XCTAssertNil(channelOwner(served, 9, 1, reserved))
        XCTAssertNil(channelOwner(served, 12, -1, reserved))
    }

    func testAnUnnamedControlStillIdentifiesItselfInAClash() {
        let unnamed = #"[{"label":"","channel":4,"type":"button"}]"#

        XCTAssertEqual(channelOwner(unnamed, 4, -1, [:]), "control 1")
    }

    func testANewControlLandsOnTheFirstChannelNothingElseIsUsing() {
        XCTAssertEqual(firstFreeChannel(served, [:]), 1)
        XCTAssertEqual(firstFreeChannel(served, [1: "Gimbal tilt"]), 2)
        XCTAssertEqual(
            firstFreeChannel(
                #"[{"channel":1,"type":"button"},{"channel":2,"type":"button"}]"#,
                [3: "a", 4: "b", 5: "c", 6: "d", 7: "e"]
            ),
            8
        )
    }

    func testChannelsOutsideTheRadiosRangeAreRefused() {
        XCTAssertEqual(channelUsable(0), false)
        XCTAssertEqual(channelUsable(19), false)
        XCTAssertEqual(channelUsable(1), true)
        XCTAssertEqual(channelUsable(18), true)
    }

    func testATypeRoundTripsThroughItsStoredName() {
        RcControlType.allCases.forEach { type in
            let json = rcControlsAdded(nil, "x", 3, type)
            XCTAssertEqual(parseRcControls(json).map(\.type), [type])
        }
    }
}
