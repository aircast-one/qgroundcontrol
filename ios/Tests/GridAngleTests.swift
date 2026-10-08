import XCTest
@testable import Aircast

final class GridAngleTests: XCTestCase {
    func testTheGridAngleReadsAsAWholeDegreeFrom0To359LikeSurveyItemEditorsSlider() {
        XCTAssertEqual(0, gridAngleShown(.nan), accuracy: 0)
        XCTAssertEqual(45, gridAngleShown(44.6), accuracy: 0)
        XCTAssertEqual(350, gridAngleShown(-10.0), accuracy: 0)
        XCTAssertEqual(0, gridAngleShown(360.0), accuracy: 0)
        XCTAssertEqual(359, gridAngleShown(359.2), accuracy: 0)
    }
}
