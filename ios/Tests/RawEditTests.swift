import XCTest
@testable import Aircast

final class RawEditTests: XCTestCase {
    func testOnlySimpleItemsOfferShowAllValues() {
        XCTAssertNil(rawEdit(JSON.parse(#"{"simple":false}"#)))
        XCTAssertEqual(RawEdit(on: true, friendlyAllowed: false), rawEdit(JSON.parse(#"{"simple":true,"rawEdit":true,"friendlyEditAllowed":false}"#)))
    }

    func testAnItemThatCannotBeShownFriendlyStaysRaw() {
        XCTAssertEqual(RAW_EDIT_STUCK, rawEditRefusal(RawEdit(on: true, friendlyAllowed: false)))
        XCTAssertNil(rawEditRefusal(RawEdit(on: true, friendlyAllowed: true)))
        XCTAssertNil(rawEditRefusal(RawEdit(on: false, friendlyAllowed: true)))
    }

    func testAnUnfinishedPatternShapeShowsItsHelpInsteadOfItsSettings() {
        XCTAssertEqual("Use the Polygon Tools", areaHelp(JSON.parse(#"{"areaHelp":"Use the Polygon Tools"}"#)))
        XCTAssertNil(areaHelp(JSON.parse(#"{"areaHelp":null}"#)))
    }

    func testAPatternOffersItsStartCornerToRotate() {
        XCTAssertEqual(EntryPoint(label: "Start from", value: "top left", path: "p.rotateEntryPoint"), entryPoint(JSON.parse(#"{"entryPoint":{"label":"Start from","value":"top left","path":"p.rotateEntryPoint"}}"#)))
        XCTAssertNil(entryPoint(JSON.parse(#"{"entryPoint":null}"#)))
    }

    func testALandingPatternTakesTheVehicleHeadingAndPosition() {
        XCTAssertTrue(itemIsLandingPattern(JSON.parse(#"{"landing":true}"#)))
        XCTAssertEqual(87.5, vehicleHeading(JSON.parse(#"{"value":87.5}"#)))
        XCTAssertNil(vehicleHeading(JSON.parse(#"{"value":null}"#)))
        XCTAssertEqual(47.4, vehicleCoordinate(JSON.parse(#"{"latitude":47.4,"longitude":8.5,"valid":true}"#))?["latitude"].double)
        XCTAssertNil(vehicleCoordinate(JSON.parse(#"{"latitude":0,"longitude":0,"valid":false}"#)))
    }
}
