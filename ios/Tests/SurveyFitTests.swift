import XCTest
@testable import Aircast

final class SurveyFitTests: XCTestCase {
    private let view = [TrackPoint(42.0, 44.0), TrackPoint(42.0, 45.0), TrackPoint(41.0, 45.0), TrackPoint(41.0, 44.0)]

    private func survey(_ corners: Int) -> Survey {
        Survey(index: 2, area: Array(repeating: TrackPoint(41.5, 44.5), count: corners), transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: "surveyAreaPolygon")
    }

    func testTheFittedAreaSitsInsideWhatTheOperatorCanSeeNotFlushAgainstTheEdge() {
        let inset = insetRing(view, 0.8)
        XCTAssertEqual(4, inset.count)
        XCTAssertTrue(inset.allSatisfy { $0.latitude < 42.0 }, "north edge must come in from the top")
        XCTAssertTrue(inset.allSatisfy { $0.latitude > 41.0 }, "south edge must come up from the bottom")
        XCTAssertEqual(41.9, inset[0].latitude, accuracy: 1e-9)
        XCTAssertEqual(44.1, inset[0].longitude, accuracy: 1e-9)
    }

    func testTheInsetKeepsTheCentreWhereItWasSoFittingDoesNotWalkTheSurvey() {
        let inset = insetRing(view, 0.8)
        XCTAssertEqual(41.5, inset.map(\.latitude).reduce(0, +) / Double(inset.count), accuracy: 1e-9)
        XCTAssertEqual(44.5, inset.map(\.longitude).reduce(0, +) / Double(inset.count), accuracy: 1e-9)
    }

    func testAShapeWithFewerCornersThanTheViewIsRefusedRatherThanHalfWritten() {
        XCTAssertFalse(fitSurveyArea(survey(3), insetRing(view, 0.8)))
        XCTAssertFalse(fitSurveyArea(survey(4), []))
    }

    func testNothingToInsetYieldsNothingRatherThanADegenerateRing() {
        XCTAssertEqual([], insetRing([], 0.8))
        XCTAssertEqual([], insetRing(Array(view.prefix(2)), 0.8))
    }
}
