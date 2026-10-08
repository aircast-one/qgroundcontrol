import XCTest
@testable import Aircast

final class CameraCalcSectionTests: XCTestCase {
    private func control(_ suffix: String) -> JSON {
        JSON.parse(#"{"class":"Control","path":"plan.missionController.visualItems.1.\#(suffix)","name":"\#(suffix)","pathSuffix":"\#(suffix)","value":1,"valueString":"1"}"#)
    }

    private func view(_ brand: String, _ custom: Bool, _ byDistance: Bool) -> JSON {
        let suffixes = ["cameraCalc.sensorWidth", "cameraCalc.distanceToSurface", "cameraCalc.imageDensity", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap", "cameraCalc.adjustedFootprintFrontal", "cameraCalc.adjustedFootprintSide"]
        return .object(["camera": .object([
            "brand": .string(brand), "manualName": .string("Manual (no camera specs)"), "custom": .bool(custom), "valueSetIsDistance": .bool(byDistance),
            "facts": .array(suffixes.map(control)),
        ])])
    }

    private func shown(_ brand: String, _ custom: Bool, _ byDistance: Bool) -> [String] {
        shownCameraFacts(cameraCalc(view(brand, custom, byDistance))!).map { $0.path.components(separatedBy: "visualItems.1.").last ?? $0.path }
    }

    func testTheGridShowsTheFigureThatSetsTheOtherAsCameraCalcGridDoes() {
        let spacing = ["cameraCalc.adjustedFootprintFrontal", "cameraCalc.adjustedFootprintSide"]
        XCTAssertEqual(["cameraCalc.sensorWidth", "cameraCalc.distanceToSurface", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap"] + spacing, shown("Sony", false, true))
        XCTAssertEqual(["cameraCalc.sensorWidth", "cameraCalc.imageDensity", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap"] + spacing, shown("Sony", false, false))
        XCTAssertEqual(["cameraCalc.sensorWidth", "cameraCalc.distanceToSurface", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap"] + spacing, shown("Custom Camera", true, true))
        XCTAssertEqual(
            ["cameraCalc.distanceToSurface", "cameraCalc.adjustedFootprintFrontal", "cameraCalc.adjustedFootprintSide"],
            shown("Manual (no camera specs)", false, false),
            "a manual camera sets its trigger distance and spacing by hand"
        )
    }

    func testSetByNamesTheDistanceTheWayTheEditorDoes() {
        let scan = JSON.object((control("cameraCalc.distanceToSurface").object ?? [:]).merging(["shortLabel": .string("Scan distance")]) { $1 })
        var block = cameraCalc(.object(["camera": .object(["brand": .string("Sony"), "facts": .array([scan])])]))!
        XCTAssertEqual("Scan distance", distanceLabel(block))
        block.facts = []
        XCTAssertEqual("Altitude", distanceLabel(block))
    }

    func testTheTransectAltitudeFrameReadsFromTheCoreAndHidesWhereItDoesNotApply() {
        var block = cameraCalc(JSON.parse(#"""
        {"camera":{"brand":"Manual (no camera specs)","manualName":"Manual (no camera specs)","distanceMode":3,"distanceModePath":"p.cameraCalc.distanceMode",
            "distanceModes":[{"raw":1,"title":"Relative (Rel)"},{"raw":3,"title":"Above Terrain Calced (AGLC)"}],"facts":[]}}
        """#))!
        XCTAssertEqual("Above Terrain Calced (AGLC)", distanceModeTitle(block))
        XCTAssertEqual("p.cameraCalc.distanceMode", block.distanceModePath)
        block.distanceMode = 5
        XCTAssertNil(distanceModeTitle(block))
    }
}
