import XCTest
@testable import Aircast

final class InstrumentChoiceTests: XCTestCase {
    func testTheTilesAlreadyShownLeadTheSheetInTheirOwnOrderAndLeaveTheirGroups() {
        let alt = InstrumentFact(name: "alt", label: "Altitude", path: "a.alt")
        let speed = InstrumentFact(name: "speed", label: "Speed", path: "a.speed")
        let roll = InstrumentFact(name: "roll", label: "Roll", path: "v.roll")
        let groups = [InstrumentGroup(group: "vehicle", title: "Vehicle", facts: [roll]), InstrumentGroup(group: "air", title: "Air", facts: [alt, speed])]

        let shown = shownFirst(groups, ["a.speed", "a.alt"])
        XCTAssertEqual(shown.map(\.title), ["Shown", "Vehicle"])
        XCTAssertEqual(shown.map { $0.facts.map(\.label) }, [["Speed", "Altitude"], ["Roll"]])
        XCTAssertEqual(shownFirst(groups, []).map(\.title), ["Vehicle", "Air"])
    }

    private let served = JSON.parse(#"""
        {"available":true,"groups":[
          {"group":"gps","title":"GPS","facts":[
            {"name":"lat","label":"Latitude","selection":"gps/lat"},
            {"name":"hdop","label":"HDOP","selection":"gps/hdop"}]},
          {"group":"wind","title":"Wind","facts":[
            {"name":"speed","label":"Wind Speed","selection":"wind/speed"}]},
          {"group":"empty","title":"Empty","facts":[]}
        ]}
        """#)

    func testAGroupTheVehicleReportsNothingForIsNotOffered() {
        XCTAssertEqual(instrumentGroups(served).map(\.group), ["gps", "wind"])
    }

    func testEachBatteryPackIsOfferedAfterTheVehiclesGroupsLikeTheBattery0FactGroupQgcsValueEditorLists() {
        let packs = JSON.parse(#"[{"group":"batteries.0","title":"Battery 1","facts":[{"name":"voltage","label":"Voltage","selection":"batteries.0/voltage"}]}]"#)
        let withPacks = JSON.object((served.object ?? [:]).merging(["packGroups": packs]) { $1 })
        XCTAssertEqual(instrumentGroups(withPacks).map(\.group), ["gps", "wind", "batteries.0"])
        XCTAssertEqual(instrumentGroups(withPacks).last?.facts.map(\.path), ["batteries.0/voltage"])
    }

    func testAFactIsAskedForByTheSelectionTheCoreSpells() throws {
        let gps = try XCTUnwrap(instrumentGroups(served).first)
        XCTAssertEqual(
            gps.facts.map(\.path),
            ["gps/lat", "gps/hdop"],
            "a bare lon resolves against the vehicle group and answers noSuchFact, which this head drops silently"
        )
        XCTAssertEqual(gps.facts.first?.label, "Latitude")
    }

    func testAFactTheCoreDidNotSpellASelectionForIsNotOffered() {
        let unspelled = JSON.parse(#"{"available":true,"groups":[{"group":"gps","title":"GPS","facts":[{"name":"lat","label":"Latitude"}]}]}"#)
        XCTAssertTrue(instrumentGroups(unspelled).isEmpty, "a row that cannot be asked for is a row that silently does nothing when tapped")
    }

    func testNoVehicleOffersNothingRatherThanAnEmptyCatalogue() {
        XCTAssertTrue(instrumentGroups(JSON.parse(#"{"available":false,"groups":[]}"#)).isEmpty)
        XCTAssertTrue(instrumentGroups(nil).isEmpty)
    }

    func testChoosingNothingDrawsNothingAndTheCoresOwnFallbackIsNeverReached() {
        XCTAssertEqual(instrumentsPath(["lat", "hdop"]), "view.instruments(lat,hdop)")
        XCTAssertEqual(instrumentsPath(DEFAULT_INSTRUMENTS), "view.instruments", "the default selection asks the core, which adds airspeed for a wing as QGCCorePlugin does")
        XCTAssertEqual(instrumentsPath(defaultInstruments("fixedWing"), vehicleClass: "fixedWing"), "view.instruments", "a wing's default list with airspeed still asks the core, keeping the AirSpd text")
        XCTAssertEqual(
            instrumentsPath(DEFAULT_INSTRUMENTS, vehicleClass: "fixedWing"),
            "view.instruments(\(DEFAULT_INSTRUMENTS.joined(separator: ",")))",
            "a wing with airspeed turned off names its list, so the core does not add it back"
        )
        XCTAssertTrue(showsInstruments(["lat"]))
        XCTAssertFalse(showsInstruments([]), "instruments_view answers its own four defaults for an empty argument list, so the strip is gated here rather than by the path")
    }

    func testChoosingTogglesWithNoLimitTheRowWrapsLikeTelemetryChipsLayer() {
        XCTAssertEqual(withInstrument(["a"], "b"), ["a", "b"])
        XCTAssertEqual(withInstrument(["a", "b"], "b"), ["a"])
        let many = (1...9).map { "f\($0)" }
        XCTAssertEqual(withInstrument(many, "extra"), many + ["extra"])
    }

    func testTheNoteSaysWhereTheOperatorStands() {
        XCTAssertTrue(instrumentChoiceNote([]).contains("no readings"))
        XCTAssertEqual(instrumentChoiceNote(["a", "b"]), "2 chosen.")
    }

    func testTheReadingsTheScreenStartsWithAreInTheCatalogueAndMatchWhatIsChosen() throws {
        let own = try XCTUnwrap(vehicleOwnGroup(JSON.parse(#"""
            {"kind":"object","available":true,"groups":[],"vehicleFacts":[
              {"name":"altitudeRelative","label":"Alt (Rel)","selection":"altitudeRelative"},
              {"name":"groundSpeed","label":"Ground Speed","selection":"groundSpeed"},
              {"name":"distanceToHome","label":"Distance to Home","selection":"distanceToHome"},
              {"name":"heading","label":"Heading","selection":"heading"},
              {"name":"climbRate","label":"Climb Rate","selection":"climbRate"},
              {"name":"rangeFinderDist","label":"Range Finder Dist","selection":"rangeFinderDist"}]}
            """#)))
        XCTAssertEqual(own.title, "Vehicle")
        XCTAssertTrue(
            DEFAULT_INSTRUMENTS.allSatisfy { name in own.facts.contains { $0.path == name } },
            "without the vehicle's own readings the four defaults are absent from the sheet and cannot be unchecked"
        )
        XCTAssertEqual(own.facts.first?.path, "altitudeRelative", "a top-level fact is asked for by bare name; only a child group's fact is qualified")
        XCTAssertEqual(own.facts.last?.label, "Range Finder Dist", "view.instrumentGroups humanises facts that carry no description")
    }

    func testNoVehicleContributesNoGroupRatherThanAnEmptyOne() {
        XCTAssertNil(vehicleOwnGroup(JSON.parse(#"{"kind":"object","available":false,"vehicleFacts":[]}"#)))
        XCTAssertNil(vehicleOwnGroup(JSON.parse(#"{"kind":"object","available":true,"vehicleFacts":[]}"#)))
        XCTAssertNil(vehicleOwnGroup(nil))
    }

    func testAnEmptyCatalogueWithAVehicleConnectedDoesNotTellTheOperatorToConnectOne() {
        XCTAssertEqual(
            emptyCatalogueText(true),
            "This vehicle reported no readings this screen can ask for.",
            "a connect prompt in front of a connected vehicle reads as a regression rather than as the head refusing to guess"
        )
        XCTAssertEqual(emptyCatalogueText(false), "Connect a vehicle to see what it can report.")
    }

    func testEachVehicleClassKeepsItsOwnInstrumentsLikeFactValueGridsSettingsKey() {
        XCTAssertEqual(instrumentVehicleClass(JSON.parse(#"{"vehicleClass":"multiRotor"}"#)), "multiRotor")
        XCTAssertEqual(instrumentVehicleClass(nil), "generic")
        XCTAssertEqual(chosenKey("fixedWing"), "chosen-fixedWing")
    }

    func testTheSizePillCyclesQgcsFourValueSizesAndSaysWhich() {
        XCTAssertEqual(ValueSize.allCases.map(\.label), ["Default", "Small", "Medium", "Large"])
        XCTAssertEqual(nextValueSize(.Default), .Small)
        XCTAssertEqual(nextValueSize(.Large), .Default)
        XCTAssertEqual(valueSizeAt(9), .Default)
        XCTAssertEqual(valueSizePillText(.Medium), "Size: Medium")
        XCTAssertEqual(ValueSize.allCases.map(\.scale), [1, 0.86, 1.25, 1.5])
    }

    func testAScaledValueKeepsTheTelemetryFiguresAndGrowsItsLineWithIt() {
        XCTAssertEqual(scaledNumber(1.5), TypeScale.telemetry.font(TypeScale.telemetry.size * 1.5))
        XCTAssertNotEqual(scaledNumber(1.5), TypeScale.telemetry.font)
    }

    func testValuesMoveSwapTheirReadingAndGoAwayInPlace() {
        let row = ["a", "b", "c"]
        XCTAssertEqual(movedInstrument(row, 1, -1), ["b", "a", "c"])
        XCTAssertEqual(movedInstrument(row, 2, 1), row)
        XCTAssertEqual(replacedInstrument(row, 1, "x"), ["a", "x", "c"])
        XCTAssertEqual(replacedInstrument(row, 0, "c"), ["c", "b"], "a reading already in the row moves rather than doubling")
        XCTAssertEqual(removedInstrument(row, 1), ["a", "c"])
    }

    func testABareVehicleSelectionAndItsServedIdAgree() {
        XCTAssertEqual(selectionId("altitudeRelative"), "vehicle/altitudeRelative")
        XCTAssertEqual(selectionId("gps/lock"), "gps/lock")
    }
}
