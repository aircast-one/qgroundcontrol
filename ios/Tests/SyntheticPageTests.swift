import UIKit
import WebKit
import XCTest
@testable import Aircast

private let TERRARIUM_ZERO = UIColor(red: 128 / 255, green: 0, blue: 0, alpha: 1)
private let RENDER_DEADLINE_S = 60.0

private final class LoadWaiter: NSObject, WKNavigationDelegate {
    let finished: XCTestExpectation
    init(_ finished: XCTestExpectation) { self.finished = finished }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) { finished.fulfill() }
}

private func flatTerrainTile() -> Data {
    UIGraphicsImageRenderer(size: CGSize(width: 256, height: 256), format: { let format = UIGraphicsImageRendererFormat(); format.scale = 1; return format }())
        .pngData { context in
            TERRARIUM_ZERO.setFill()
            context.fill(CGRect(x: 0, y: 0, width: 256, height: 256))
        }
}

@MainActor
final class SyntheticPageTests: XCTestCase {
    func testThePageDrawsTheViewWithTerrainBuiltInItsWorkers() async throws {
        let tile = flatTerrainTile()
        let original = MapTileHost.fetch
        MapTileHost.fetch = { _, reply in reply(tile); return 0 }
        defer { MapTileHost.fetch = original }
        let loaded = expectation(description: "page loaded")
        let waiter = LoadWaiter(loaded)
        let web = syntheticWebView(waiter)
        web.frame = CGRect(x: 0, y: 0, width: 320, height: 180)
        let window = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { ($0 as? UIWindowScene)?.keyWindow }.first)
        window.addSubview(web)
        defer { web.removeFromSuperview() }
        await fulfillment(of: [loaded], timeout: RENDER_DEADLINE_S)
        let pose = JSON.parse(#"{"available":true,"latitude":-35.364,"longitude":149.164,"aboveHome":60,"homeLatitude":-35.3630336,"homeLongitude":149.165248,"heading":20,"pitch":-25,"roll":0,"fov":70,"imagery":"Bing Satellite","aimable":true,"pan":0}"#)
        let rendered = "document.body.className === 'placed' && scene.globe.tilesLoaded && scene.frameState.frameNumber > 10"
        let deadline = Date().addingTimeInterval(RENDER_DEADLINE_S)
        let drawn = try await drawnBefore(deadline, web, pose, rendered)
        XCTAssertTrue(drawn, "the camera is placed above home and the terrain tiles finish, which needs Cesium's workers behind the aircast:// scheme")
    }

    private func drawnBefore(_ deadline: Date, _ web: WKWebView, _ pose: JSON, _ check: String) async throws -> Bool {
        _ = try? await web.evaluateJavaScript(syntheticPoseScript(pose))
        let drawn = (try? await web.evaluateJavaScript(check)) as? Bool ?? false
        guard !drawn, Date() < deadline else { return drawn }
        try await Task.sleep(for: .milliseconds(500))
        return try await drawnBefore(deadline, web, pose, check)
    }
}
