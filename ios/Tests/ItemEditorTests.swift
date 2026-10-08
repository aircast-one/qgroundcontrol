import XCTest
@testable import Aircast

final class ItemEditorTests: XCTestCase {
    func testStructureScanNoteComesFromTheCoreAndBlankMeansNoneLikeStructureScanEditor() {
        XCTAssertEqual("The polygon outlines the structure's surface, not the flight path.", gridNote(JSON.parse(#"{"gridNote":"The polygon outlines the structure's surface, not the flight path."}"#)))
        XCTAssertNil(gridNote(JSON.parse(#"{"gridNote":null}"#)))
        XCTAssertNil(gridNote(nil))
    }

    func testEveryItemButMissionSettingsOffersDeleteWaypointLikeTheMissionItemEditorMenu() {
        XCTAssertEqual("Delete waypoint", DELETE_ITEM_LABEL)
        XCTAssertFalse(itemDeletable(0))
        XCTAssertTrue(itemDeletable(1))
    }

    func testVehiclePositionModeShowsTheVehiclesCoordinateAndFrameAltitudeLikeEditPositionDialog() {
        let shown = vehiclePositionOf(JSON.parse(#"{"valid":true,"latitude":47.3977419,"longitude":8.5455938}"#), JSON.parse(#"{"kind":"fact","value":12.3,"valueString":"12.3","units":"m"}"#))
        XCTAssertEqual(VehiclePosition(latitude: "47.3977419", longitude: "8.5455938", altitude: "12.3 m"), shown)
        XCTAssertEqual("", vehiclePositionOf(JSON.parse(#"{"valid":true,"latitude":1.0,"longitude":2.0}"#), nil)?.altitude)
        XCTAssertNil(vehiclePositionOf(JSON.parse(#"{"valid":false}"#), nil))
        XCTAssertEqual(["Alt (Rel)", "Alt (AMSL)", "Alt (AGL)", "Alt (AGL)", nil], [1, 2, 3, 4, 0].map { vehicleAltitudeLabel($0) })
    }

    func testCommandsAndCategoriesReadFromTheCommandTree() {
        let commands = commandChoices(JSON.parse(#"[{"command":16,"friendlyName":"Waypoint","description":"Travel to a position in 3D space."},{"command":19,"friendlyName":"Loiter (time)","description":"d"}]"#))
        XCTAssertEqual([16, 19], commands.map(\.id))
        XCTAssertEqual("Waypoint", commands[0].name)
        XCTAssertEqual(["Basic", "Advanced"], categoryNames(JSON.parse(#"["Basic","Advanced"]"#)))
        XCTAssertTrue(commandChoices(nil).isEmpty)
    }

    func testAnItemsCommandIsWrittenOnItsOwnPath() {
        XCTAssertEqual("plan.missionController.visualItems.3.command", itemCommandPath(3))
        XCTAssertEqual("view.itemFacts(3)", itemFactsPath(3))
        XCTAssertEqual(1, itemFields(JSON.parse(#"{"fields":[{"name":"Hold","label":"Hold","control":"number","path":"p"}]}"#)).count)
    }

    func testLandingAltitudesCarryThePatternsAltitudeFrameLikeAltitudeFactTextField() {
        let fields = itemFields(JSON.parse(#"""
        {"fields":[
            {"name":"finalApproachAltitude","label":"Altitude","control":"number","units":"m","path":"plan.missionController.visualItems.4.finalApproachAltitude"},
            {"name":"landingAltitude","label":"Altitude","control":"number","units":"m","path":"plan.missionController.visualItems.4.landingAltitude"},
            {"name":"loiterRadius","label":"Radius","control":"number","units":"m","path":"plan.missionController.visualItems.4.loiterRadius"}]}
        """#))
        XCTAssertEqual(["m Rel", "m Rel", "m"], withLandingFrameUnits(fields, true).map(\.units))
        XCTAssertEqual(["m AMSL", "m AMSL", "m"], withLandingFrameUnits(fields, false).map(\.units))
        XCTAssertEqual(["m", "m", "m"], withLandingFrameUnits(fields, nil).map(\.units))
    }

    func testASectionHeaderSitsAboveTheFirstRowOfEachLandingEditorSection() {
        let view = JSON.parse(#"""
        {"fields":[
            {"path":"a","section":"Final approach"},{"path":"b","section":"Final approach"},
            {"path":"c","section":"Landing point"},{"path":"d","section":"Landing point"},{"path":"e"}]}
        """#)
        XCTAssertEqual(["a": "Final approach", "c": "Landing point"], sectionStarts(view))
        XCTAssertTrue(sectionStarts(JSON.parse(#"{"fields":[{"path":"a"}]}"#)).isEmpty)
    }

    func testDistanceAndGlideSlopeRowsCarryTheRadioButtonThatPicksBetweenThem() {
        let view = JSON.parse(#"""
        {"fields":[
            {"path":"i.landingDistance","choice":{"path":"i.valueSetIsDistance","value":true,"selected":false}},
            {"path":"i.glideSlope","choice":{"path":"i.valueSetIsDistance","value":false,"selected":true}},{"path":"i.landingHeading"}]}
        """#)
        XCTAssertEqual(
            [
                "i.landingDistance": RadioChoice(path: "i.valueSetIsDistance", value: true, selected: false),
                "i.glideSlope": RadioChoice(path: "i.valueSetIsDistance", value: false, selected: true),
            ],
            radioChoices(view)
        )
    }

    func testTheSpeedSectionIsOfferedOnlyWhereTheItemHasOne() {
        let speed = speedSection(JSON.parse(#"{"speedSection":{"available":true,"specified":true,"value":8.5,"units":"m/s","path":"p","specifyPath":"s"}}"#))!
        XCTAssertTrue(speed.specified)
        XCTAssertEqual(8.5, speed.value)
        XCTAssertNil(speedSection(JSON.parse(#"{"speedSection":{"available":false}}"#)))
        XCTAssertNil(speedSection(JSON.parse(#"{"speedSection":null}"#)))
    }

    func testPositionFormsSeedTheUtmAndMgrsFields() {
        let forms = positionForms(JSON.parse(#"{"utm":{"zone":32,"southern":false,"easting":464617.5,"northing":5247152.25},"mgrs":"32T MN 64617 47152"}"#))!
        XCTAssertEqual("32", forms.zone)
        XCTAssertEqual("464617.50", forms.easting)
        XCTAssertEqual("32T MN 64617 47152", forms.mgrs)
        XCTAssertEqual("view.utmToGeo(1,2,32,true)", utmToGeoPath("1", "2", "32", true))
        let geo = geoOf(JSON.parse(#"{"valid":true,"latitude":47.0,"longitude":8.0}"#))
        XCTAssertEqual(47, geo?.0)
        XCTAssertEqual(8, geo?.1)
        XCTAssertNil(geoOf(JSON.parse(#"{"valid":false}"#)))
    }

    func testALandingPatternsNotesComeFromTheCoreInOrder() {
        let view = JSON.parse(#"{"landing":true,"landingNotes":["* Actual flight path will vary.","* Avoid tailwind on approach to land."]}"#)
        XCTAssertEqual(["* Actual flight path will vary.", "* Avoid tailwind on approach to land."], landingNotes(view))
        XCTAssertEqual([], landingNotes(JSON.parse("{}")))
    }
}
