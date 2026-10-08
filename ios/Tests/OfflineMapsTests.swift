import XCTest
@testable import Aircast

final class OfflineMapsTests: XCTestCase {
    func testTheVisibleCornersBecomeTheRegionTheUrlFactoryCounts() throws {
        let region = try XCTUnwrap(offlineRegion([
            TrackPoint(latitude: 47.40, longitude: 8.50),
            TrackPoint(latitude: 47.40, longitude: 8.60),
            TrackPoint(latitude: 47.35, longitude: 8.60),
            TrackPoint(latitude: 47.35, longitude: 8.50),
        ]))
        XCTAssertEqual(region, OfflineRegion(west: 8.50, north: 47.40, east: 8.60, south: 47.35))
        XCTAssertEqual(
            offlineMapsPath("Google Satellite", region, 13, 19),
            "view.offlineMaps(Google Satellite,8.5000000,47.4000000,8.6000000,47.3500000,13,19,true)"
        )
        XCTAssertEqual(offlineMapsPath("Google Satellite", nil, 13, 19), "view.offlineMaps")
        XCTAssertNil(offlineRegion([]))
    }

    func testSetsAndTheEstimateReadFromTheView() throws {
        let read = try XCTUnwrap(offlineMaps(JSON.parse(
            #"{"class":"OfflineMaps","available":true,"sets":[{"id":1,"name":"System Wide Tile Cache","defaultSet":true,"#
                + #""downloadStatus":"1.0MB","downloading":false,"complete":true}],"mapList":["Bing Road"],"uniqueName":"Tile Set 001","#
                + #""takenNames":["Default Tile Set"],"estimate":{"tileCountText":"1,234","tileSizeText":"5.0MB","tooMany":false}}"#
        )))
        XCTAssertEqual(read.sets.map(\.name), ["System Wide Tile Cache"])
        XCTAssertEqual(read.uniqueName, "Tile Set 001")
        XCTAssertEqual(read.estimate, OfflineEstimate(tileCountText: "1,234", tileSizeText: "5.0MB", tooMany: false))
    }

    func testOkRenamesOnlyToANewNonBlankName() {
        XCTAssertEqual(renameWanted("Set 1", " Valley "), "Valley")
        XCTAssertNil(renameWanted("Set 1", "Set 1"))
        XCTAssertNil(renameWanted("Set 1", "  "))
    }

    func testZoomPreviewsCentreOnTheChosenRegionLikeOfflineMapEditorsPreviewMaps() {
        XCTAssertEqual(regionCentre(OfflineRegion(west: 44.0, north: 42.0, east: 45.0, south: 41.0)), TrackPoint(latitude: 41.5, longitude: 44.5))
    }
}
