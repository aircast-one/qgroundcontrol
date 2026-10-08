import XCTest
@testable import Aircast

private func mission(_ index: Int, _ sequence: Int, _ latitude: Double, _ longitude: Double, _ command: String, _ altitude: Double, _ change: (inout MissionItem) -> Void = { _ in }) -> MissionItem {
    var item = MissionItem(index: index, sequence: sequence, latitude: latitude, longitude: longitude, command: command, selected: false, altitude: altitude)
    change(&item)
    return item
}

final class PlanItemsTests: XCTestCase {
    private func item(_ index: Int, _ sequence: Int, _ name: String, altitude: Double = 50.0, kind: String = "waypoint", commandId: Int = 16) -> MissionItem {
        mission(index, sequence, 41.0, 44.0, name, altitude) {
            $0.kind = kind
            $0.commandId = commandId
            $0.altitudeText = altitude.isNaN ? "" : "\(Int(altitude)) m"
        }
    }

    func testASurveysAreaAndPhotosMoveFromTheSheetLineIntoTiles() {
        var survey = item(4, 4, "Survey")
        survey.cameraShots = 1043
        let stats = SurveyStats(areaText: "90049 m\u{00b2}", warning: "", intervalText: "2.0 s", distanceText: "4123 m")
        let tiles = surveyTiles(survey, stats)
        XCTAssertEqual(["AREA", "DISTANCE", "PHOTOS", "INTERVAL"], tiles.map(\.0))
        XCTAssertEqual(["90049 m\u{00b2}", "4123 m", "1043", "2.0 s"], tiles.map(\.1))
        XCTAssertEqual("50 m", sheetDetail(survey, stats))
        XCTAssertTrue(surveyTiles(survey, nil).isEmpty)
        var structure = item(5, 5, "Structure Scan", kind: KIND_STRUCTURE)
        structure.cameraShots = 40
        var noArea = stats
        noArea.areaText = ""
        XCTAssertEqual(["PHOTOS", "INTERVAL"], surveyTiles(structure, noArea).map(\.0), "StructureScanEditor's stats have no Distance row")
        XCTAssertEqual("50 m \u{00b7} 1043 photos", sheetDetail(survey, nil))
    }

    func testTheDeleteButtonNamesTheKindOfItemItRemoves() {
        XCTAssertEqual("Delete waypoint", deleteLabel(item(2, 2, "Waypoint")))
        XCTAssertEqual("Delete item", deleteLabel(item(2, 2, "")))
    }

    func testTheSheetPlacesAnItemAmongTheListedOnesAndNamesTheLegFromTheOneBefore() {
        let home = item(0, 0, "Home")
        let takeoff = item(1, 1, "Takeoff")
        var second = item(2, 2, "Waypoint")
        second.distance = 130.0
        second.distanceText = "130 m"
        let items = [home, takeoff, second]
        XCTAssertEqual("Item 2 of 2 \u{00b7} 130 m from item 1", itemPlace(second, items))
        XCTAssertEqual("Item 1 of 2", itemPlace(takeoff, items))
        XCTAssertNil(itemPlace(home, items))
    }

    func testARowCarriesTheNumberTheNameTheAltitudeAndTheMarkerColour() {
        let rows = itemRows([item(1, 1, "Waypoint", altitude: 49.6)])
        XCTAssertEqual(["1"], rows.map(\.number))
        XCTAssertEqual(["Waypoint"], rows.map(\.name))
        XCTAssertEqual(["49 m"], rows.map(\.detail))
        XCTAssertEqual([WAYPOINT_COLOUR], rows.map(\.colour))
    }

    func testTheListIsColouredTheSameWayTheMapIsSoARowAndItsMarkerMatch() {
        let rows = itemRows([
            item(0, 0, "Mission Start", kind: "settings"),
            item(1, 1, "Takeoff", kind: "takeoff", commandId: 22),
            item(2, 2, "Return To Launch", kind: "command", commandId: MAV_CMD_NAV_RETURN_TO_LAUNCH),
        ])
        XCTAssertEqual([START_COLOUR, TAKEOFF_COLOUR, RETURN_COLOUR], rows.map(\.colour))
    }

