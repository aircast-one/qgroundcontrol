import XCTest
@testable import Aircast

final class VehicleLinksTests: XCTestCase {
    private func view(
        available: Bool = true,
        watching: Bool = true,
        primary: String = #""LTE""#,
        contactLost: String = "false",
        links: String = #"[{"name":"LTE","primary":true,"commLost":false},{"name":"radio","primary":false,"commLost":false}]"#
    ) -> JSON {
        JSON.parse(#"{"kind":"object","class":"VehicleLinks","available":\#(available),"watching":\#(watching),"primary":\#(primary),"contactLost":\#(contactLost),"reason":"","links":\#(links)}"#)
    }

    func testAnotherViewIsNotALinksReading() {
        XCTAssertNil(vehicleLinks(nil))
        XCTAssertNil(vehicleLinks(JSON.parse(#"{"kind":"object","class":"AdsbTraffic"}"#)))
    }

    func testOneLinkIsTheOrdinaryCaseAndSaysNothing() {
        XCTAssertNil(linkCell(vehicleLinks(view(links: #"[{"name":"LTE","primary":true,"commLost":false}]"#))))
    }

    func testTwoHealthyLinksSaySoBecauseRedundancyIsTheThingBeingReported() {
        XCTAssertEqual(linkCell(vehicleLinks(view())), LinkCell(text: "2 links", degraded: false))
    }

    func testASilentRadioIsCountedNotNamedTheStripHasNoRoomForALinkName() {
        let degraded = view(links: #"[{"name":"LTE","primary":true,"commLost":false},{"name":"radio","primary":false,"commLost":true}]"#)
        XCTAssertEqual(linkCell(vehicleLinks(degraded)), LinkCell(text: "1 link lost", degraded: true))
    }

    func testEveryLinkSilentIsADifferentSentenceFromOneOfThemSilent() {
        let gone = view(contactLost: "true", links: #"[{"name":"LTE","primary":true,"commLost":true},{"name":"radio","primary":false,"commLost":true}]"#)
        XCTAssertEqual(linkCell(vehicleLinks(gone)), LinkCell(text: "no link heard", degraded: true))
    }

    func testAnUnwatchedVehicleReportsNoLossAndANullIsNotAFalse() throws {
        let unwatched = view(watching: false, contactLost: "null", links: #"[{"name":"LTE","primary":true,"commLost":null},{"name":"radio","primary":false,"commLost":null}]"#)
        let reading = try XCTUnwrap(vehicleLinks(unwatched))
        XCTAssertNil(reading.links[0].commLost)
        XCTAssertEqual(linkCell(reading), LinkCell(text: "2 links", degraded: false))
    }

    func testNoVehicleIsNoCell() {
        XCTAssertNil(linkCell(vehicleLinks(view(available: false, links: "[]"))))
    }

    func testTwoOfThreeSilentIsNotOneOfThem() {
        let worse = view(links: #"[{"name":"LTE","primary":true,"commLost":false},{"name":"radio","primary":false,"commLost":true},{"name":"spare","primary":false,"commLost":true}]"#)
        XCTAssertEqual(linkCell(vehicleLinks(worse)), LinkCell(text: "2 links lost", degraded: true))
    }

    func testOneRadioGoingQuietSaysNothingHereAndTheNoContactBannerIsWhy() {
        let alone = JSON.parse(#"{"kind":"object","class":"VehicleLinks","available":true,"links":[{"name":"Telemetry","primary":true,"commLost":true}]}"#)
        XCTAssertNil(
            linkCell(vehicleLinks(alone)),
            "the cell is about which of several links is carrying; with one link there is no which, and flyState.contactLost already puts a banner on the screen"
        )
    }
}
