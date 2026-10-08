import XCTest
@testable import Aircast

final class ObstacleOverlayTests: XCTestCase {
    private func overlay(_ ranges: [Double], increment: Double = 90.0, offset: Double = 0.0, max: Double = 40.0) -> ObstacleOverlay {
        ObstacleOverlay(ranges: ranges, texts: ranges.map { String(format: "%.2f", $0) }, increment: increment, offset: offset, maxMetres: max)
    }

    private func assertPoint(_ actual: CGPoint, _ x: CGFloat, _ y: CGFloat, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(actual.x, x, accuracy: 1e-3, file: file, line: line)
        XCTAssertEqual(actual.y, y, accuracy: 1e-3, file: file, line: line)
    }

    func testTheOverlayReadsTheCoreDrawingAndNeedsAnIncrementAndARange() {
        let view = JSON.parse(#"{"ringIncrement":5.0,"ringOffset":0.0,"overlay":{"ranges":[3.2,655.35],"texts":["3.20","655.35"],"maxMetres":40.0}}"#)
        XCTAssertEqual(obstacleOverlay(view), ObstacleOverlay(ranges: [3.2, 655.35], texts: ["3.20", "655.35"], increment: 5.0, offset: 0.0, maxMetres: 40.0))
        XCTAssertNil(obstacleOverlay(JSON.parse(#"{"ringIncrement":null,"overlay":{"ranges":[3.2],"texts":["3.20"],"maxMetres":40.0}}"#)))
        XCTAssertNil(obstacleOverlay(JSON.parse(#"{"ringIncrement":5.0,"overlay":null}"#)))
    }

    func testRangeIndexRoundsUpAndTurnsWithHeadingLikeRangeIdx() {
        let ring = overlay([1.0, 2.0, 3.0, 4.0])
        XCTAssertEqual(rangeIndex(0.0, ring, 0.0), 0)
        XCTAssertEqual(rangeIndex(10.0, ring, 0.0), 1)
        XCTAssertEqual(rangeIndex(0.0, ring, 90.0), 3)
        XCTAssertEqual(rangeIndex(90.0, overlay([1.0, 2.0, 3.0, 4.0], offset: 180.0), 0.0), 3)
    }

    func testMapPointsSitBetweenTheInnerAndOuterRadiusAndFlipToTrueScaleWhenZoomedIn() {
        let ring = overlay([40.0, 20.0, 0.0, 10.0])
        let shape = mapOverlayShape(ring, CGPoint(x: 500, y: 500), 1000, 100.0, 100, 0.0, 0.0)
        XCTAssertEqual(shape.gradientFrom, 90, accuracy: 1e-3)
        XCTAssertEqual(shape.gradientTo, 450, accuracy: 1e-3)
        assertPoint(shape.points[0].outer, 500, 50)
        assertPoint(shape.points[1].outer, 770, 500)
        assertPoint(shape.points[2].outer, 500, 590)
        let zoomed = mapOverlayShape(ring, CGPoint(x: 500, y: 500), 1000, 2.0, 100, 0.0, 0.0)
        XCTAssertEqual(zoomed.gradientFrom, 0, accuracy: 1e-3)
        XCTAssertEqual(zoomed.gradientTo, 2000, accuracy: 1e-3)
    }

    func testMapLabelsTakeTheNearerOfEachPairAndSkipRepeatsWithinTwoMetres() {
        let ring = overlay([10.0, 9.0, 30.0, 11.0, 12.0, 30.0, 25.0, 50.0, 50.0], increment: 40.0)
        let shape = mapOverlayShape(ring, .zero, 1000, 100.0, 100, 0.0, 0.0)
        XCTAssertEqual(mapOverlayLabels(ring, shape.points).map(\.range), [9.0, 25.0])
    }

    func testVideoSegmentsStopAtTheLevelThatReachesTheNearestRangeAndLabelOnlyIt() throws {
        let ring = overlay((0..<16).map { $0 == 0 ? 15.0 : 655.35 }, increment: 22.5)
        let (radii, segments) = try XCTUnwrap(videoOverlaySegments(ring, 1000, true))
        XCTAssertEqual(radii.0, 450 - 450 * 0.2 / 8 * 4 * 2, accuracy: 0.01)
        let first = segments.filter { $0.radFrom == 0.0 }
        XCTAssertEqual(first.count, 3)
        XCTAssertEqual(first.map(\.label), [nil, nil, "15.00"])
        XCTAssertEqual(segments.count, 15 + 3)
        XCTAssertNil(videoOverlaySegments(overlay([1.0], max: 250.0), 1000, true))
    }
}