    func testAPatternSpanningHeightsShowsTheBandHavingNoSingleAltitude() {
        let survey = mission(4, 4, 41.0, 44.0, "Survey", .nan) {
            $0.kind = "survey"
            $0.altitudeBandText = "585 m to 660 m"
        }
        XCTAssertEqual("585 m to 660 m", itemDetail(survey))
    }

    func testAnItemWithItsOwnAltitudeIgnoresAnyBand() {
        let waypoint = mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.altitudeText = "50.0 m"
            $0.altitudeBandText = "nonsense"
        }
        XCTAssertEqual("50.0 m", itemDetail(waypoint))
    }

    func testAnItemWithNoAltitudeSaysNothingRatherThanNaN() {
        XCTAssertEqual([""], itemRows([item(1, 1, "Waypoint", altitude: .nan)]).map(\.detail))
    }

    func testAnItemTheVehicleDidNotNameIsStillReachableByItsNumber() {
        XCTAssertEqual(["Item 3"], itemRows([item(3, 3, "")]).map(\.name))
    }

    func testACommandWithNoPlaceByDesignIsListedAndSaysNothingAboutPosition() {
        let rows = itemRows([mission(1, 1, .nan, .nan, "Change Speed", .nan) {
            $0.kind = "command"
            $0.commandId = 178
            $0.placed = false
            $0.specifiesCoordinate = false
        }])
        XCTAssertEqual([""], rows.map(\.detail))
        XCTAssertEqual([false], rows.map(\.placed))
    }

    func testAnItemThatIsSupposedToHaveAPlaceAndHasNoneStillSaysSo() {
        let rows = itemRows([mission(1, 1, .nan, .nan, "Waypoint", .nan) {
            $0.kind = "waypoint"
            $0.commandId = 16
            $0.placed = false
            $0.specifiesCoordinate = true
        }])
        XCTAssertEqual([NO_POSITION], rows.map(\.detail))
    }

    func testAnArduPilotTakeoffSpecifiesAnAltitudeAndNoPlaceSoTheAltitudeIsWhatItSays() {
        let takeoff = mission(1, 1, .nan, .nan, "Takeoff", 50.0) {
            $0.kind = KIND_TAKEOFF
            $0.commandId = 22
            $0.placed = false
            $0.altitudeText = "50 m"
        }
        XCTAssertEqual(["50 m"], itemRows([takeoff]).map(\.detail))
    }

    func testTheAltitudeIsWhateverTheCoreSpelledBecauseTheOperatorMayNotBeInMetres() {
        let feet = mission(1, 1, 41.0, 44.0, "Waypoint", 75.0) {
            $0.kind = "waypoint"
            $0.commandId = 16
            $0.altitudeText = "246 ft"
        }
        XCTAssertEqual(["246 ft"], itemRows([feet]).map(\.detail))
    }

    func testAnItemSittingAfterTheLandSaysSoBecauseTheMapJustDrawsNoLineToIt() {
        let stranded = mission(3, 3, 41.0, 44.0, "Waypoint", 50.0) {
            $0.routed = false
            $0.kind = "waypoint"
            $0.commandId = 16
            $0.afterRouteEnds = true
            $0.altitudeText = "50 m"
        }
        XCTAssertEqual(["50 m \u{00b7} after the route ends"], itemRows([stranded]).map(\.detail))
    }

    func testAnItemTheRouteDoesNotPassThroughIsNotAccusedOfBeingStranded() {
        let roi = mission(2, 2, 41.0, 44.0, "Region Of Interest", .nan) {
            $0.routed = false
            $0.kind = "roi"
            $0.commandId = 201
        }
        XCTAssertEqual([""], itemRows([roi]).map(\.detail))
    }

    func testARowIsFoundByTheIndexTheMapSelectsWithNotByPosition() {
        let rows = itemRows([item(4, 1, "Waypoint"), item(9, 2, "Land", kind: "land", commandId: 21)])
        XCTAssertEqual("Land", rowAt(rows, 9)?.name)
        XCTAssertNil(rowAt(rows, 1))
    }
}

