import XCTest
@testable import Aircast

final class FenceFeatureTests: XCTestCase {
    private func ring(_ index: Int, inclusion: Bool = true) -> FencePolygon {
        FencePolygon(
            index: index,
            inclusion: inclusion,
            vertices: [TrackPoint(latitude: 41.0, longitude: 44.0), TrackPoint(latitude: 41.0, longitude: 44.1), TrackPoint(latitude: 41.1, longitude: 44.1)]
        )
    }

    func testACircleCarriesTheIndexThatFindsItAgain() {
        let features = fenceFeatures([], circles: [ring(4)]).shapes
        XCTAssertEqual(features.count, 1)
        XCTAssertEqual(features.first?.getNumberProperty(CIRCLE_INDEX_PROPERTY).map { Int($0) }, 4)
    }

    func testAPolygonCarriesNoCircleIndexSoItCannotBeMistakenForOne() {
        let features = fenceFeatures([ring(0)], circles: []).shapes
        XCTAssertEqual(features.count, 1)
        XCTAssertNil(features.first?.getNumberProperty(CIRCLE_INDEX_PROPERTY))
    }

    func testPolygonsAndCirclesKeepSeparateIndexSpacesInOneCollection() {
        let features = fenceFeatures([ring(0)], circles: [ring(0)]).shapes
        let tagged = features.filter { $0.getNumberProperty(CIRCLE_INDEX_PROPERTY) != nil }
        XCTAssertEqual(features.count, 2)
        XCTAssertEqual(tagged.count, 1)
    }

    func testAKeepOutFenceIsDrawnInADifferentColourFromAKeepInOne() {
        let features = fenceFeatures([ring(0, inclusion: true), ring(1, inclusion: false)], circles: []).shapes
        XCTAssertEqual(features.map { $0.getBooleanProperty(KEEPS_IN_PROPERTY) }, [true, false])
        XCTAssertNotEqual(KEEP_IN_COLOUR, KEEP_OUT_COLOUR)
    }

    func testAKeepOutCircleCarriesItsBoundaryTooNotJustPolygons() {
        XCTAssertEqual(fenceFeatures([], circles: [ring(2, inclusion: false)]).shapes.first?.getBooleanProperty(KEEPS_IN_PROPERTY), false)
    }
}
