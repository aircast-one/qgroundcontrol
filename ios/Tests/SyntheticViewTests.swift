import XCTest
@testable import Aircast

final class SyntheticViewTests: XCTestCase {
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
        XCTAssertEqual(syntheticTilt(-15, 900, 1000), 0, accuracy: 1e-9)
        XCTAssertEqual(syntheticTilt(-15, -2000, 1000), -90, accuracy: 1e-9)
        XCTAssertEqual(syntheticPan(0, 500, 1000, 70), -35, accuracy: 1e-9)
        XCTAssertEqual(syntheticPan(170, -2000, 1000, 10), -170, accuracy: 1e-9)
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
}
