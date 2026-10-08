import XCTest
@testable import Aircast

final class SaveBlockedTests: XCTestCase {
    private func view(_ body: String) -> JSON { JSON.parse(#"{"kind":"object","readiness":\#(body)}"#) }

    func testAPlanTheCoreCallsReadyIsNotBlocked() {
        XCTAssertNil(saveBlockedReason(view(#"{"state":0,"ready":true,"reason":""}"#)))
    }

    func testTheCoresOwnSentenceIsShownNotThisHeadsParaphraseOfTheSameState() {
        XCTAssertEqual(
            "Waiting for terrain heights before the plan can be saved or sent.",
            saveBlockedReason(view(#"{"state":1,"ready":false,"reason":"Waiting for terrain heights before the plan can be saved or sent."}"#))
        )
        XCTAssertEqual(
            "An item is still being drawn, so the plan cannot be saved or sent.",
            saveBlockedReason(view(#"{"state":2,"ready":false,"reason":"An item is still being drawn, so the plan cannot be saved or sent."}"#))
        )
    }

    func testNoAnswerBlocksTheSaveRatherThanLettingItThroughUnchecked() {
        XCTAssertEqual("The plan could not be checked for saving.", saveBlockedReason(nil))
        XCTAssertEqual("The plan could not be checked for saving.", saveBlockedReason(JSON.parse(#"{"kind":"null"}"#)))
    }

    func testAStateTheCoreHasNoSentenceForStillRefusesRatherThanPassingSilently() {
        XCTAssertEqual("The plan could not be checked for saving.", saveBlockedReason(view(#"{"state":9,"ready":false,"reason":""}"#)))
    }
}

final class PlanStatusTests: XCTestCase {
    private func view(_ status: String) -> JSON { .object(["kind": .string("object"), "status": .string(status)]) }

    func testTheCoreSpellsTheStatusIncludingWhetherChangesAreUnsavedOrUnsent() {
        XCTAssertEqual("ridge.plan \u{00b7} not uploaded", planStatusText(view("ridge.plan \u{00b7} not uploaded")))
        XCTAssertEqual("ridge.plan \u{00b7} unsaved changes", planStatusText(view("ridge.plan \u{00b7} unsaved changes")))
    }

    func testNoAnswerShowsNothingSoASilentCoreIsVisibleRatherThanPaperedOver() {
        XCTAssertEqual("", planStatusText(nil))
        XCTAssertEqual("", planStatusText(JSON.parse(#"{"kind":"null"}"#)))
    }
}

final class PlanActionsTests: XCTestCase {
    private func actions(open: Bool = true, save: Bool = true, exportKml: Bool = true, newPlan: Bool = true, clearMission: Bool = true) -> PlanActions {
        planActions(JSON.parse(#"{"kind":"object","actions":{"open":\#(open),"save":\#(save),"exportKml":\#(exportKml),"newPlan":\#(newPlan),"clearMission":\#(clearMission)}}"#))
    }

    func testEachActionIsTheCoresAnswerRatherThanThisHeadsArithmetic() {
        XCTAssertFalse(actions(save: false).save)
        XCTAssertTrue(actions(save: false).open)
        XCTAssertFalse(actions(exportKml: false).exportKml)
    }

    func testClearMissionIsTheCoresNameForClearingTheVehicleWhichIsWhatThisButtonDoes() {
        XCTAssertFalse(actions(clearMission: false).clearFromVehicle)
        XCTAssertTrue(actions(clearMission: true).clearFromVehicle)
    }

    func testDownloadFromVehicleFollowsTheCoreAndConfirmsOverUnsavedChanges() {
        XCTAssertTrue(planActions(JSON.parse(#"{"actions":{"download":true}}"#)).download)
        XCTAssertFalse(planActions(JSON.parse(#"{"actions":{"download":false}}"#)).download)
        XCTAssertEqual("Plan overwrite", confirmCopy(.Download).title)
    }

    func testOpeningOverChangesAndCreatingOverItemsAskInPlanViewsWords() {
        XCTAssertEqual("Plan overwrite", confirmCopy(.Open).title)
        XCTAssertEqual("You have unsaved/unsent changes. Loading from a file will lose these changes. Are you sure you want to load from a file?", confirmCopy(.Open).body)
        XCTAssertEqual("Create Plan", confirmCopy(.NewPlan).title)
    }

    func testNoAnswerOffersNothingRatherThanGuessingWhatIsAllowed() {
        let none = planActions(nil)
        XCTAssertFalse(none.open)
        XCTAssertFalse(none.save)
        XCTAssertFalse(none.newPlan)
    }
}

final class PlanHistoryTests: XCTestCase {
    func testAPlanWithNothingBehindItOffersNeither() {
        XCTAssertEqual(PlanHistory(canUndo: false, canRedo: false), planHistory(JSON.parse(#"{"kind":"object","canUndo":false,"canRedo":false}"#)))
    }

    func testAnEditedPlanCanBeUndoneButNotYetRedone() {
        XCTAssertEqual(PlanHistory(canUndo: true, canRedo: false), planHistory(JSON.parse(#"{"kind":"object","canUndo":true,"canRedo":false}"#)))
    }

    func testAViewThatNeverAnsweredOffersNothingRatherThanEnabledButtonsThatRefuse() {
        XCTAssertEqual(PlanHistory(canUndo: false, canRedo: false), planHistory(nil))
        XCTAssertEqual(PlanHistory(canUndo: false, canRedo: false), planHistory(JSON.parse("{}")))
    }

    func testTheTitleIsTheFilesBaseNameOrUntitledPlanLikePlanToolBarIndicators() {
        XCTAssertEqual("ridge", planTitle("ridge.plan"))
        XCTAssertEqual("a.b", planTitle("a.b.plan"))
        XCTAssertEqual("ridge", planTitle("ridge"))
        XCTAssertEqual(".plan", planTitle(".plan"))
        XCTAssertEqual("Untitled Plan", planTitle(nil))
        XCTAssertEqual("Untitled Plan", planTitle(""))
    }
}
