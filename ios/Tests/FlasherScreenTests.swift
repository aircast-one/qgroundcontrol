#if os(macOS)
import XCTest
@testable import Aircast

final class FlasherScreenTests: XCTestCase {
    private let view = JSON.parse(
        #"""
        {"available":true,"channel":"stable","channels":["stable","staging","development"],
         "releases":{"state":"ready","error":null,"items":[{"version":"v0.3.5","label":"Aircast OS v0.3.5","prerelease":false,"published":"2026-08-22","size":576627168,"sizeText":"577 MB"}]},
         "selected":"v0.3.5",
         "form":{"hostname":"falcon-01","ssid":"field-net","wifiPassword":"","noWifi":false,"country":"GE","sshMode":"key-only","authorizedKey":"","devicePassword":"","authKey":"","controlServer":""},
         "problems":{"ssid":"Enter the WiFi network name, or choose no WiFi."},
         "keyIdentity":{"algorithm":"ssh-ed25519","comment":"operator@base","fingerprint":"SHA256:abc"},
         "summary":[{"label":"Hostname","value":"falcon-01.local","warn":false},{"label":"Device access","value":"Image default (pi / raspberry)","warn":true}],
         "disks":[{"id":"disk4","name":"Generic SD Reader"}],
         "card":{"state":"ready","id":"disk4","label":"Generic SD","capacity":16000000000,"capacityText":"16.0 GB"},
         "download":{"state":"running","version":"v0.3.5","done":10,"total":40,"percent":25.0},
         "job":{"phase":"writing","busy":true,"cancellable":true,"done":1,"total":4,"percent":25.0,"speed":12300000,"speedText":"12 MB/s","remainingText":"about 4 min left","error":null,"hostname":"falcon-01"},
         "firstProblem":{"field":"ssid","message":"Enter the WiFi network name, or choose no WiFi."},
         "canStart":false,"blocked":"A card is being written"}
        """#
    )

    private func parsed() throws -> FlasherState { try XCTUnwrap(flasherState(view)) }

    private func state(phase: String = "idle", busy: Bool = false, card: String = "none", disks: [FlasherDisk] = [FlasherDisk(id: "disk4", name: "Reader")]) throws -> FlasherState {
        withChanges(try parsed()) {
            $0.phase = phase
            $0.busy = busy
            $0.cardState = card
            $0.disks = disks
        }
    }

    func testTheCoreViewBecomesScreenState() throws {
        let read = try parsed()
        XCTAssertEqual(read.releases, [FlasherRelease(version: "v0.3.5", label: "Aircast OS v0.3.5", published: "2026-08-22", sizeText: "577 MB")])
        XCTAssertEqual(read.form.hostname, "falcon-01")
        XCTAssertEqual(read.form.country, "GE")
        XCTAssertEqual(read.problems["ssid"], "Enter the WiFi network name, or choose no WiFi.")
        XCTAssertEqual(read.keyIdentity, "ssh-ed25519 SHA256:abc operator@base")
        XCTAssertEqual(read.summary[1], FlasherSummaryRow(label: "Device access", value: "Image default (pi / raspberry)", warn: true))
        XCTAssertEqual(read.cardText, "Generic SD · 16.0 GB")
        XCTAssertEqual(read.downloadText, "Downloading Aircast OS… 25%")
        XCTAssertEqual(read.downloadState, "running")
        XCTAssertEqual(read.percent, 25)
        XCTAssertEqual(read.blocked, "A card is being written")
    }

    func testAMissingOrForeignViewIsNoState() {
        XCTAssertNil(flasherState(nil))
        XCTAssertNil(flasherState(JSON.parse(#"{"ports":[]}"#)))
    }

    func testCardAndDownloadStatesReadAsSentences() {
        XCTAssertEqual(flasherCardText(JSON.parse(#"{"state":"waiting"}"#)), "Allow Aircast to use the card reader when asked")
        XCTAssertEqual(flasherCardText(JSON.parse(#"{"state":"failed","error":"no card in the reader"}"#)), "no card in the reader")
        XCTAssertEqual(flasherCardText(JSON.parse(#"{"state":"none"}"#)), "")
        XCTAssertEqual(flasherDownloadText(JSON.parse(#"{"state":"failed","error":"offline"}"#)), "Download failed: offline")
        XCTAssertEqual(flasherDownloadText(JSON.parse(#"{"state":"ready","imageSizeText":"3.9 GB"}"#)), "Aircast OS is downloaded")
    }

    func testEveryJobPhaseHasATitle() {
        for phase in ["downloading", "preparing", "writing", "verifying", "customizing", "done", "failed", "cancelled"] {
            XCTAssertFalse(flasherPhaseLabel(phase).isBlank, phase)
        }
        XCTAssertEqual(flasherPhaseLabel("idle"), "")
    }

    func testARunningOrFinishedJobAlwaysShowsTheWriteStep() throws {
        XCTAssertEqual(flasherStepFor(try state(phase: "writing", busy: true)), .Write)
        XCTAssertEqual(flasherStepFor(try state(phase: "done")), .Write)
        XCTAssertNil(flasherStepFor(try state()))
        XCTAssertNil(flasherStepFor(nil))
    }

    func testASingleReaderOpensByItselfButAChoiceIsLeftToTheUser() throws {
        XCTAssertEqual(flasherReaderToOpen(try state()), "disk4")
        XCTAssertNil(flasherReaderToOpen(try state(card: "opening")))
        XCTAssertNil(flasherReaderToOpen(try state(disks: [FlasherDisk(id: "a", name: "A"), FlasherDisk(id: "b", name: "B")])))
        XCTAssertNil(flasherReaderToOpen(try state(disks: [])))
        XCTAssertNil(flasherReaderToOpen(try state(phase: "done")))
    }

    func testProblemsShowOnceAFieldIsTouchedOrContinueIsPressed() {
        let problems = ["ssid": "Enter the WiFi network name, or choose no WiFi.", "hostname": "Use lowercase letters"]
        XCTAssertEqual(flasherVisibleProblems(problems, [], false), [:])
        XCTAssertEqual(Set(flasherVisibleProblems(problems, ["hostname"], false).keys), ["hostname"])
        XCTAssertEqual(flasherVisibleProblems(problems, [], true), problems)
        XCTAssertFalse(flasherSetupComplete(problems))
        XCTAssertTrue(flasherSetupComplete([:]))
    }

    func testAMacWithoutAKeySignsInWithAPassword() {
        XCTAssertEqual(flasherDefaultSshMode(FlasherForm()), FLASHER_PASSWORD_MODE)
        XCTAssertNil(flasherDefaultSshMode(FlasherForm(authorizedKey: "ssh-ed25519 AAAA")))
        XCTAssertNil(flasherDefaultSshMode(FlasherForm(sshMode: "disabled")))
    }

    func testAMeteredConnectionIsWarnedAboutOnlyBeforeTheDownloadStarts() {
        let release = FlasherRelease(version: "v0.3.5", label: "Aircast OS v0.3.5", published: "2026-08-22", sizeText: "577 MB", recommended: true)
        XCTAssertEqual(flasherMeteredNote(release, "idle", true), "Aircast OS is a 577 MB download and this Mac is on a metered connection.")
        XCTAssertNil(flasherMeteredNote(release, "idle", false))
        XCTAssertNil(flasherMeteredNote(release, "running", true))
        XCTAssertNil(flasherMeteredNote(release, "ready", true))
        XCTAssertNil(flasherMeteredNote(nil, "idle", true))
    }

    func testTheFirstProblemIsNamedByItsFieldAndOpensMoreOptionsWhenItLivesThere() throws {
        let read = try parsed()
        XCTAssertEqual(flasherProblemNote(read), "WiFi network: Enter the WiFi network name, or choose no WiFi.")
        XCTAssertFalse(flasherProblemInMoreOptions(read))
        XCTAssertTrue(flasherProblemInMoreOptions(withChanges(read) { $0.firstProblemField = "controlServer" }))
        XCTAssertEqual(flasherProblemNote(withChanges(read) { $0.firstProblem = "" }), "")
        XCTAssertEqual(read.remainingText, "about 4 min left")
    }

    func testInternalChannelsStayOutOfCustomerBuilds() {
        XCTAssertFalse(flasherShowsChannels(false, "stable"))
        XCTAssertTrue(flasherShowsChannels(true, "stable"))
        XCTAssertTrue(flasherShowsChannels(false, "staging"))
    }

    func testTheMacStaysAwakeWhileDownloadingOrWriting() throws {
        XCTAssertTrue(flasherKeepsAwake(try state(phase: "writing", busy: true)))
        XCTAssertTrue(flasherKeepsAwake(withChanges(try parsed()) {
            $0.busy = false
            $0.phase = "idle"
            $0.downloadState = "running"
        }))
        XCTAssertFalse(flasherKeepsAwake(withChanges(try parsed()) {
            $0.busy = false
            $0.phase = "idle"
            $0.downloadState = "ready"
        }))
        XCTAssertFalse(flasherKeepsAwake(nil))
    }

    func testTheEndingSaysWhatHappensNext() {
        let steps = flasherNextSteps("field-net", "falcon-01", false)
        XCTAssertEqual(steps.count, 3)
        XCTAssertTrue(steps[1].contains("joins field-net"), steps[1])
        XCTAssertTrue(steps[2].contains("http://falcon-01.local"), steps[2])
        XCTAssertTrue(flasherNextSteps("", "falcon-01", true)[1].contains("Ethernet or cellular"))
    }

    func testBackReturnsToThePreviousStepUnlessWritingOrFinished() {
        XCTAssertEqual(flasherPreviousStep(.Write, false, "idle"), .Setup)
        XCTAssertEqual(flasherPreviousStep(.Setup, false, "idle"), .Card)
        XCTAssertNil(flasherPreviousStep(.Card, false, "idle"))
        XCTAssertNil(flasherPreviousStep(.Write, true, "writing"))
        XCTAssertNil(flasherPreviousStep(.Write, false, "done"))
    }

    func testAReleaseReadsAsItsDateSizeAndRecommendation() {
        XCTAssertEqual(
            flasherReleaseDetail(FlasherRelease(version: "v0.3.5", label: "Aircast OS v0.3.5", published: "2026-08-22", sizeText: "577 MB", recommended: true)),
            "2026-08-22 · 577 MB download · Recommended"
        )
        XCTAssertEqual(flasherReleaseDetail(FlasherRelease(version: "v0.3.5", label: "", published: "", sizeText: "577 MB")), "577 MB download")
    }

    func testTheActionNoteNamesARefusalThenTheSetupProblemThenWhatBlocksWriting() throws {
        let idle = try state()
        XCTAssertEqual(flasherActionNote(.Write, idle, "The card was removed", ""), "The card was removed")
        XCTAssertEqual(flasherActionNote(.Setup, idle, "", "WiFi network: missing"), "WiFi network: missing")
        XCTAssertEqual(flasherActionNote(.Write, idle, "", ""), "A card is being written")
        XCTAssertEqual(flasherActionNote(.Write, try state(phase: "writing", busy: true), "", ""), "")
        XCTAssertEqual(flasherActionNote(.Card, idle, "", "WiFi network: missing"), "")
    }
}
#endif