final class UnplacedSelectionTests: XCTestCase {
    private let unplaced = mission(1, 1, .nan, .nan, "Takeoff", .nan) {
        $0.kind = "takeoff"
        $0.commandId = 22
        $0.placed = false
    }

    func testSelectingAnItemTheMapCannotDrawIsNotThrownAwayOnTheNextRead() {
        XCTAssertTrue(selectionSurvives(.Waypoint(index: 1), [unplaced], [], [], [], []))
    }

    func testASelectionOfAnItemThatLeftThePlanIsStillThrownAway() {
        XCTAssertFalse(selectionSurvives(.Waypoint(index: 1), [], [], [], [], []))
    }
}

final class WorthListingTests: XCTestCase {
    private func item(_ index: Int) -> MissionItem { mission(index, index, 41.0, 44.0, "Waypoint", 50.0) { $0.kind = "waypoint" } }

    func testAPlanHoldingOnlyItsSettingsItemHasNothingToList() {
        XCTAssertFalse(worthListing([item(HOME_ITEM)]))
    }

    func testOneRealItemIsEnoughToBeWorthListing() {
        XCTAssertTrue(worthListing([item(HOME_ITEM), item(1)]))
    }

    func testNoPlanAtAllIsNothingToList() {
        XCTAssertFalse(worthListing([]))
    }
}

final class InsertAfterTests: XCTestCase {
    private let plan = (0..<3).map { index in mission(index, index, 41.0, 44.0, "Waypoint", 50.0) { $0.kind = "waypoint" } }

    func testWithNothingSelectedAnItemGoesOnTheEnd() {
        XCTAssertEqual(AT_END, insertAfter(nil, plan))
    }

    func testANewItemFollowsTheOneTheOperatorSelectedAsItDoesInQgc() {
        XCTAssertEqual(2, insertAfter(.Waypoint(index: 1), plan))
    }

    func testSelectingTheLastItemStillMeansTheEndNotAPlacePastIt() {
        XCTAssertEqual(AT_END, insertAfter(.Waypoint(index: 2), plan))
    }

    func testASelectionThePlanNoLongerHoldsDoesNotNameAPosition() {
        XCTAssertEqual(AT_END, insertAfter(.Waypoint(index: 9), plan))
    }

    func testASelectedPatternCornerInsertsAfterItsPatternAndANonMissionSelectionAppends() {
        XCTAssertEqual(2, insertAfter(.SurveyVertex(item: 1, vertex: 0), plan))
        XCTAssertEqual(AT_END, insertAfter(.Rally(index: 1), plan))
    }
}

final class SelectionSequenceTests: XCTestCase {
    private let plan = [
        mission(0, 0, 41.0, 44.0, "Mission Start", 0.0) { $0.kind = "settings" },
        mission(1, 1, 41.0, 44.0, "Takeoff", 50.0) { $0.kind = KIND_TAKEOFF },
        mission(2, 4, 41.0, 44.0, "Waypoint", 50.0) { $0.kind = "waypoint" },
    ]

    func testTheSequenceIsSentNotTheIndexBecauseTheyAreNotTheSameNumber() {
        XCTAssertEqual(4, selectionSequence(.Waypoint(index: 2), plan))
    }

    func testNothingSelectedSelectsNothing() {
        XCTAssertNil(selectionSequence(nil, plan))
    }

    func testASelectionThePlanNoLongerHoldsSendsNothing() {
        XCTAssertNil(selectionSequence(.Waypoint(index: 9), plan))
    }

    func testAFenceVertexIsNotAMissionItemAndDoesNotMoveThePlanView() {
        XCTAssertNil(selectionSequence(.FenceVertex(polygon: 0, vertex: 1), plan))
    }
}

final class AddingAfterTextTests: XCTestCase {
    private let plan = [
        mission(0, 0, 41.0, 44.0, "Mission Start", 0.0) { $0.kind = "settings" },
        mission(2, 5, 41.0, 44.0, "Waypoint", 50.0) { $0.kind = "waypoint" },
    ]

    func testTheLineNamesTheSequenceTheOperatorSeesOnTheMarker() {
        XCTAssertEqual("Adding after #5", addingAfterText(.Waypoint(index: 2), plan))
    }

