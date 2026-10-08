import XCTest
@testable import Aircast

final class CircleModeTests: XCTestCase {
    private let centre = TrackPoint(47.0, 8.0)
    private var ring: [TrackPoint] { circleRing(centre, 100.0, 36) }
    private var fence: FencePolygon { FencePolygon(index: 0, inclusion: true, vertices: ring) }

    func testACirclesRadiusIsTheDistanceFromItsCentreToAVertex() throws {
        XCTAssertEqual(100.0, try XCTUnwrap(circleRadius(ring)), accuracy: 0.5)
        XCTAssertNil(circleRadius([]))
    }

    func testSettingTheRadiusRedrawsTheRingAroundTheSameCentre() throws {
        let wider = try XCTUnwrap(circleAround(ring, 250.0))
        XCTAssertEqual(250.0, try XCTUnwrap(circleRadius(wider)), accuracy: 0.5)
        XCTAssertEqual(centre.latitude, try XCTUnwrap(polygonCentre(wider)).latitude, accuracy: 1e-6)
        XCTAssertNil(circleAround(ring, 0.0), "a zero radius is not a circle")
    }

    func testOnlyACircledShapeGetsARadiusHandleAndLosesItsCorners() {
        XCTAssertEqual(0, radiusHandleFeatures([fence], [], []).count)
        XCTAssertEqual(1, radiusHandleFeatures([fence], [], [fencePath(0)]).count)
    }

    func testATargetFindsItsOwnVertices() {
        XCTAssertEqual(ring, shapeVertices(ShapeTarget(path: fencePath(0), line: false), [fence], []))
        XCTAssertEqual([], shapeVertices(ShapeTarget(path: fencePath(3), line: false), [fence], []))
    }

    func testACirclesCentreIsEditedAsItsShapeCentreLikeEditCenterPosition() throws {
        let (hit, at) = try XCTUnwrap(shapeCentreHit(ShapeTarget(path: fencePath(0), line: false), [fence], []))
        XCTAssertEqual(MapHit.ShapeCentre(fence: true, owner: 0), hit)
        XCTAssertEqual(centre.latitude, at.latitude, accuracy: 1e-6)
        XCTAssertEqual(centre.longitude, at.longitude, accuracy: 1e-6)
        XCTAssertEqual("Edit center position", positionTitle(hit))
        XCTAssertEqual("Edit vertex position", positionTitle(.FenceVertex(polygon: 0, vertex: 1)))
        XCTAssertNil(shapeCentreHit(ShapeTarget(path: fencePath(3), line: false), [fence], []))
    }
}

final class DragStepTests: XCTestCase {
    func testADragStepDispatchedBeforeTheDropNeverLandsAfterIt() {
        let before = moveGeneration()
        let midpoint = MapHit.Midpoint(path: "", invokable: "", segment: 0)
        _ = writeMove(midpoint, 0.0, 0.0, [], [])
        XCTAssertEqual(true, writeDragStep(before, midpoint, 0.0, 0.0, [], [], [], []))
        XCTAssertEqual(false, writeDragStep(moveGeneration(), midpoint, 0.0, 0.0, [], [], [], []), "a current step is written, and a midpoint write reports false")
    }
}

final class LiveCircleTests: XCTestCase {
    func testCircleModeFollowsTheShapeAtThatPathNotJustItsIndex() throws {
        let ring = try XCTUnwrap(circleAround(circleRing(TrackPoint(47.0, 8.0), 100.0, 16), 100.0))
        let square = [TrackPoint(47.0, 8.0), TrackPoint(47.001, 8.0), TrackPoint(47.001, 8.001), TrackPoint(47.0, 8.001)]
        let chosen: Set<String> = [fencePath(0)]
        XCTAssertEqual(chosen, liveCircles(chosen, [FencePolygon(index: 0, inclusion: true, vertices: ring)], []))
        XCTAssertEqual([], liveCircles(chosen, [FencePolygon(index: 0, inclusion: true, vertices: square)], []), "fence 0 was deleted and a square slid into its place")
        XCTAssertEqual([], liveCircles(chosen, [], []))
    }
}
