import XCTest
@testable import Aircast

final class BridgeTests: XCTestCase {
    func testJsonReadsLikeOrgJson() {
        let view = JSON.parse(#"{"class":"FlyState","armed":true,"rcSignal":null,"mode":"Hold","count":3,"ratio":2.5,"items":[1,"two"]}"#)
        XCTAssertEqual(view["class"].string, "FlyState")
        XCTAssertTrue(view["armed"].bool)
        XCTAssertTrue(view["rcSignal"].isNull)
        XCTAssertFalse(view.has("rcSignal"))
        XCTAssertEqual(view["missing"].string, "")
        XCTAssertEqual(view["count"].int, 3)
        XCTAssertEqual(view["count"].string, "3")
        XCTAssertEqual(view["ratio"].double, 2.5)
        XCTAssertEqual(view["items"][1].string, "two")
        XCTAssertEqual(view["missing"].int(-1), -1)
        XCTAssertTrue(view["missing"].bool(true))
    }

    func testJsonEncodesArgumentsForTheBridge() {
        XCTAssertEqual(JSON.encode([1, "a", true, nil] as [Any?]), #"[1,"a",true,null]"#)
        XCTAssertEqual(JSON.encode(["value": Double.nan]), #"{"value":null}"#)
        XCTAssertEqual(JSON.encode(2.5), "2.5")
    }

    func testRefusalExplainsAFailedAnswer() {
        XCTAssertNil(refusal(JSON.parse(#"{"ok":true}"#)))
        XCTAssertEqual(refusal(JSON.parse(#"{"ok":false,"reason":"Not armed"}"#)), "Not armed")
        XCTAssertEqual(refusal(JSON.parse(#"{"ok":false}"#)), "The vehicle refused.")
        XCTAssertEqual(refusal(nil), "The vehicle did not answer.")
    }

    func testFactReadsCoreMetadata() {
        let fact = Qgc.fact("settings.appSettings", JSON.parse(#"{"name":"indoorPalette","shortDescription":"Color scheme:","valueString":"Dark","value":1,"enumStrings":["Light","Dark"],"enumValues":["0","1"],"enumIndex":1,"typeIsInteger":true,"defaultValueAvailable":true,"valueEqualsDefault":false}"#))
        XCTAssertEqual(fact.path, "settings.appSettings.indoorPalette")
        XCTAssertEqual(fact.title, "Color scheme")
        XCTAssertTrue(fact.isEnum)
        XCTAssertTrue(fact.changedFromDefault)
        XCTAssertTrue(fact.wholeNumbersOnly)
    }

    func testSentenceCaseKeepsProperNouns() {
        XCTAssertEqual(sentenceCase("Show Telemetry Log Replay Status Bar"), "Show telemetry log replay status bar")
        XCTAssertEqual(sentenceCase("Use Google Maps"), "Use Google maps")
        XCTAssertEqual(sentenceCase("PX4 Pro Flight Stack"), "PX4 Pro flight stack")
        XCTAssertEqual(sentenceCase("RTK GPS"), "RTK GPS")
    }

    func testTileAddressesRoundTrip() {
        let url = qgcTileUrl("Bing Hybrid").replacingOccurrences(of: "{z}", with: "3")
            .replacingOccurrences(of: "{x}", with: "4").replacingOccurrences(of: "{y}", with: "5")
        XCTAssertEqual(URL(string: url).flatMap { tileAddress($0.path) }, TileAddress(mapType: "Bing Hybrid", z: 3, x: 4, y: 5))
    }
}
