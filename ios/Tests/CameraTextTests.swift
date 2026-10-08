import XCTest
@testable import Aircast

final class CameraTextTests: XCTestCase {
    private func stats(_ json: String) -> SurveyStats? { surveyStats(JSON.parse(json)) }

    func testASurveySaysHowHighAboveTheSurfaceWhatAShotCoversAndHowOftenItFires() {
        let view = stats(#"{"available":true,"areaText":"9.0 ha","warning":"","surfaceDistanceText":"60.0 m","footprintText":"12.5 × 8.0 m","intervalText":"2.4 s"}"#)
        XCTAssertEqual("60.0 m above the surface · each shot covers 12.5 × 8.0 m · a shot every 2.4 s", cameraText(view))
    }

    func testTheCoresEmDashIsAnAbsenceNotAFigureToPrint() {
        let view = stats(#"{"available":true,"areaText":"—","warning":"","surfaceDistanceText":"—","footprintText":"—","intervalText":"—"}"#)
        XCTAssertNil(cameraText(view))
        XCTAssertEqual("", view?.areaText)
    }

    func testASurveyTheCoreAnsweredPartlySaysThePartItKnows() {
        let view = stats(#"{"available":true,"areaText":"9.0 ha","warning":"","footprintText":"12.5 × 8.0 m"}"#)
        XCTAssertEqual("each shot covers 12.5 × 8.0 m", cameraText(view))
    }

    func testNoSurveyAndNoStatsSayNothingRatherThanAnEmptyRow() {
        XCTAssertNil(cameraText(nil))
        XCTAssertNil(cameraText(stats(#"{"available":false}"#)))
    }

    func testAnUnavailableSurveyHasNoStatsToShow() {
        XCTAssertNil(surveyStats(nil))
        XCTAssertNil(surveyStats(JSON.parse(#"{"available":false,"areaText":"12 ha"}"#)))
    }
}
