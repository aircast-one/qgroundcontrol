import XCTest
@testable import Aircast

final class FlyDeckTests: XCTestCase {
    private func ids(_ deck: [(String, Bool)]) -> [String] { deck.map(\.0) }
    private func primaries(_ deck: [(String, Bool)]) -> [Bool] { deck.map(\.1) }

    func testGroundOffersChecklistArmAndTakeoffWithTakeoffPrimary() {
        let deck = deckIds([CHECKLIST, "arm", "takeoff", "changeSpeed"], false)
        XCTAssertEqual(ids(deck), [CHECKLIST, "arm", "takeoff"])
        XCTAssertEqual(primaries(deck), [false, false, true])
    }

    func testGroundWithoutTakeoffArmsAsPrimary() {
        let deck = deckIds(["arm"], false)
        XCTAssertEqual(ids(deck), ["arm"])
        XCTAssertEqual(primaries(deck), [true])
    }

    func testFlyingOffersPauseReturnLandWithReturnPrimary() {
        let deck = deckIds(["arm", PAUSE, "rtl", "land", "changeAltitude"], true)
        XCTAssertEqual(ids(deck), [PAUSE, "rtl", "land"])
        XCTAssertEqual(primaries(deck), [false, true, false])
    }

    func testAGripperAddsDropToTheFlyingDeckInAnyModeButNotOnTheGround() {
        XCTAssertEqual(ids(deckIds(["arm", PAUSE, "rtl", "land", RELEASE], true)), [PAUSE, "rtl", "land", RELEASE])
        XCTAssertEqual(ids(deckIds(["arm", "takeoff", RELEASE], false)), ["arm", "takeoff"])
    }

    func testArmedOnTheGroundKeepsTheGroundDeckWithDisarmBesideTakeoff() {
        let deck = deckIds(["arm", "takeoff"], true)
        XCTAssertEqual(ids(deck), ["arm", "takeoff"])
        XCTAssertEqual(primaries(deck), [false, true])
    }

    private let flying = deckIds(["arm", PAUSE, "rtl", "land", RELEASE], true)

    func testOnAPhoneEveryFlightActionStaysOnTheDeckAndTheCameraMovesAboveIt() {
        let fits = [358, 328].map { bottomDeck(flying, $0, 112) }
        XCTAssertEqual(fits.map { ids($0.deck) }, [[PAUSE, "rtl", "land", RELEASE], [PAUSE, "rtl", "land", RELEASE]])
        XCTAssertEqual(fits.map(\.centreInline), [false, false])
    }

    func testADeckWithRoomKeepsTheCameraBetweenTheActions() {
        XCTAssertTrue(bottomDeck(flying, 700, 112).centreInline)
        XCTAssertTrue(bottomDeck(flying, 328, 0).centreInline)
        XCTAssertEqual(ids(bottomDeck(flying, 700, 112).deck), ids(flying))
    }

    func testWhenTheActionsRunOutOfRoomTheLeastCriticalMoveIntoMoreFirst() {
        XCTAssertEqual([288, 222, 156, 90, 40].map { ids(bottomDeck(flying, $0, 112).deck) }, [["rtl", "land", RELEASE], ["rtl", "land"], ["rtl"], [], []])
    }

    func testTheGroundDeckGivesUpTheChecklistThenArmBeforeTakingOff() {
        let ground = deckIds([CHECKLIST, "arm", "takeoff"], false)
        XCTAssertEqual([270, 200, 140].map { ids(bottomDeck(ground, $0, 0).deck) }, [[CHECKLIST, "arm", "takeoff"], ["arm", "takeoff"], ["takeoff"]])
    }

    func testADeckNotMeasuredYetShowsEveryAction() {
        XCTAssertEqual(ids(bottomDeck(flying, 0, 112).deck), ids(flying))
        XCTAssertTrue(bottomDeck(flying, 0, 112).centreInline)
        XCTAssertEqual(ids(railDeck(flying, 0).deck), ids(flying))
        XCTAssertTrue(railDeck(flying, 0).labelled)
    }

    func testASidewaysPhoneDropsTheRailLabelsBeforeItDropsAnAction() {
        let noDrop = flying.filter { $0.0 != RELEASE }
        XCTAssertEqual(ids(railDeck(flying, 286).deck), ids(flying))
        XCTAssertFalse(railDeck(flying, 286).labelled)
        XCTAssertTrue(railDeck(noDrop, 286).labelled)
        XCTAssertEqual(ids(railDeck(flying, 271).deck), ["rtl", "land", RELEASE])
    }

    func testACrowdedDeckDropsTheSlideInstructionSoFiveButtonsFitAPhone() {
        XCTAssertEqual([deckLabel("Slide to land", 4), deckLabel("Slide to land", 5), deckLabel("Slide to drop 2", 5)], ["Slide to land", "Land", "Drop 2"])
    }

    func testTheRailLabelsDropTheHoldInstruction() {
        XCTAssertEqual(["Slide to land", "Return", "Slide to take off"].map(railLabel), ["Land", "Return", "Take off"])
    }

    func testTheFlyMenuGroupsTheAircraftsTilesAboveTheScreensAndAGroupWithNothingInItIsNotShown() {
        let tile = { (label: String) in MoreTile(label: label, icon: .tune, enabled: true, onClick: {}) }
        let both = shownSections(flyMenuSections([tile("All status"), tile(CALIBRATE)], [tile("Choose readings")]))
        XCTAssertEqual(both.map(\.title), [AIRCRAFT_SECTION, SCREEN_SECTION])
        XCTAssertEqual(both.map { $0.tiles.map(\.label) }, [["All status", CALIBRATE], ["Choose readings"]])
        XCTAssertEqual(shownSections(flyMenuSections([], [tile("Choose readings")])).map(\.title), [SCREEN_SECTION])
    }

    func testCalibrateIsOfferedOnlyToAnAircraftWithASensorsPageOpensItAndIsNotPressableWhileArmed() {
        final class Opened { var times = 0 }
        let opened = Opened()
        let onGround = calibrateTile(sensorsPage: true, armed: false, open: { opened.times += 1 })
        XCTAssertEqual(onGround?.label, CALIBRATE)
        XCTAssertEqual(onGround?.enabled, true)
        onGround?.onClick()
        XCTAssertEqual(opened.times, 1)
        XCTAssertEqual(calibrateTile(sensorsPage: true, armed: true, open: {})?.enabled, false)
        XCTAssertNil(calibrateTile(sensorsPage: false, armed: false, open: {}))
        let setup = JSON.parse(#"{"components":[{"name":"Sensors"}]}"#)
        XCTAssertEqual([SENSORS, "Power"].map { hasSetupPage(setup, $0) }, [true, false])
    }

    func testSlidingAcrossMostOfTheButtonConfirmsAsSurelyAsHoldingIt() {
        XCTAssertEqual(slideProgress(300, 1000), 0.5, accuracy: 1e-9)
        XCTAssertEqual(slideProgress(600, 1000), 1, accuracy: 1e-9)
        XCTAssertEqual(slideProgress(900, 1000), 1, accuracy: 1e-9)
        XCTAssertEqual(slideProgress(-200, 1000), 0, accuracy: 1e-9, "sliding back the other way never confirms")
    }
}
