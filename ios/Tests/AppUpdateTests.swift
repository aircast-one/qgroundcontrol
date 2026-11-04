#if os(macOS)
import XCTest
@testable import Aircast

final class AppUpdateTests: XCTestCase {
    private let dmg = "/Users/pilot/Library/Caches/Aircast/AircastQGCNative-v5.6.5-beta.22.dmg"

    private func reading(
        _ phase: String,
        canInstall: Bool = true,
        skipped: String = "",
        later: Bool = false,
        updatedFrom: String = "",
        error: String = "",
        progress: Double = 0.0,
        latest: String = "5.6.5-beta.22",
        file: String? = nil
    ) -> AppUpdateReading {
        let held = file ?? dmg
        return appUpdateReading(JSON.parse(###"""
            {"class":"AppUpdate","current":"5.6.5-beta.21","channel":"beta","phase":"\###(phase)","latest":"\###(latest)",
            "notes":"## Fixes\n\n- a [link](https://x) here\n\n**bold**","releaseUrl":"https://github.com/r","assetName":"AircastQGCNative-v5.6.5-beta.22.dmg",
            "assetUrl":"https://dl/a.dmg","sizeBytes":151340969,"progress":\###(progress),"file":"\###(held)","error":"\###(error)",
            "checkedAt":"2026-10-31T10:00:00Z","autoCheck":true,"skipped":"\###(skipped)","later":\###(later),"updatedFrom":"\###(updatedFrom)","canInstall":\###(canInstall)}
            """###))!
    }

    func testEveryPhaseHasAStatusLineAndTheLatestVersionIsNamedWhereItMatters() {
        XCTAssertEqual(updateStatus(reading("idle")), "")
        XCTAssertEqual(updateStatus(reading("checking")), "Checking for updates…")
        XCTAssertEqual(updateStatus(reading("upToDate")), "Aircast is up to date")
        XCTAssertEqual(updateStatus(reading("available")), "Aircast 5.6.5-beta.22 is available")
        XCTAssertEqual(updateStatus(reading("downloading", progress: 0.45)), "Downloading Aircast 5.6.5-beta.22… 45%")
        XCTAssertEqual(updateStatus(reading("ready")), "Aircast 5.6.5-beta.22 is ready to install")
        XCTAssertEqual(updateStatus(reading("failed", error: "a.dmg did not match its published checksum")), "a.dmg did not match its published checksum")
        XCTAssertEqual(updateStatus(reading("failed")), "The update check failed.")
        XCTAssertEqual(reading("something new").phase, .Idle)
    }

    func testTheBannerShowsForAnAvailableDownloadingOrReadyUpdateAndForNothingElse() {
        XCTAssertTrue(bannerShown(reading("available")))
        XCTAssertTrue(bannerShown(reading("downloading")))
        XCTAssertTrue(bannerShown(reading("ready")))
        for phase in ["idle", "checking", "upToDate", "failed"] {
            XCTAssertFalse(bannerShown(reading(phase)), phase)
        }
    }

    func testAnOpenLinkHidesTheBannerAndBlocksTheInstallButton() {
        XCTAssertFalse(bannerShown(reading("available", canInstall: false)))
        XCTAssertEqual(installRefusal(reading("ready", canInstall: false)), DISCONNECT_FIRST)
        XCTAssertNil(installRefusal(reading("ready")))
    }

    func testLaterHidesTheBannerForThisRunAndASkipHidesOnlyTheVersionItNamed() {
        XCTAssertFalse(bannerShown(reading("available", later: true)))
        XCTAssertFalse(bannerShown(reading("available", skipped: "5.6.5-beta.22")))
        XCTAssertTrue(skippedThis(reading("ready", skipped: "5.6.5-beta.22")))
        XCTAssertTrue(bannerShown(reading("available", skipped: "5.6.5-beta.21")))
        XCTAssertEqual(updateStatus(reading("available", skipped: "5.6.5-beta.22")), "Aircast 5.6.5-beta.22 is available")
    }

    func testTheUpgradeIsReportedOnceNotWhileAVehicleIsConnectedAndLinksToTheReleaseThatIsRunning() {
        XCTAssertTrue(updatedShown(reading("upToDate", updatedFrom: "5.6.5-beta.20")))
        XCTAssertFalse(updatedShown(reading("upToDate")))
        XCTAssertFalse(updatedShown(reading("upToDate", canInstall: false, updatedFrom: "5.6.5-beta.20")))
        XCTAssertEqual(releaseTagUrl(reading("upToDate").current), "https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v5.6.5-beta.21")
    }

    func testAnInstallFailureReplacesTheReadyLine() {
        XCTAssertEqual(updateStatus(reading("ready"), failure: .NotOpened), InstallFailure.NotOpened.text)
        XCTAssertEqual(updateStatus(reading("ready"), failure: .Missing), InstallFailure.Missing.text)
    }

    func testEachFailureOffersItsOwnWayForward() {
        XCTAssertEqual(readyAction(.Missing), .OpenReleasePage)
        XCTAssertEqual(readyAction(.NotOpened), .Install)
        XCTAssertEqual(readyAction(nil), .Install)
        XCTAssertEqual(installLabel(.NotOpened), "Try again")
        XCTAssertEqual(installLabel(nil), "Install")
    }

    func testAStoredInstallFailureOnlyStandsBesideTheReadyFileItCameFrom() {
        let failed = InstallOutcome(file: dmg, failure: .NotOpened)
        XCTAssertEqual(installFailureFor(failed, reading("ready")), .NotOpened)
        XCTAssertNil(installFailureFor(failed, reading("ready", canInstall: false)))
        XCTAssertNil(installFailureFor(failed, reading("available")))
        XCTAssertNil(installFailureFor(failed, reading("ready", latest: "5.6.5-beta.23", file: "/Users/pilot/Library/Caches/Aircast/AircastQGCNative-v5.6.5-beta.23.dmg")))
        XCTAssertNil(installFailureFor(nil, reading("ready")))
    }

    func testTheSizeRidesOnTheButtonsAndInTheMeteredQuestion() {
        XCTAssertEqual(sizedLabel("Update", "151 MB"), "Update (151 MB)")
        XCTAssertEqual(sizedLabel("Download", nil), "Download")
        XCTAssertEqual(meteredQuestion("151 MB"), "This update is 151 MB and you're on a metered connection. Download now?")
        XCTAssertEqual(meteredQuestion(nil), "You're on a metered connection. Download the update now?")
        XCTAssertNil(updateSizeText(0))
        XCTAssertNotNil(updateSizeText(reading("available").sizeBytes))
    }

    func testReleaseNotesLoseTheirMarkdownButKeepTheirLines() {
        XCTAssertEqual(plainNotes(reading("available").notes), "Fixes\n\na link here\n\nbold")
        XCTAssertEqual(checkedAtMillis("2026-10-31T10:00:00Z"), 1_793_440_800_000)
        XCTAssertNil(checkedAtMillis(""))
    }
}
#endif
