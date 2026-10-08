import XCTest
@testable import Aircast

final class MidpointTests: XCTestCase {
    private let ring = EditableShape(
        path: "plan.geoFenceController.polygons.0",
        midpoints: [
            TrackPoint(latitude: 41.700, longitude: 44.825), TrackPoint(latitude: 41.705, longitude: 44.830),
            TrackPoint(latitude: 41.710, longitude: 44.825), TrackPoint(latitude: 41.705, longitude: 44.820),
        ],
        splitInvokable: "splitPolygonSegment",
        canRemoveVertex: true
    )

    private lazy var square = FencePolygon(
        index: 0,
        inclusion: true,
        vertices: [
            TrackPoint(latitude: 41.70, longitude: 44.82), TrackPoint(latitude: 41.70, longitude: 44.83),
            TrackPoint(latitude: 41.71, longitude: 44.83), TrackPoint(latitude: 41.71, longitude: 44.82),
        ],
        editable: ring
    )

    private let corridor = EditableShape(
        path: "plan.missionController.visualItems.2.corridorPolyline",
        midpoints: [TrackPoint(latitude: 41.700, longitude: 44.825), TrackPoint(latitude: 41.705, longitude: 44.830)],
        splitInvokable: "splitSegment",
        canRemoveVertex: true
    )

    func testEverySegmentOffersSomewhereToAddACorner() {
        let features = midpointFeatures([ring]).shapes
        XCTAssertEqual(features.count, 4)
        XCTAssertEqual(features.map { $0.getNumberProperty(VERTEX_INDEX_PROPERTY).map { Int($0) } }, [0, 1, 2, 3])
    }

    func testASurveyEdgeIsSplitByItsOwnInvokableNotTheFenceOne() {
        let features = midpointFeatures([ring, corridor]).shapes
        XCTAssertEqual(features.count, 6)
        XCTAssertEqual(
            features.map { $0.getStringProperty(SPLIT_INVOKABLE_PROPERTY) },
            ["splitPolygonSegment", "splitPolygonSegment", "splitPolygonSegment", "splitPolygonSegment", "splitSegment", "splitSegment"]
        )
        let paths = features.compactMap { $0.getStringProperty(SHAPE_PATH_PROPERTY) }
        XCTAssertEqual(paths.enumerated().filter { paths.firstIndex(of: $0.element) == $0.offset }.map(\.element), [ring.path, corridor.path])
    }

    func testAShapeTheCoreGaveNoMidpointsForOffersNone() {
        XCTAssertEqual(midpointFeatures([withChanges(ring) { $0.midpoints = [] }]).shapes.count, 0)
        XCTAssertEqual(midpointFeatures([nil]).shapes.count, 0)
        XCTAssertEqual(midpointFeatures([]).shapes.count, 0)
    }

    func testAShapeTheCoreNamedNoInvokableForIsNotOffered() {
        XCTAssertEqual(midpointFeatures([withChanges(ring) { $0.splitInvokable = "" }]).shapes.count, 0)
    }

    func testAMidpointIsAButtonSoItNeverWritesAPosition() {
        XCTAssertFalse(writeMove(.Midpoint(path: ring.path, invokable: ring.splitInvokable, segment: 2), 41.0, 44.0, [], []))
    }

    func testAMidpointIsNeverALastingSelection() {
        XCTAssertFalse(selectionSurvives(.Midpoint(path: ring.path, invokable: ring.splitInvokable, segment: 2), [], [square], [], [], []))
    }

    func testTheLegIntoTheSelectedItemSplitsAtItsMiddle() throws {
        let item = { (index: Int, lat: Double, lon: Double) in
            MissionItem(index: index, sequence: index, latitude: lat, longitude: lon, command: "Waypoint", selected: false, altitude: 50.0)
        }
        let items = [item(1, 47.0, 8.0), item(2, 47.2, 8.4)]
        let split = try XCTUnwrap(legSplit(items, 2))
        XCTAssertEqual(metresBetween(TrackPoint(latitude: 47.0, longitude: 8.0), split), metresBetween(split, TrackPoint(latitude: 47.2, longitude: 8.4)), accuracy: 0.5)
        XCTAssertEqual(split.latitude, 47.1, accuracy: 0.01)
        XCTAssertEqual(azimuthBetween(TrackPoint(latitude: 0.0, longitude: 0.0), TrackPoint(latitude: 0.0, longitude: 1.0)), 90.0, accuracy: 1e-9)
        XCTAssertNil(legSplit(items, 1))
        XCTAssertNil(legSplit(items, nil))
    }

    func testTheSplitSkipsHomeAndStandaloneItemsAndStartsAtAPatternsExitAsMissionControllerDoes() throws {
        let item = { (index: Int, lat: Double, routed: Bool, exit: TrackPoint?) in
            MissionItem(index: index, sequence: index, latitude: lat, longitude: 8.0, command: "Waypoint", selected: false, altitude: 50.0, exit: exit, routed: routed)
        }
        let home = item(0, 46.0, true, nil)
        XCTAssertNil(legSplit([home, item(1, 47.0, true, nil)], 1), "the leg from home is not split")
        let roi = item(2, 47.5, false, nil)
        XCTAssertNil(legSplit([home, item(1, 47.0, true, nil), roi], 2), "a standalone item has no split of its own")
        let after = try XCTUnwrap(legSplit([home, item(1, 47.0, true, nil), roi, item(3, 48.0, true, nil)], 3))
        XCTAssertEqual(after.latitude, 47.5, accuracy: 0.01, "the ROI is skipped when looking back")
        let survey = item(1, 47.0, true, TrackPoint(latitude: 47.4, longitude: 8.0))
        let fromExit = try XCTUnwrap(legSplit([home, survey, item(2, 48.0, true, nil)], 2))
        XCTAssertEqual(fromExit.latitude, 47.7, accuracy: 0.01)
    }
}
