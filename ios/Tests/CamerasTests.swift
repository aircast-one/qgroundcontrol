import XCTest
@testable import Aircast

final class CamerasTests: XCTestCase {
    private let view = JSON.parse("""
        {"kind":"object","class":"Cameras","readable":true,"reason":null,"active":2,
         "pip":{"enabled":false,"slot":0},
         "cameras":[
           {"slot":0,"stored":0,"title":"Front gimbal","short":"Front","name":"Front gimbal","source":"RTSP Video Stream","url":"rtsp://10.0.0.5:8554/front","problem":null,"fromDrone":false,"active":false,"status":"connecting"},
           {"slot":1,"stored":1,"title":"Camera 2","short":"Cam 2","name":"","source":"Back Camera","url":"","problem":"This kind of camera cannot show video in this app.","fromDrone":false,"active":false,"status":"bogus"},
           {"slot":2,"stored":null,"title":"SIYI A8","short":"SIYI","name":"SIYI A8","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600","problem":null,"fromDrone":true,"active":true,"status":"live"}],
         "kinds":[
           {"raw":"RTSP Video Stream","label":"RTSP Video Stream","group":"Video streams","needsUrl":true,"hint":"rtsp://192.168.1.10:8554/live"},
           {"raw":"Herelink Hotspot","label":"Herelink Hotspot","group":"Vehicle and radio presets","needsUrl":false,"hint":""},
           {"raw":"Back Camera","label":"Back Camera","group":"This device","needsUrl":false,"hint":""},
           {"raw":"Front Camera","label":"Front Camera","group":"This device","needsUrl":false,"hint":""}]}
        """)