    func testWithNothingSelectedTheInsertionPointIsTheEndAndNeedsNoLine() {
        XCTAssertNil(addingAfterText(nil, plan))
    }

    func testAFenceVertexDoesNotMoveTheInsertionPointSoItSaysNothing() {
        XCTAssertNil(addingAfterText(.FenceVertex(polygon: 0, vertex: 1), plan))
    }

    func testASelectionThePlanNoLongerHoldsSaysNothingRatherThanAStaleNumber() {
        XCTAssertNil(addingAfterText(.Waypoint(index: 9), plan))
    }
}

final class LegTextTests: XCTestCase {
    private func item(distanceText: String = "449.4 m", azimuthText: String = "47", altitudeChangeText: String = "12.0 m", headingText: String = "", gradientText: String = "2 deg") -> MissionItem {
        mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.commandId = 16
            $0.distanceText = distanceText
            $0.azimuthText = azimuthText
            $0.altitudeChangeText = altitudeChangeText
            $0.headingText = headingText
            $0.gradientText = gradientText
        }
    }

    func testEachFigureCarriesTheLabelAndOrderPlanToolBarIndicatorsGivesIt() {
        XCTAssertEqual("Alt diff 12.0 m \u{00b7} Azimuth 47 \u{00b7} Heading 90 \u{00b7} Gradient 2 deg \u{00b7} Prev WP 449.4 m", legText(item(headingText: "90")))
    }

    func testZerosShowLikeQgcWhichHidesAStatOnlyWhenItHasNoNumber() {
        XCTAssertEqual("Alt diff 0.0 m \u{00b7} Azimuth 0 \u{00b7} Prev WP 0.0 m", legText(item(distanceText: "0.0 m", azimuthText: "0", altitudeChangeText: "0.0 m", gradientText: "")))
    }

    func testAnItemTheControllerHasNotMeasuredSaysNothing() {
        XCTAssertNil(legText(item(distanceText: "", azimuthText: "", altitudeChangeText: "", gradientText: "")))
    }
}

final class BlockedReasonTests: XCTestCase {
    func testAnItemThatBlocksTheSaveSaysWhyWhereTheOperatorIsLooking() {
        let item = mission(4, 4, 41.0, 44.0, "Landing pattern", .nan) {
            $0.kind = "land"
            $0.blockedReason = "Landing point not set"
        }
        XCTAssertEqual("Landing point not set", itemDetail(item))
    }

    func testAReasonJoinsWhateverElseTheRowAlreadySays() {
        let item = mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.altitudeText = "50.0 m"
            $0.blockedReason = "Needs a value"
        }
        XCTAssertEqual("50.0 m · Needs a value", itemDetail(item))
    }

    func testAnItemWithNothingWrongSaysNothingExtra() {
        let item = mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.altitudeText = "50.0 m"
        }
        XCTAssertEqual("50.0 m", itemDetail(item))
    }
}

final class PhotosTextTests: XCTestCase {
    func testAPatternThatTakesPhotosSaysHowMany() {
        let survey = mission(5, 5, 41.0, 44.0, "Survey", .nan) {
            $0.kind = "survey"
            $0.altitudeBandText = "0.0 m to 40.0 m"
            $0.cameraShots = 340
        }
        XCTAssertEqual("0.0 m to 40.0 m · 340 photos", itemDetail(survey))
    }

    func testOnePhotoIsNotOnePhotos() {
        XCTAssertEqual("1 photo", photosText(1))
        XCTAssertEqual("2 photos", photosText(2))
    }

    func testAnItemThatTakesNoneSaysNothingAboutPhotos() {
        XCTAssertNil(photosText(0))
        XCTAssertNil(photosText(-1))
        let waypoint = mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.altitudeText = "50.0 m"
        }
        XCTAssertEqual("50.0 m", itemDetail(waypoint))
    }
}

final class SpeedChangeRowTests: XCTestCase {
    func testAnItemThatCommandsASpeedSaysWhichInTheOperatorsUnit() {
        let change = mission(3, 3, 41.0, 44.0, "Change speed", .nan) {
            $0.kind = "command"
            $0.speedChangeText = "12.0 m/s"
        }
        XCTAssertEqual("12.0 m/s", itemDetail(change))
    }

