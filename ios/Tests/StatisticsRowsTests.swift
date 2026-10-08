import XCTest
@testable import Aircast

final class StatisticsRowsTests: XCTestCase {
    func testTransectsAndStructuresListQgcsStatisticsRows() {
        let survey = SurveyStats(areaText: "1.2 ha", warning: "", intervalText: "2.0 s", distanceText: "840 m", photosText: "120")
        XCTAssertEqual(["Area", "Distance", "Photos", "Photo interval"], statisticsRows(survey).map(\.0))
        var structure = survey
        structure.structure = StructureStats(layers: "3", layerHeight: "10 m", top: "40 m", bottom: "20 m")
        let rows = statisticsRows(structure)
        XCTAssertEqual(["Layers", "Layer height", "Top layer altitude", "Bottom layer altitude", "Photos", "Photo interval"], rows.map(\.0))
        XCTAssertEqual(["3", "10 m", "40 m", "20 m", "120", "2.0 s"], rows.map(\.1))
    }
}