    func testEveryCameraReadsAsOneListAndOnlyTheOperatorsOwnCanBeEdited() {
        let reading = camerasReading(view)!
        XCTAssertEqual(reading.cameras.count, 3)
        XCTAssertEqual(reading.stored.map(\.stored), [0, 1])
        XCTAssertNil(reading.cameras[2].stored)
        XCTAssertTrue(reading.cameras[2].fromDrone)
        XCTAssertEqual(reading.cameras[1].problem, "This kind of camera cannot show video in this app.")
        XCTAssertNil(reading.cameras[0].problem)
        XCTAssertEqual(reading.kinds.first?.hint, "rtsp://192.168.1.10:8554/live")
        XCTAssertNil(camerasReading(JSON.parse(#"{"class":"Video"}"#)))
    }

    func testEachCameraCarriesItsSignalShortNameAndThePictureInPicturePick() {
        let reading = camerasReading(view)!
        XCTAssertEqual(reading.cameras.map(\.status), [.Connecting, .Idle, .Live])
        XCTAssertEqual(reading.cameras.map(\.short), ["Front", "Cam 2", "SIYI"])
        XCTAssertEqual(reading.pip, CameraPip(enabled: false, slot: 0))
        XCTAssertEqual(camerasReading(JSON.parse(#"{"class":"Cameras","pip":{"enabled":true,"slot":null}}"#))!.pip, CameraPip(enabled: true, slot: nil))
        XCTAssertEqual(camerasReading(JSON.parse(#"{"class":"Cameras","cameras":[{"status":"noSignal"}]}"#))!.cameras.first?.status, .NoSignal)
    }

    func testARowNamesItsAddressOrThatTheDroneBroughtIt() {
        XCTAssertEqual(camerasReading(view)!.cameras.map(cameraDetail), ["rtsp://10.0.0.5:8554/front", "Back camera", "From the drone"])
    }

    func testTheAddSheetOffersThisPhonesCamerasFirstThenPresetsNeverOneAlreadyListed() {
        let reading = camerasReading(view)!
        XCTAssertEqual(otherSources(reading).map(\.raw), ["Front Camera", "Herelink Hotspot"])
        XCTAssertEqual(otherSources(reading).map(otherSourceLabel), ["This phone's front camera", "Herelink Hotspot"])
        XCTAssertEqual(otherSources(nil), [])
    }

    func testAnAddressIsClassifiedByTheCoreAndAnAmbiguousOneLetsTheOperatorPick() {
        let ambiguous = cameraGuess(JSON.parse(#"{"ok":true,"address":"0.0.0.0:5600","kind":"UDP h.264 Video Stream","choices":["UDP h.264 Video Stream","UDP h.265 Video Stream","MPEG-TS Video Stream","TCP-MPEG2 Video Stream"],"ambiguous":true,"problem":null}"#))!
        XCTAssertEqual(ambiguous.kind, "UDP h.264 Video Stream")
        XCTAssertEqual(ambiguous.choices.count, 4)
        XCTAssertEqual(guessText(ambiguous), "Which kind of stream is it?")
        XCTAssertEqual(chosenKind(ambiguous, nil, ""), "UDP h.264 Video Stream", "the default choice when nothing is picked")
        XCTAssertEqual(chosenKind(ambiguous, "UDP h.265 Video Stream", ""), "UDP h.265 Video Stream")
        XCTAssertEqual(chosenKind(ambiguous, "RTSP Video Stream", ""), "UDP h.264 Video Stream", "a pick the address does not allow falls back to the guess")

        let rtsp = cameraGuess(JSON.parse(#"{"ok":true,"address":"rtsp://cam/live","kind":"RTSP Video Stream","choices":["RTSP Video Stream"],"ambiguous":false,"problem":null}"#))!
        XCTAssertEqual(guessText(rtsp), "RTSP stream")

        let refused = cameraGuess(JSON.parse(#"{"ok":true,"address":"ftp://x","kind":null,"choices":[],"ambiguous":false,"problem":"Start the address with rtsp://, http://, udp:// or tcp://, or type it as host:port."}"#))!
        XCTAssertNil(refused.kind)
        XCTAssertEqual(guessText(refused), refused.problem)

        XCTAssertEqual(chosenKind(nil, nil, ""), "", "no guess yet leaves the core to infer")
        XCTAssertEqual(guessText(nil), "")
        XCTAssertNil(cameraGuess(JSON.parse(#"{"ok":false,"reason":"no"}"#)))
        XCTAssertNil(cameraGuess(nil))
    }

    func testRenamingAWorkingCameraKeepsItsKindEvenWhereItsAddressAloneReadsAsAnother() {
        let webrtc = "WebRTC (WHEP) Video Stream"
        let hostPort = cameraGuess(JSON.parse(#"{"ok":true,"address":"192.168.1.10:8889","kind":"UDP h.264 Video Stream","choices":["UDP h.264 Video Stream","UDP h.265 Video Stream","MPEG-TS Video Stream","TCP-MPEG2 Video Stream"],"ambiguous":true,"problem":null}"#))
        let kept = keptGuess(hostPort, webrtc, "192.168.1.10:8889", "192.168.1.10:8889")!
        XCTAssertEqual(kept, CameraGuess(kind: webrtc, choices: [webrtc], ambiguous: false, problem: nil))
        XCTAssertEqual(chosenKind(kept, webrtc, webrtc), webrtc)

        let unknown = cameraGuess(JSON.parse(#"{"ok":true,"address":"cam:8889/whep","kind":null,"choices":[],"ambiguous":false,"problem":"Start the address with rtsp://, http://, udp:// or tcp://, or type it as host:port."}"#))
        XCTAssertEqual(keptGuess(unknown, webrtc, "cam:8889/whep ", "cam:8889/whep")?.kind, webrtc, "an address the classifier cannot read still saves under its kind")

        XCTAssertEqual(keptGuess(hostPort, webrtc, "192.168.1.10:8890", "192.168.1.10:8889"), hostPort, "a changed address is guessed afresh")
        XCTAssertEqual(keptGuess(hostPort, webrtc, "192.168.1.10:8889", nil), hostPort, "a broken or new camera has nothing to keep")
        XCTAssertEqual(keptGuess(hostPort, "MPEG-TS Video Stream", "192.168.1.10:8889", "192.168.1.10:8889"), hostPort, "a kind among the choices stays a choice")
        XCTAssertNil(keptGuess(nil, webrtc, "192.168.1.10:8889", "192.168.1.10:8889"))
    }

    private let rtspGuess = CameraGuess(kind: "RTSP Video Stream", choices: ["RTSP Video Stream"], ambiguous: false, problem: nil)
    private let unreadable = CameraGuess(kind: nil, choices: [], ambiguous: false, problem: "Start the address with rtsp://, http://, udp:// or tcp://, or type it as host:port.")
    private let newDraft = CameraDraft(stored: nil, title: "", name: "Belly", source: "", url: "rtsp://cam/live")
    private let editDraft = CameraDraft(stored: 0, title: "Front gimbal", name: "Front gimbal", source: "RTSP Video Stream", url: "ftp://cam", picked: "RTSP Video Stream", kept: "rtsp://10.0.0.5:8554/front")

    func testSavingBeforeTheTypedAddressWasClassifiedClassifiesItOnTheSpot() {
        var asked: [String] = []
        let save = cameraSave(newDraft, nil) { address in
            asked.append(address)
            return rtspGuess
        }
        XCTAssertEqual(asked, ["rtsp://cam/live"])
        XCTAssertEqual(save, .Add(name: "Belly", source: "RTSP Video Stream", url: "rtsp://cam/live"))
    }

    func testAGuessAlreadyOnScreenIsSavedAsShownWithoutAskingAgain() {
        let save = cameraSave(newDraft, rtspGuess) { _ in
            XCTFail("classified twice")
            return nil
        }
        XCTAssertEqual(save, .Add(name: "Belly", source: "RTSP Video Stream", url: "rtsp://cam/live"))
    }

    func testAnEditToAnAddressTheCoreCannotReadIsRefusedBeforeAnythingIsWritten() {
        XCTAssertEqual(cameraSave(editDraft, unreadable) { _ in XCTFail("classified twice"); return nil }, .Refused(problem: unreadable.problem!))
        XCTAssertEqual(cameraSave(editDraft, nil) { _ in unreadable }, .Refused(problem: unreadable.problem!))
    }

    func testANewAddressTheClassifierCannotReadIsStillHandedToTheCoreWhichNamesTheProblem() {
        XCTAssertEqual(cameraSave(newDraft, unreadable) { _ in XCTFail("classified twice"); return nil }, .Add(name: "Belly", source: "", url: "rtsp://cam/live"))
    }

    func testAnEditKeepsTheCamerasKindForItsOldAddressAndACameraWithNoAddressIsRenamedWithoutClassifying() {
        var kept = editDraft
        kept.url = "rtsp://10.0.0.5:8554/front"
        XCTAssertEqual(cameraSave(kept, nil) { _ in unreadable }, .Update(slot: 0, name: "Front gimbal", source: "RTSP Video Stream", url: "rtsp://10.0.0.5:8554/front"))
        let phone = CameraDraft(stored: 1, title: "Camera 2", name: "Phone", source: "Back Camera", url: "", needsUrl: false)
        XCTAssertEqual(cameraSave(phone, nil) { _ in XCTFail("nothing to classify"); return nil }, .Update(slot: 1, name: "Phone", source: "Back Camera", url: ""))
    }

    func testARefusedSaveLandsInTheSheetAsItIsNowAndNeverReopensASheetTheOperatorClosed() {
        var typedWhileSaving = newDraft
        typedWhileSaving.name = "Belly cam"
        var refused = typedWhileSaving
        refused.refusal = "No"
        XCTAssertEqual(refusedDraft(typedWhileSaving, "No"), refused)
        XCTAssertNil(refusedDraft(nil, "No"), "a cancelled sheet stays closed")
        XCTAssertNil(refusedDraft(typedWhileSaving, nil), "a save that went through closes the sheet")
    }

    func testTheVideoSourcesPageIsListedUnderTransmissionThoughItHasNoSettingsOfItsOwn() {
        let pages = settingsPages(JSON.parse(#"{"pages":[{"title":"Video sources","showsVideoSources":true,"sections":[]}]}"#))
        XCTAssertEqual(pages.map(\.title), [VIDEO_SOURCES_PAGE])
        XCTAssertEqual(pageLook(VIDEO_SOURCES_PAGE).group, .Transmission)
    }

    func testTheSettingsIndexNamesTheCameraOnScreenAndHowManyThereAre() {
        XCTAssertEqual(camerasGlance(camerasReading(view)), "SIYI A8 · 3 cameras")
        XCTAssertEqual(camerasGlance(nil), "")
        XCTAssertEqual(kindLabel("RTSP Video Stream"), "RTSP")
    }
}
