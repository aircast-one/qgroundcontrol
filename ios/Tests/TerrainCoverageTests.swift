import XCTest
@testable import Aircast

final class TerrainCoverageTests: XCTestCase {
    private func at(_ distance: Double, _ terrain: Double?) -> ProfilePoint { ProfilePoint(distance: distance, terrain: terrain, planned: 100.0) }

    func testTerrainUnderTheWholeRouteIsFullCoverage() {
        XCTAssertEqual(1.0, TerrainProfile(points: [at(0.0, 400.0), at(500.0, 410.0), at(1000.0, 420.0)]).terrainCoverage, accuracy: 1e-9)
    }

    func testALegWithNoGroundUnderItDoesNotCount() {
        XCTAssertEqual(0.5, TerrainProfile(points: [at(0.0, 400.0), at(500.0, 410.0), at(1000.0, nil)]).terrainCoverage, accuracy: 1e-9)
    }

    func testGroundAtOneEndOnlyIsMeasuredByDistanceNotBySampleCount() {
        let dense = (0...9).map { at(Double($0) * 10.0, 400.0) }
        let bare = [at(1000.0, nil), at(2000.0, nil)]
        XCTAssertEqual(0.045, TerrainProfile(points: dense + bare).terrainCoverage, accuracy: 1e-9)
    }

    func testNoTerrainAnywhereIsNoCoverage() {
        XCTAssertEqual(0.0, TerrainProfile(points: [at(0.0, nil), at(1000.0, nil)]).terrainCoverage, accuracy: 1e-9)
    }

    func testARouteOfNoLengthHasNoCoverageRatherThanDividingByZero() {
        XCTAssertEqual(0.0, TerrainProfile(points: []).terrainCoverage, accuracy: 1e-9)
    }
}
