import XCTest
@testable import Aircast

final class PlanFileRulesTests: XCTestCase {
    func testAReasonTheCoreDeclinesToGiveDoesNotReachTheScreenAsTheWordNull() {
        XCTAssertEqual("The plan could not be checked for saving.", saveBlockedReason(JSON.parse(#"{"readiness":{"ready":false,"reason":null}}"#)))
        XCTAssertEqual("The plan could not be checked for saving.", saveBlockedReason(JSON.parse(#"{"readiness":{"ready":false,"reason":""}}"#)))
        XCTAssertEqual("This plan has no items.", saveBlockedReason(JSON.parse(#"{"readiness":{"ready":false,"reason":"This plan has no items."}}"#)))
    }

    func testTheShpOfAPickedShapefileSetIsTheBoundaryToRead() {
        XCTAssertEqual("area.SHP", mainBoundaryName(["area.prj", "area.SHP", "area.dbf"]))
        XCTAssertEqual("route.kml", mainBoundaryName(["route.kml"]))
        XCTAssertNil(mainBoundaryName([]))
    }

    func testASavedNameWithoutThePlanSuffixGetsOneAsQgcFileDialogAppendsIt() {
        XCTAssertEqual("survey.plan", withExtension("survey", PLAN_EXTENSION))
        XCTAssertEqual("survey.PLAN", withExtension("survey.PLAN", PLAN_EXTENSION))
        XCTAssertEqual("survey.txt.plan", withExtension("survey.txt", PLAN_EXTENSION))
        XCTAssertEqual("area.kml", withExtension("area", KML_EXTENSION))
    }

    func testAPickedBoundaryIsStagedUnderItsOwnLowerCasedExtensionOrKmlWhenItHasNone() {
        XCTAssertEqual("boundary.kml", boundaryCacheName(nil))
        XCTAssertEqual("boundary.shp", boundaryCacheName("Field.SHP"))
        XCTAssertEqual("boundary.kml", boundaryCacheName("field."))
        XCTAssertEqual("boundary.kml", boundaryCacheName("field"))
        XCTAssertEqual("boundary.kml", boundaryCacheName("area.prj.KML"))
    }

    func testOnlyACancelledPickerStaysQuietEveryOtherPickerFailureIsReported() {
        XCTAssertTrue(userCancelled(CocoaError(.userCancelled)))
        XCTAssertFalse(userCancelled(CocoaError(.fileReadNoPermission)))
        XCTAssertFalse(userCancelled(URLError(.cancelled)))
    }
}
