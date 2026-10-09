import XCTest
@testable import Aircast

final class WaypointActionsTests: XCTestCase {
    private let labels = ["No change", "Take photo", "Take photos (time)", "Take photos (distance)", "Stop taking photos", "Start recording video", "Stop recording video"]
    private let idleCamera = CameraExtras(intervalTime: nil, intervalDistance: nil, distanceUnits: "m", modeSupported: true, commandsMode: false, mode: 0, commandsGimbal: false, pitch: 0.0, yaw: 0.0)
    private let noHold = WaypointHold(seconds: 0.0, units: "s", path: "h")
    private let noTurn = WaypointYaw(degrees: nil, units: "deg", path: "y")

    func testAPlainWaypointHasNoActionsAndOffersEveryOneItCanCarry() {
        XCTAssertEqual([], activeActions(noHold, noTurn, CameraChoices(labels: labels, chosen: 0), idleCamera))
        XCTAssertEqual(
            ["Hover", "Photo", "Video", "Tilt camera", "Camera mode", "Turn aircraft"],
            offeredActions(noHold, noTurn, CameraChoices(labels: labels, chosen: 0), idleCamera).map(\.title)
        )
    }

    func testEachActionSetOnTheWaypointShowsOnceWithItsValueAndLeavesTheOffers() {
        let active = activeActions(
            WaypointHold(seconds: 5.0, units: "s", path: "h"),
            WaypointYaw(degrees: 90.0, units: "deg", path: "y"),
            CameraChoices(labels: labels, chosen: 2),
            withChanges(idleCamera) {
                $0.intervalTime = 10.0
                $0.commandsGimbal = true
                $0.pitch = -45.0
                $0.commandsMode = true
                $0.mode = 1
            }
        )
        XCTAssertEqual(
            ["Hover 5 s", "Take photos (time) every 10 s", "Gimbal -45° / 0°", "Camera mode Video", "Turn aircraft 90°"],
            active.map { "\($0.title) \($0.value)" }
        )
        XCTAssertEqual([], offeredActions(
            WaypointHold(seconds: 5.0, units: "s", path: "h"),
            WaypointYaw(degrees: 90.0, units: "deg", path: "y"),
            CameraChoices(labels: labels, chosen: 2),
            withChanges(idleCamera) {
                $0.commandsGimbal = true
                $0.commandsMode = true
            }
        ))
    }

    func testPhotoAndVideoStartFromTheirFirstActionAndTheEditorOffersTheRest() {
        XCTAssertEqual([1, 5], [startingChoice(labels, "photo"), startingChoice(labels, "video")])
        XCTAssertNil(startingChoice(["No change"], "photo"))
    }

    func testVideoActionsReadAsVideoAndStoppingPhotosReadsAsStop() {
        XCTAssertEqual(
            [Icon.planPhoto, .planVideo, .planVideoOff, .planStop],
            ["Take photo", "Start recording video", "Stop recording video", "Stop taking photos"].map(cameraIcon)
        )
    }

    func testAnUnsetHeadingReadsAsNoTurnAndAnItemWithoutOneOffersNone() {
        XCTAssertEqual(WaypointYaw(degrees: nil, units: "deg", path: "p"), waypointYaw(JSON.parse(#"{"yaw":{"value":null,"units":"deg","path":"p"}}"#)))
        XCTAssertEqual(WaypointYaw(degrees: 45.0, units: "deg", path: "p"), waypointYaw(JSON.parse(#"{"yaw":{"value":45.0,"units":"deg","path":"p"}}"#)))
        XCTAssertNil(waypointYaw(JSON.parse(#"{"yaw":null}"#)))
    }

    func testEachLegIsLabelledAtItsMiddleWithTheDistanceTheCoreMeasured() {
        let home = MissionItem(index: 0, sequence: 0, latitude: 41.0, longitude: 44.0, command: "Home", selected: true, altitude: .nan)
        let first = MissionItem(index: 1, sequence: 1, latitude: 41.0, longitude: 44.002, command: "Waypoint", selected: true, altitude: 50.0, distance: 168.0, distanceText: "168 m")
        let onTop = MissionItem(index: 2, sequence: 2, latitude: 41.0, longitude: 44.002, command: "Waypoint", selected: true, altitude: 50.0, distance: 0.0, distanceText: "0 m")
        let legs = legLabels([home, first, onTop])
        XCTAssertEqual(["168 m"], legs.map(\.1))
        XCTAssertEqual(1, legs.count)
        XCTAssertEqual(44.001, legs[0].0.longitude, accuracy: 1e-9)
        XCTAssertEqual(41.0, legs[0].0.latitude, accuracy: 1e-9)
    }
}
