import XCTest
@testable import Aircast

final class SyntheticViewTests: XCTestCase {
    func testTheSyntheticViewSaysWhichCameraItStandsInForOnceItHasDrawnThenStepsOffTheAimPointToTheEdge() {
        let standIn = "No video from SITL camera · showing synthetic view"
        XCTAssertEqual(shownNote(.Loading, standIn, announced: true), ShownNote(text: "Loading 3D view…", atEdge: false), "an empty pane has no aim point to cover")
        XCTAssertEqual(shownNote(.Failed, standIn, announced: true), ShownNote(text: "3D view failed to start", atEdge: false))
        XCTAssertEqual(shownNote(.Drawn, standIn, announced: false), ShownNote(text: standIn, atEdge: false), "the fallback is announced in the middle first")
        XCTAssertEqual(shownNote(.Drawn, standIn, announced: true), ShownNote(text: standIn, atEdge: true), "once announced it leaves the middle, where the camera points, clear")
        XCTAssertNil(shownNote(.Drawn, nil, announced: false), "the synthetic view chosen as the camera says nothing")
    }

    func testThe3dMapWaitsForAnAircraftUnlessItHoldsASpotThenSaysNothingOnceItHasDrawn() {
        let following = map3dState(nil)
        let holding = map3dState(JSON.parse(#"{"heading":0,"following":false,"aircraft":false}"#))
        XCTAssertEqual(map3dNote(nil, following), LOOKING_TITLE)
        XCTAssertEqual(map3dNote(JSON.parse(#"{"available":false,"native":true}"#), following), LOOKING_TITLE)
        XCTAssertEqual(map3dNote(JSON.parse(#"{"available":false,"draw":"loading"}"#), holding), "Loading 3D view…")
        XCTAssertEqual(map3dNote(JSON.parse(#"{"available":true,"draw":"loading"}"#), following), "Loading 3D view…")
        XCTAssertNil(map3dNote(JSON.parse(#"{"available":true,"draw":"drawn"}"#), following))
    }

    func testThe3dMapStateFollowsTheAircraftUntilTheCoreSaysOtherwise() {
        XCTAssertEqual(map3dState(nil), Map3dState(heading: 0, following: true, aircraft: false))
        XCTAssertEqual(map3dState(JSON.parse(#"{"heading":250,"following":false,"aircraft":true}"#)), Map3dState(heading: 250, following: false, aircraft: true))
    }

    func testAPressOnThe3dMapLandsOnTheGroundTheCoreFoundOrNowhere() {
        XCTAssertEqual(pickedPoint(JSON.parse(#"{"ok":true,"latitude":46.5,"longitude":7.9}"#)), MapPoint(latitude: 46.5, longitude: 7.9))
        XCTAssertNil(pickedPoint(JSON.parse(#"{"ok":false,"reason":"There is no ground under that point."}"#)))
        XCTAssertNil(pickedPoint(JSON.parse(#"{"ok":true,"latitude":46.5}"#)))
        XCTAssertNil(pickedPoint(nil))
    }

    func testTheMapToggleNamesTheMapItSwitchesTo() {
        XCTAssertEqual(map3dToggleText(false), "3D")
        XCTAssertEqual(map3dToggleLabel(false), "Show the 3D map")
        XCTAssertEqual(map3dToggleText(true), "2D")
        XCTAssertEqual(map3dToggleLabel(true), "Show the flat map")
    }

    func testThePageAsksForTilesBesideItselfAndTheCoreAnswersThem() {
        XCTAssertEqual(syntheticTilePath("/synthetic/tiles/Bing%20Satellite/15/29961/19829"), "/Bing%20Satellite/15/29961/19829")
        XCTAssertNil(syntheticTilePath("/synthetic/Cesium/Cesium.js"))
    }

    func testThePageAndCesiumComeFromTheirBundleFolders() {
        XCTAssertEqual(syntheticBundlePath("/synthetic/index.html"), "synthetic/index.html")
        XCTAssertEqual(syntheticBundlePath("/synthetic/Cesium/Workers/createVerticesFromHeightmap.js"), "Cesium/Workers/createVerticesFromHeightmap.js")
        XCTAssertNil(syntheticBundlePath("/elsewhere/file.js"))
    }

    func testDraggingAimsTheViewTheSameWayAsOnAndroid() {
        XCTAssertEqual(syntheticTilt(-15, -500, 1000), -60, accuracy: 1e-9)
        XCTAssertEqual(syntheticTilt(-15, 300, 1000), 12, accuracy: 1e-9)
        XCTAssertEqual(syntheticTilt(-15, 900, 1000), 30, accuracy: 1e-9)
        XCTAssertEqual(syntheticTilt(-15, -2000, 1000), -90, accuracy: 1e-9)
        XCTAssertEqual(syntheticPan(0, 250, 1000), 90, accuracy: 1e-9)
        XCTAssertEqual(syntheticPan(30, 1000, 1000), 30, accuracy: 1e-9)
        XCTAssertEqual(syntheticPan(170, -100, 1000), 134, accuracy: 1e-9)
    }

    func testTheViewIsDrawnOnlyWhenTheCoreCanPlaceTheCamera() {
        XCTAssertTrue(syntheticAvailable(JSON.parse(#"{"available":true}"#)))
        XCTAssertFalse(syntheticAvailable(JSON.parse(#"{"available":false}"#)))
        XCTAssertFalse(syntheticAvailable(nil))
        XCTAssertTrue(syntheticPoseScript(JSON.parse(#"{"heading":90}"#)).hasPrefix("window.aircast && window.aircast.pose("))
    }

    func testTheMapBeamIsThePolygonTheCorePlaces() {
        let view = JSON.parse(#"{"available":true,"beam":[{"latitude":-35.36,"longitude":149.16},{"latitude":-35.3597,"longitude":149.1604},{"latitude":-35.358,"longitude":149.162},{"latitude":-35.3585,"longitude":149.1628},{"latitude":-35.36,"longitude":149.16}]}"#)
        XCTAssertEqual(cameraBeam(view).count, 5)
        XCTAssertTrue(cameraBeam(JSON.parse(#"{"available":false,"beam":[]}"#)).isEmpty)
    }

    func testTheSyntheticSourceIsOfferedAndAddedUnderItsOwnName() {
        let synthetic = CameraKind(raw: SYNTHETIC_SOURCE, label: SYNTHETIC_SOURCE, group: "This device", needsUrl: false, hint: "")
        XCTAssertEqual(otherSourceLabel(synthetic), "Synthetic view")
        XCTAssertEqual(otherSourceName(synthetic), "Synthetic view")
    }

    func testSidewaysDragsAimOnlyASyntheticViewPickedAsTheCamera() {
        let aimable = JSON.parse(#"{"available":true,"aimable":true}"#)
        let cameras = { (source: String) in camerasReading(JSON.parse(#"{"class":"Cameras","cameras":[{"slot":0,"source":"\#(source)","active":true}]}"#)) }
        XCTAssertTrue(syntheticAims(aimable, cameras(SYNTHETIC_SOURCE)))
        XCTAssertFalse(syntheticAims(aimable, cameras("RTSP Video Stream")), "a dropped stream's synthetic fallback still lets a swipe switch cameras")
        XCTAssertFalse(syntheticAims(JSON.parse(#"{"available":true,"aimable":false,"locksOnTarget":true}"#), cameras(SYNTHETIC_SOURCE)), "a view locked on a tracked target takes no aim")
        XCTAssertTrue(syntheticAims(JSON.parse(#"{"available":true,"aimable":false,"gimbalAimed":true}"#), cameras(SYNTHETIC_SOURCE)), "picked a moment ago, it aims before the core sends a view that knows")
        XCTAssertFalse(syntheticAims(aimable, nil))
    }

    func testPickingTheSyntheticViewHandsTheDragToTheViewAtOnceBeforeTheCoreSendsAViewMadeAfterThePick() {
        let standingIn = JSON.parse(#"{"available":true,"aimable":false,"gimbalAimed":true,"locksOnTarget":false}"#)
        XCTAssertFalse(syntheticFree(standingIn, picked: false), "standing in for the gimbal's camera, the view follows the gimbal")
        XCTAssertTrue(syntheticFree(standingIn, picked: true), "the gimbal belongs to the camera the pilot left, so the very next drag aims the view")
        let pickedEarlier = JSON.parse(#"{"available":true,"aimable":true,"gimbalAimed":true,"locksOnTarget":false}"#)
        XCTAssertFalse(syntheticFree(pickedEarlier, picked: false), "and switching back to the gimbal camera takes the drag off the view just as fast")
        XCTAssertFalse(syntheticFree(JSON.parse(#"{"available":true,"aimable":false,"gimbalAimed":false,"locksOnTarget":true}"#), picked: true), "a target lock still holds the picked view")
        XCTAssertFalse(syntheticFree(JSON.parse(#"{"available":false}"#), picked: true))
    }

    func testWithAGimbalAimingTheViewAFingerDragSteersTheGimbalWithoutOneItLooksAround() {
        let gimballed = JSON.parse(#"{"available":true,"aimable":false,"gimbalAimed":true}"#)
        XCTAssertEqual(syntheticDrag(gimballed, picked: false, aimable: true), .Gimbal, "the view follows the gimbal, so a drag has to move the gimbal or it does nothing")
        XCTAssertEqual(syntheticDrag(JSON.parse(#"{"available":true,"aimable":true,"gimbalAimed":false}"#), picked: false, aimable: true), .Look, "free look keeps its own drag")
        XCTAssertNil(syntheticDrag(gimballed, picked: false, aimable: false), "a thumbnail or second pane never takes the drag")
        XCTAssertNil(syntheticDrag(JSON.parse(#"{"available":false,"aimable":false}"#), picked: false, aimable: true))
        XCTAssertEqual(syntheticDrag(JSON.parse(#"{"available":true,"aimable":false,"locksOnTarget":true}"#), picked: false, aimable: true), .Gimbal, "a view locked on a tracked target is not the pilot's to aim")
    }

    func testPickingTheSyntheticViewHandsTheDragFromTheGimbalToTheView() {
        let standingIn = JSON.parse(#"{"available":true,"aimable":false,"gimbalAimed":true,"locksOnTarget":false}"#)
        XCTAssertEqual(syntheticDrag(standingIn, picked: true, aimable: true), .Look, "the gimbal belongs to the camera the pilot left, so the very next drag aims the view")
        let pickedEarlier = JSON.parse(#"{"available":true,"aimable":true,"gimbalAimed":true,"locksOnTarget":false}"#)
        XCTAssertEqual(syntheticDrag(pickedEarlier, picked: false, aimable: true), .Gimbal, "and switching back to the gimbal camera hands the drag back to the gimbal just as fast")
        XCTAssertEqual(syntheticDrag(JSON.parse(#"{"available":true,"aimable":false,"gimbalAimed":false,"locksOnTarget":true}"#), picked: true, aimable: true), .Gimbal, "a target lock still holds the picked view")
    }

    func testADragStartsFromWhereTheActiveGimbalPoints() {
        let view = JSON.parse(#"{"gimbals":[{"active":false,"pitch":-80.0,"bodyYaw":90.0},{"active":true,"pitch":-12.5,"bodyYaw":30.0}]}"#)
        XCTAssertEqual(gimbalAim(view), GimbalPoint(tilt: -12.5, yaw: 30))
        XCTAssertNil(gimbalAim(JSON.parse(#"{"gimbals":[{"active":true,"pitch":null,"bodyYaw":null}]}"#)), "a gimbal that has not reported its attitude gives no start")
        XCTAssertNil(gimbalAim(nil))
    }

    func testASecondDragCarriesOnFromTheFirstInsteadOfSnappingBackToAStaleReport() {
        let reported = GimbalPoint(tilt: -2, yaw: 0)
        let sent = GimbalPoint(tilt: -20, yaw: 75)
        XCTAssertEqual(gimbalDragStart(reported, sent, sentAgoMs: 300), sent, "the gimbal is still on its way to the last aim, so that is where the finger left it")
        XCTAssertEqual(gimbalDragStart(reported, sent, sentAgoMs: 5000), reported, "long after, the gimbal's own report wins, in case something else moved it")
        XCTAssertEqual(gimbalDragStart(nil, nil, sentAgoMs: 0), GimbalPoint(tilt: 0, yaw: 0))
    }
}
