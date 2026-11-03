import XCTest
@testable import Aircast

final class PictureHoldTests: XCTestCase {
    private let spot = CGPoint(x: 0.25, y: 0.75)

    func testASpotOnTheGroundOpensTheVideoMenuWithTheCoresActionsAndItsFieldOfViewWarning() {
        let warning = "The camera's field of view isn't set, so this spot may be off. Set it in Settings > Gimbal."
        let press = picturePress(
            spot,
            JSON.parse(#"{"ok":true,"latitude":47.4,"longitude":8.5,"actions":["GoTo","Orbit","Roi","StopLooking"],"warning":"\#(warning)"}"#),
            hasGimbal: true
        )
        XCTAssertEqual(press.point, MapPoint(latitude: 47.4, longitude: 8.5))
        XCTAssertEqual(press.actions, ["GoTo", "Orbit", "Roi", "StopLooking"])
        XCTAssertEqual(press.warning, warning)
        XCTAssertEqual(press.spot, spot)
        XCTAssertNil(picturePress(spot, JSON.parse(#"{"ok":true,"latitude":47.4,"longitude":8.5,"actions":[],"warning":null}"#), hasGimbal: true).warning)
    }

    func testASpotThatCannotBePlacedSaysWhyInTheWordsQgcUses() {
        let sky = picturePress(spot, JSON.parse(#"{"ok":false,"refusal":"notGround","reason":"That is above the horizon, or farther than 2 km."}"#), hasGimbal: true)
        XCTAssertNil(sky.point)
        XCTAssertEqual(sky.reason, BELOW_HORIZON)
        XCTAssertEqual(picturePress(spot, nil, hasGimbal: false).reason, NO_GIMBAL)
        XCTAssertNil(picturePress(spot, JSON.parse(#"{"ok":true,"latitude":47.4,"longitude":8.5}"#), hasGimbal: false).point)
        let unplaced = "Where the aircraft is and where its camera looks are not known yet."
        XCTAssertEqual(picturePress(spot, JSON.parse(#"{"ok":false,"refusal":"notPlaced","reason":"\#(unplaced)"}"#), hasGimbal: true).reason, unplaced)
    }

    func testTheReticleSaysOrbitingOverLookingAndStopPausesAnOrbitButCancelsALookAt() {
        XCTAssertEqual(pointing(looking: true, orbiting: true), .Orbiting)
        XCTAssertEqual(Pointing.Orbiting.stopPath, PAUSE_VEHICLE_PATH)
        XCTAssertEqual(pointing(looking: true, orbiting: false), .Looking)
        XCTAssertEqual(Pointing.Looking.stopPath, STOP_ROI_PATH)
        XCTAssertNil(pointing(looking: false, orbiting: false))
        XCTAssertEqual(Pointing.Looking.text, "Looking at this point")
        XCTAssertEqual(Pointing.Orbiting.text, "Orbiting this point")
    }

    func testAPressOnTheVideoClosesAStartOrContinueMissionPromptButNeverTheLandAbort() {
        XCTAssertTrue(dismissesMissionPrompt("startMission"))
        XCTAssertTrue(dismissesMissionPrompt("continueMission"))
        XCTAssertFalse(dismissesMissionPrompt("landAbort"))
        XCTAssertFalse(dismissesMissionPrompt("rtl"))
        XCTAssertFalse(dismissesMissionPrompt(nil))
    }

    func testAnOrbitOutlineIsDrawnOnlyBetweenNeighboursBothInFrontOfTheCamera() {
        let spots = outlineSpots(JSON.parse(#"{"outline":[{"x":0.1,"up":0.0},{"x":0.2,"up":0.0},null,{"x":0.4,"up":0.0},{"x":0.5,"up":0.0}]}"#))
        XCTAssertEqual(spots.count, 5)
        XCTAssertNil(spots[2])
        XCTAssertEqual(
            outlineLegs(spots),
            [ArLeg(from: ArSpot(x: 0.1, up: 0), to: ArSpot(x: 0.2, up: 0)), ArLeg(from: ArSpot(x: 0.4, up: 0), to: ArSpot(x: 0.5, up: 0))]
        )
        XCTAssertEqual(outlineSpots(nil), [])
    }

    func testTheRunningOrbitsOutlineComesWithTheVideoOverlayAndNoneWhenNotOrbiting() {
        XCTAssertEqual(orbitOutline(JSON.parse(#"{"available":true,"orbit":{"radiusMetres":30.0,"outline":[{"x":0.5,"up":-0.1},null]}}"#)), [ArSpot(x: 0.5, up: -0.1), nil])
        XCTAssertEqual(orbitOutline(JSON.parse(#"{"available":true,"orbit":null}"#)), [])
    }
}
