import XCTest
@testable import Aircast

final class HostNoticesTests: XCTestCase {
    private func view(_ unseen: String, destination: String? = nil, banners: String = "") -> JSON {
        JSON.parse(#"{"unseen":[\#(unseen)],"destination":\#(destination.map { "\"\($0)\"" } ?? "null"),"banners":[\#(banners)]}"#)
    }

    func testNothingUnseenIsNoBatch() {
        XCTAssertNil(noticeBatch(nil))
        XCTAssertNil(noticeBatch(view("")))
        XCTAssertNil(noticeBatch(view(#"{"kind":"message","title":"no id"}"#)))
    }

    func testABatchAcknowledgesThroughItsLatestIdAndCarriesTheCoresRules() throws {
        let batch = try XCTUnwrap(noticeBatch(view(
            #"{"id":4,"kind":"navigation","known":true},{"id":6,"kind":"somethingNew","known":false}"#,
            destination: "plan",
            banners: #""Battery · Low voltage""#
        )))
        XCTAssertEqual(batch.through, 6)
        XCTAssertEqual(batch.destination, "plan")
        XCTAssertEqual(batch.banners, ["Battery · Low voltage"])
        XCTAssertEqual(batch.unknownKinds, ["somethingNew"])
        XCTAssertEqual(hostNoticesPath(batch.through), "view.hostNotices(6)")
    }

    func testABannerShownInTheLastThirtySecondsIsHeldBackAndTheRestAreKeptOnce() {
        let shownAt: [String: Int64] = ["Battery · Low voltage": 1_000]
        XCTAssertEqual(quietBanners(["Battery · Low voltage", "GPS lost", "GPS lost"], shownAt, 20_000), ["GPS lost"])
        XCTAssertEqual(quietBanners(["Battery · Low voltage"], shownAt, 1_000 + REPEAT_QUIET_MS), ["Battery · Low voltage"])
    }

    func testAppMessagesArriveAsOkDialogsAsShowAppMessageOpensOne() {
        let batch = noticeBatch(JSON.parse(#"{"unseen":[{"id":4,"kind":"message"}],"banners":[],"dialogs":[{"title":"Aircast QGC","text":"Parameters missing","action":null},{"title":"Aircast QGC","text":"Reboot","action":"rebootVehicle"}]}"#))
        XCTAssertEqual(batch?.dialogs, [AppMessage(title: "Aircast QGC", text: "Parameters missing"), AppMessage(title: "Aircast QGC", text: "Reboot", action: REBOOT_VEHICLE_ACTION)])
    }

    func testASecondErrorInOneBatchIsStillCriticalAndSaysMoreArrived() {
        XCTAssertNil(criticalBanner([]))
        XCTAssertEqual(criticalBanner(["EKF failure"]), "EKF failure")
        XCTAssertEqual(criticalBanner(["EKF failure", "GPS glitch"]), "EKF failure \u{00b7} \(ADDITIONAL_ERRORS)")
    }
}