    func testACommandedSpeedReadsAlongsideTheAltitudeRatherThanInsteadOfIt() {
        let waypoint = mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.altitudeText = "50.0 m"
            $0.speedChangeText = "8.0 m/s"
        }
        XCTAssertEqual("50.0 m \u{00b7} 8.0 m/s", itemDetail(waypoint))
    }

    func testAnItemThatBothHoldsAndChangesSpeedSaysBothBecauseNeitherRanks() {
        let both = mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.altitudeText = "50.0 m"
            $0.speedChangeText = "8.0 m/s"
            $0.extraSeconds = 15.0
        }
        XCTAssertEqual("50.0 m \u{00b7} 8.0 m/s \u{00b7} holds 15 s", itemDetail(both))
    }

    func testAnItemTheCoreWithheldASpeedForSaysNothingAboutSpeed() {
        XCTAssertEqual("", itemDetail(mission(3, 3, 41.0, 44.0, "Change speed", .nan) { $0.kind = "command" }))
    }
}

final class HoldTextTests: XCTestCase {
    func testAnItemThatWaitsSaysHowLong() {
        let waypoint = mission(2, 2, 41.0, 44.0, "Waypoint", 50.0) {
            $0.kind = "waypoint"
            $0.altitudeText = "50.0 m"
            $0.extraSeconds = 30.0
        }
        XCTAssertEqual("50.0 m · holds 30 s", itemDetail(waypoint))
    }

    func testAnItemThatDoesNotWaitSaysNothingAboutWaiting() {
        XCTAssertNil(holdText(0.0))
        XCTAssertNil(holdText(.nan))
        XCTAssertNil(holdText(-1.0))
    }
}

final class SurveyStatsRowTests: XCTestCase {
    private let survey = mission(5, 5, 41.0, 44.0, "Survey", .nan) {
        $0.kind = "survey"
        $0.altitudeBandText = "0.0 m to 40.0 m"
        $0.cameraShots = 171
    }

    func testASurveyCarriesItsAreaBesideItsPhotoCount() {
        XCTAssertEqual("0.0 m to 40.0 m · 0.03 km² · 171 photos", itemDetail(survey, SurveyStats(areaText: "0.03 km²", warning: "")))
    }

    func testACameraThatCannotKeepUpSaysSoLastWhereItIsRead() {
        let stats = SurveyStats(areaText: "0.03 km²", warning: "The camera needs 2.00 s between shots but the survey asks for 1.50 s.")
        XCTAssertTrue(itemDetail(survey, stats).hasSuffix("The camera needs 2.00 s between shots but the survey asks for 1.50 s."))
    }

    func testAnItemWithNoSurveyStatsIsUnchanged() {
        XCTAssertEqual("0.0 m to 40.0 m · 171 photos", itemDetail(survey, nil))
    }

    func testNothingToSayIsNullRatherThanALineSayingNothing() {
        XCTAssertNil(photosText(0), "a waypoint taking no photographs must not draw a photo line")
        XCTAssertNil(photosText(-1))
        XCTAssertNil(holdText(0.0), "an item that does not hold must not draw a hold line")
        XCTAssertNil(holdText(.nan))
        XCTAssertNil(holdText(-3.0))
    }

    func testDraggingTheBreachReturnPointSaysSoAndSurvivesOnlyWhileThePointExists() {
        XCTAssertEqual("Moved the breach return point", movedText(.BreachReturn, []))
        XCTAssertEqual(true, selectionSurvives(.BreachReturn, [], [], [], [], [], breach: true))
        XCTAssertEqual(false, selectionSurvives(.BreachReturn, [], [], [], [], []))
    }

    func testDeletingKeepsTheSelectionAtTheSameIndexOrTheNewLastItemAsRemoveVisualItemDoes() {
        XCTAssertEqual(MapHit.Waypoint(index: 2), selectionAfterRemove(2, 5))
        XCTAssertEqual(MapHit.Waypoint(index: 3), selectionAfterRemove(4, 5))
        XCTAssertNil(selectionAfterRemove(1, 2))
    }
}
