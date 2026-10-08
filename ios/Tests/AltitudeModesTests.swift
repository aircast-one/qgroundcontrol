import XCTest
@testable import Aircast

private let SERVED = """
{"class": "AltitudeModes", "context": "mission", "current": 2, "holdsAltitudeAboveTerrain": true,
 "modes": [
  {"raw": 1, "title": "Relative To Launch", "help": "Above the launch position.", "enabled": true, "current": false, "reason": ""},
  {"raw": 2, "title": "AMSL", "help": "Above mean sea level.", "enabled": true, "current": true, "reason": ""},
  {"raw": 4, "title": "Terrain Frame", "help": "Above terrain, held by the vehicle in flight.", "enabled": false, "current": false,
   "reason": "This vehicle does not hold an altitude above terrain."},
  {"raw": 0, "title": "Mixed Modes", "help": "Each item sets its own.", "enabled": true, "current": false, "reason": ""}
 ],
 "omitted": []}
"""

private let EMPTY_PLAN = """
{"class": "AltitudeModes", "context": "mission", "current": 0, "holdsAltitudeAboveTerrain": true,
 "modes": [
  {"raw": 1, "title": "Relative To Launch", "help": "", "enabled": false, "current": false,
   "reason": "Add a mission item before choosing how its altitude is measured."},
  {"raw": 2, "title": "AMSL", "help": "", "enabled": false, "current": false,
   "reason": "Add a mission item before choosing how its altitude is measured."},
  {"raw": 4, "title": "Terrain Frame", "help": "", "enabled": false, "current": false,
   "reason": "Add a mission item before choosing how its altitude is measured."},
  {"raw": 0, "title": "Mixed Modes", "help": "", "enabled": true, "current": true, "reason": ""}
 ],
 "omitted": []}
"""

private let ONE_OPTION = """
{"class": "AltitudeModes", "context": "mission", "current": 2, "holdsAltitudeAboveTerrain": false,
 "modes": [
  {"raw": 1, "title": "Relative To Launch", "help": "", "enabled": false, "current": false,
   "reason": "This vehicle reports no launch position."},
  {"raw": 2, "title": "AMSL", "help": "Above mean sea level.", "enabled": true, "current": true, "reason": ""},
  {"raw": 4, "title": "Terrain Frame", "help": "", "enabled": false, "current": false,
   "reason": "This vehicle does not hold an altitude above terrain."},
  {"raw": 0, "title": "Mixed Modes", "help": "", "enabled": true, "current": false, "reason": ""}
 ],
 "omitted": []}
"""

final class AltitudeModesTests: XCTestCase {
    func testOnePickableModeIsTheOneAlreadySetSoTheControlChangesNothing() {
        XCTAssertEqual(false, offersChoice(altitudeModesView(JSON.parse(ONE_OPTION))))
        XCTAssertEqual(1, choosable(altitudeModesView(JSON.parse(ONE_OPTION))).filter(\.enabled).count)
    }

    func testAPickerHoldingOneOptionPromisesAChoiceItCannotDeliver() {
        XCTAssertEqual(false, offersChoice(altitudeModesView(JSON.parse(EMPTY_PLAN))))
    }

    func testAPlanWithRealAlternativesKeepsThePickerLive() {
        XCTAssertTrue(offersChoice(altitudeModesView(JSON.parse(SERVED))))
    }

    func testTheRefusedModesAreStillCountedByTheCoresJudgementNeverByRederivingWhy() {
        let view = altitudeModesView(JSON.parse(EMPTY_PLAN))
        XCTAssertEqual(3, choosable(view).count)
        XCTAssertEqual("Add a mission item before choosing how its altitude is measured.", refusalFor(view, 1))
    }

    func testMixedModesIsAStateThePlanCanBeInNeverAModeToPick() {
        let picks = choosable(altitudeModesView(JSON.parse(SERVED)))
        XCTAssertEqual([1, 2, 4], picks.map(\.raw))
        XCTAssertTrue(!picks.contains { $0.raw == ALT_MODE_MIXED }, "offering Mixed would ask the operator to set every item at once")
    }

    func testAModeTheVehicleCannotFlyCarriesTheCoresReasonRatherThanVanishing() {
        XCTAssertEqual("This vehicle does not hold an altitude above terrain.", refusalFor(altitudeModesView(JSON.parse(SERVED)), 4))
        XCTAssertNil(refusalFor(altitudeModesView(JSON.parse(SERVED)), 2), "a mode that is offered has nothing to explain")
    }

    func testThePathCarriesTheContextAndTheCurrentModeWhichIsWhatTheCoreKeysOff() {
        XCTAssertEqual("view.altitudeModes(mission,2)", altitudeModesPath(MISSION_CONTEXT, 2))
        XCTAssertEqual("view.altitudeModes(item,3)", altitudeModesPath(ITEM_CONTEXT, 3))
        XCTAssertEqual("plan.missionController.visualItems.3.altitudeMode", altitudeModePath(3))
    }

    func testAnythingThatIsNotTheAltitudeModesViewIsRefused() {
        XCTAssertNil(altitudeModesView(JSON.parse(#"{"class": "Radio"}"#)))
        XCTAssertNil(altitudeModesView(nil))
    }
}
