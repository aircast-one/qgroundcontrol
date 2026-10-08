import XCTest
@testable import Aircast

final class StructureScanRouteTests: XCTestCase {
    private func survey(transects: [TrackPoint] = [], loop: [TrackPoint] = []) -> Survey {
        Survey(index: 0, area: [], transects: transects, cameraShots: 0, kind: "structure", shape: "shape", property: "property", editable: nil, flightLoop: loop, layers: 3)
    }

    private let square = [TrackPoint(41.0, 44.0), TrackPoint(41.0, 44.1), TrackPoint(41.1, 44.1)]

    func testAStructureScanFliesAClosedLoopSoTheRouteReturnsToWhereItStarted() {
        let route = flownRoute(survey(loop: square))
        XCTAssertEqual(square.count + 1, route.count)
        XCTAssertEqual(route.first, route.last)
    }

    func testALoopAlreadyClosedIsNotClosedTwice() {
        let closed = square + [square[0]]
        XCTAssertEqual(closed, flownRoute(survey(loop: closed)))
    }

    func testASurveyMowsSoItsTransectsAreTheRouteAndAreLeftOpen() {
        XCTAssertEqual(square, flownRoute(survey(transects: square)))
    }

    func testAnItemWithNeitherDrawsNoRouteRatherThanADegenerateOne() {
        XCTAssertEqual([], flownRoute(survey()))
        XCTAssertEqual([], flownRoute(survey(loop: Array(square.prefix(1)))))
    }
}

final class StructureScanLayersTests: XCTestCase {
    private func survey(_ layers: Int, _ span: String = "") -> Survey {
        Survey(index: 0, area: [], transects: [], cameraShots: 0, kind: "structure", shape: "shape", property: "property", editable: nil, flightLoop: [], layers: layers, layerSpanText: span)
    }

    func testStackedCircuitsAreSaidInWordsBecauseOnAMapTheySitExactlyOnEachOther() {
        XCTAssertEqual("3 layers, one drawn", layersText(survey(3)))
    }

    func testTheSpanIsTheCoresSentenceSoAFeetRigDoesNotReadMetres() {
        XCTAssertEqual("3 layers, 12.0 m to 42.0 m, one drawn", layersText(survey(3, "12.0 m to 42.0 m")))
        XCTAssertEqual("3 layers, 39 ft to 138 ft, one drawn", layersText(survey(3, "39 ft to 138 ft")))
    }

    func testASpanTheCoreWithheldLeavesTheCountAloneRatherThanInventingOne() {
        XCTAssertEqual("3 layers, one drawn", layersText(survey(3)))
    }

    func testASingleLayerSaysNothingBecauseTheLoopDrawnIsTheWholeMission() {
        XCTAssertNil(layersText(survey(1)))
        XCTAssertNil(layersText(survey(0)))
        XCTAssertNil(layersText(nil))
    }
}

final class PatternNameTests: XCTestCase {
    private func item(_ index: Int, _ name: String) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: 41.0, longitude: 44.0, command: name, selected: false)
    }

    func testAStructureScanIsNotCalledASurveyBecauseTheCoreAlreadyNamedIt() {
        let items = [item(2, "Structure Scan"), item(3, "Corridor Scan")]
        XCTAssertEqual("Structure Scan", patternName(2, items))
        XCTAssertEqual("Corridor Scan", patternName(3, items))
    }

    func testAnItemTheListDoesNotHoldFallsBackToAWordThatIsTrueOfAllThree() {
        XCTAssertEqual("pattern", patternName(9, []))
        XCTAssertEqual("pattern", patternName(2, [item(2, "")]))
    }
}
