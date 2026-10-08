import MapLibre
import XCTest
@testable import Aircast

final class Viewer3DTests: XCTestCase {
    func testTheCoresBuildingsBecomeExtrudedPolygonsCentredOnTheFilesBounds() throws {
        let view = JSON.parse(#"{"class":"Viewer3D","available":true,"reason":null,"bounds":{"south":47.0,"west":8.0,"north":47.2,"east":8.2},"buildings":[{"outer":[[[8.01,47.01],[8.02,47.01],[8.02,47.02],[8.01,47.01]]],"inner":[],"height":6.0},{"outer":[[[8.0,47.0]]],"inner":[],"height":3.0}]}"#)
        let scene = scene3d(view)
        XCTAssertEqual(1, scene.buildings.count)
        let centre = try XCTUnwrap(scene.centre)
        XCTAssertEqual(8.1, centre.0, accuracy: 1e-9)
        XCTAssertEqual(47.1, centre.1, accuracy: 1e-9)
        let features = buildingFeatures(scene.buildings).shapes
        XCTAssertEqual(1, features.count)
        XCTAssertEqual(6.0, try XCTUnwrap(features.first?.attribute(forKey: "height") as? Double), accuracy: 0)
    }

    func testAnUnavailableViewCarriesTheReasonToShow() {
        XCTAssertEqual("Turn on the 3D view in Settings.", scene3d(JSON.parse(#"{"available":false,"reason":"Turn on the 3D view in Settings.","buildings":[]}"#)).reason)
    }

    func testOuterRingsWindCounterClockwiseAndHolesClockwiseSoMapLibreKeepsCourtyardsAsHoles() {
        let square = [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]
        XCTAssertTrue(signedArea(wound(square, true)) > 0)
        XCTAssertTrue(signedArea(wound(square, false)) < 0)
        let closed = wound(square, true)
        XCTAssertTrue(closed.first! == closed.last!)
    }

    func testAClimbingSegmentFloatsAsSteppedSlabsAtItsAltitude() {
        let pieces = ribbon(Point3D(lon: 8.0, lat: 47.0, alt: 20.0), Point3D(lon: 8.0, lat: 47.0003, alt: 40.0), "orange")
        XCTAssertEqual(5, pieces.count)
        XCTAssertTrue(pieces.first!.base < pieces.last!.base)
        XCTAssertEqual(22.0, (pieces.first!.base + pieces.first!.top) / 2, accuracy: 1e-9)
    }

    func testMarkersFloatAtTheirAltitudeAndTheVehicleComesFromItsOwnView() {
        XCTAssertEqual([28.5], pathSlabs(JSON.parse(#"{"markers":[{"at":[8.0,47.0,30.0],"name":"W","colour":"black"}],"segments":[]}"#)).map(\.base))
        let one = vehicleSlabs(JSON.parse(#"{"vehicles":[{"at":[8.0,47.0,12.0],"heading":0.0}]}"#))
        XCTAssertTrue(!one.isEmpty && one.allSatisfy { (11.0...13.0).contains($0.base) })
        XCTAssertEqual(2 * one.count, vehicleSlabs(JSON.parse(#"{"vehicles":[{"at":[8.0,47.0,12.0],"heading":0.0},{"at":[8.001,47.0,30.0],"heading":90.0}]}"#)).count)
        XCTAssertTrue(vehicleSlabs(JSON.parse(#"{"vehicles":[]}"#)).isEmpty)
    }

    func testTheQuadFramesRedFrontArmsPointAlongTheHeading() {
        let at = Point3D(lon: 8.0, lat: 47.0, alt: 20.0)
        func centre(_ slabs: [Slab], _ pick: ((Double, Double)) -> Double) -> Double {
            let values = slabs.flatMap(\.corners).map(pick)
            return values.reduce(0, +) / Double(values.count)
        }
        let frame = quadFrame(at, 90.0)
        XCTAssertTrue(centre(frame.filter { $0.colour == "#E53935" }, \.0) > at.lon)
        XCTAssertTrue(centre(frame.filter { $0.colour == "#ECEFF1" }, \.0) < at.lon)
        XCTAssertTrue(centre(quadFrame(at, 0.0).filter { $0.colour == "#E53935" }, \.1) > at.lat)
    }

    func testAStraightClimbIsOneColumnFromTheLowerToTheHigherAltitude() {
        let column = ribbon(Point3D(lon: 8.0, lat: 47.0, alt: 10.0), Point3D(lon: 8.0, lat: 47.0, alt: 40.0), "orange")
        XCTAssertEqual(1, column.count)
        XCTAssertEqual(9.5, column[0].base, accuracy: 1e-9)
        XCTAssertEqual(40.5, column[0].top, accuracy: 1e-9)
    }

    func testASteepClimbLeavesNoGapsAndALongLegStaysCapped() {
        let steep = ribbon(Point3D(lon: 8.0, lat: 47.0, alt: 0.0), Point3D(lon: 8.0, lat: 47.0003, alt: 20.0), "orange")
        XCTAssertTrue(zip(steep, steep.dropFirst()).allSatisfy { a, b in b.base <= a.top + 1e-9 })
        XCTAssertEqual(64, ribbon(Point3D(lon: 8.0, lat: 47.0, alt: 10.0), Point3D(lon: 8.0, lat: 48.0, alt: 10.0), "orange").count)
    }

    func testEveryMarkerCarriesItsLabelThreeMetresAboveItLikeViewer3DVehicleItems() {
        let labels = pathLabels(JSON.parse(#"{"markers":[{"at":[8.0,47.0,30.0],"name":"W","label":"3","colour":"black"},{"at":[8.1,47.1,0.0],"name":"","label":"","colour":"black"}]}"#))
        XCTAssertEqual([Label3D(at: Point3D(lon: 8.0, lat: 47.0, alt: 33.0), text: "3")], labels)
    }

    func testALabelRisesOnScreenByItsHeightForeshortenedByTheCameraTilt() {
        let flat = lifted((100, 200), 2.0, 0.0, 50.0)
        XCTAssertEqual([100, 200], [flat.0, flat.1])
        let tilted = lifted((100, 200), 2.0, 30.0, 50.0)
        XCTAssertEqual(100, tilted.0)
        XCTAssertEqual(150, tilted.1, accuracy: 1e-4)
        let below = lifted((100, 200), 2.0, 30.0, -5.0)
        XCTAssertEqual([100, 200], [below.0, below.1])
    }

    func testATappedMarkerTurnsYellowLikeViewer3DInstancingsHighlightAndAMissPicksNothing() {
        let view = JSON.parse(#"{"markers":[{"at":[8.0,47.0,30.0],"colour":"black"},{"at":[8.001,47.0,30.0],"colour":"green"}],"segments":[]}"#)
        XCTAssertEqual(["black", "#FFFF00"], pathSlabs(view, [1]).map(\.colour))
        XCTAssertEqual(2, pathMarkers(view).count)
        let onScreen: [(Float, Float)?] = [(10, 10), nil, (100, 100)]
        XCTAssertEqual(2, pickedMarker((95, 104), onScreen, 24))
        XCTAssertNil(pickedMarker((50, 50), onScreen, 24))
        XCTAssertEqual(0, pickedMarker((20, 10), onScreen, 24))
    }

    func testEachVehicleKeepsItsOwnSelectedMarkerLikeViewer3DModelsPerVehicleWaypointInstancing() {
        let a = Point3D(lon: 8.0, lat: 47.0, alt: 30.0)
        let b = Point3D(lon: 8.001, lat: 47.0, alt: 30.0)
        let c = Point3D(lon: 8.002, lat: 47.0, alt: 30.0)
        let markers = [PathMarker(mission: 0, at: a), PathMarker(mission: 0, at: b), PathMarker(mission: 1, at: c), PathMarker(mission: 1, at: a)]
        let first = selectedAfterTap([], markers, 1)
        let both = selectedAfterTap(first, markers, 3)
        XCTAssertEqual([1, 3], both)
        XCTAssertEqual([0, 3], selectedAfterTap(both, markers, 0))
        XCTAssertEqual([], selectedAfterTap(both, markers, nil))
        let firstMissionChanged = [PathMarker(mission: 0, at: a), PathMarker(mission: 1, at: c), PathMarker(mission: 1, at: a)]
        XCTAssertEqual([2], selectionKept(both, markers, firstMissionChanged))
        XCTAssertEqual([1, 3], selectionKept(both, markers, markers))
    }

    func testMarkersCarryTheMissionTheyBelongTo() {
        XCTAssertEqual([0, 2], pathMarkers(JSON.parse(#"{"markers":[{"at":[8.0,47.0,30.0],"mission":0},{"at":[8.0,47.0,30.0],"mission":2}]}"#)).map(\.mission))
    }
}
