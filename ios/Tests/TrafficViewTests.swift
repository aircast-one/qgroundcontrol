import XCTest
@testable import Aircast

final class TrafficViewTests: XCTestCase {
    private let near = #"{"icaoAddress":11259375,"callsign":"BAW42","distance":1200.0,"bearingDegrees":95.0,"altitude":450.0,"relativeAltitude":200.0,"emergency":null,"alert":false,"stale":false}"#

    private func synthetic(alert: Bool = false) -> String {
        String(near.replacingOccurrences(of: #""alert":false"#, with: #""alert":\#(alert)"#).dropLast()) + #","simulated":true}"#
    }

    private func view(
        enabled: Bool = true,
        available: Bool = true,
        connected: Bool = true,
        receiving: Bool = true,
        ownPositionKnown: Bool = true,
        alerting: String = "null",
        alertUnknown: Int = 0,
        emergency: String = "null",
        error: String = "null",
        contacts: String = "[]"
    ) -> JSON {
        JSON.parse(#"""
            {"kind":"object","class":"AdsbTraffic","enabled":\#(enabled),"available":\#(available),
             "connected":\#(connected),"receiving":\#(receiving),"ownPositionKnown":\#(ownPositionKnown),"alerting":\#(alerting),"alertUnknown":\#(alertUnknown),
             "emergency":\#(emergency),"error":\#(error),
             "units":{"altitude":"m","velocity":"m/s","heading":"deg","distance":"m","bearing":"deg"},
             "contacts":\#(contacts)}
            """#)
    }

    private func read(_ json: JSON) throws -> TrafficReading { try XCTUnwrap(trafficReading(json)) }

    func testASkyWithNothingButSyntheticTargetsDoesNotReportAircraft() throws {
        XCTAssertEqual(
            trafficSummary(try read(view(contacts: "[" + synthetic() + "," + synthetic() + "]"))),
            "Traffic: 2 simulated",
            "counting a rig's synthetic targets as aircraft states something false about the sky"
        )
    }

    func testRealAndSyntheticTargetsAreCountedApart() throws {
        XCTAssertEqual(trafficSummary(try read(view(contacts: "[\(near)," + synthetic() + "," + synthetic() + "]"))), "Traffic: 1 aircraft \u{00b7} 2 simulated")
    }

    func testASkyOfRealAircraftReadsExactlyAsBefore() throws {
        XCTAssertEqual(trafficSummary(try read(view(contacts: "[\(near)]"))), "Traffic: 1 aircraft")
        XCTAssertEqual(trafficSummary(try read(view(contacts: "[\(near),\(near),\(near)]"))), "Traffic: 3 aircraft")
    }

    func testASimulatedTargetIsNotAnAircraftAndTheRowSaysSoFirst() throws {
        let reading = try read(view(contacts: "[\(synthetic(alert: true))]"))
        let line = trafficContactText(reading.contacts[0], reading.units)
        XCTAssertTrue(line.hasPrefix("simulated"), "ADSB_FLAGS_SIMULATED reaches the head on every contact and must reach the row")
        XCTAssertTrue(line.contains("alerting"))
    }

    func testARealContactCarriesNoSuchMark() throws {
        let reading = try read(view(contacts: "[\(near)]"))
        XCTAssertFalse(trafficContactText(reading.contacts[0], reading.units).contains("simulated"))
    }

    func testAPayloadFromAnotherViewIsNotATrafficReading() {
        XCTAssertNil(trafficReading(nil))
        XCTAssertNil(trafficReading(JSON.parse(#"{"kind":"object","class":"Detections"}"#)))
    }

    func testAContactCarriesTheCoresConvertedNumbersAndTheBlockCarriesTheUnit() throws {
        let reading = try read(view(contacts: "[\(near)]"))
        XCTAssertEqual(reading.contacts.count, 1)
        XCTAssertEqual(reading.contacts[0].name, "BAW42")
        XCTAssertEqual(trafficContactText(reading.contacts[0], reading.units), "1200 m  95 deg  200 m above")
    }

    func testAContactWithNoCallsignIsNamedByItsIcaoAddress() throws {
        let anonymous = near.replacingOccurrences(of: #""callsign":"BAW42""#, with: #""callsign":null"#)
        XCTAssertEqual(try read(view(contacts: "[\(anonymous)]")).contacts[0].name, "ABCDEF")
    }

    func testTheCoreWithholdsRangeWhenItCannotPlaceTheOperatorAndZeroWouldBeALie() throws {
        let unplaced = near
            .replacingOccurrences(of: #""distance":1200.0"#, with: #""distance":null"#)
            .replacingOccurrences(of: #""bearingDegrees":95.0"#, with: #""bearingDegrees":null"#)
        let reading = try read(view(contacts: "[\(unplaced)]"))
        XCTAssertEqual(trafficContactText(reading.contacts[0], reading.units), "bearing unknown  200 m above")
    }

    func testAStaleFixSaysSoBecauseAtJetSpeedItIsNotWhereItIsDrawn() throws {
        let old = near.replacingOccurrences(of: #""stale":false"#, with: #""stale":true"#)
        let reading = try read(view(contacts: "[\(old)]"))
        XCTAssertEqual(trafficContactText(reading.contacts[0], reading.units), "1200 m  95 deg  200 m above  stale")
    }

    func testTheServerSwitchDoesNotGovernWhatTheVehicleRelaysOverMavlink() throws {
        XCTAssertFalse(trafficShown(try read(view(enabled: false))))
        XCTAssertTrue(trafficShown(try read(view(enabled: false, contacts: "[\(near)]"))))
        XCTAssertTrue(trafficShown(try read(view())))
    }

    func testAConfiguredReceiverSwitchedOffIsNotAnAbsentOne() throws {
        XCTAssertEqual(trafficSummary(try read(view(available: false, receiving: false))), "No traffic receiver")
        XCTAssertFalse(try read(view(enabled: false)).enabled)
    }

    func testEachWayOfHearingNothingGetsItsOwnSentence() throws {
        XCTAssertEqual(trafficSummary(try read(view())), "Traffic clear")
        XCTAssertEqual(trafficSummary(try read(view(receiving: false))), "No traffic feed")
        XCTAssertEqual(
            trafficSummary(try read(view(receiving: false, error: #"{"token":"connectFailed","detail":"connection refused"}"#))),
            "Traffic server unreachable"
        )
        XCTAssertEqual(
            trafficSummary(try read(view(receiving: false, error: #"{"token":"linkLost","detail":"closed"}"#))),
            "Traffic feed dropped"
        )
    }

    func testContactsOutrankEveryComplaintAboutTheFeedBecauseTheyAreTheAnswerAskedFor() throws {
        XCTAssertEqual(trafficSummary(try read(view(available: false, receiving: false, contacts: "[\(near)]"))), "Traffic: 1 aircraft")
    }

    func testAnUnreportedAlertIsCautionNotACalmSky() throws {
        XCTAssertEqual(trafficLevel(try read(view())), .Good)
        XCTAssertEqual(trafficLevel(try read(view(alertUnknown: 1))), .Caution)
        XCTAssertEqual(trafficLevel(try read(view(connected: false, receiving: false))), .Caution)
        XCTAssertEqual(trafficLevel(try read(view(alerting: "true"))), .Warning)
    }

    func testRelayedTrafficNeverStatesAnAlertSoItsUnknownCountIsNotASignal() throws {
        let relayed = view(available: false, connected: false, receiving: true, alertUnknown: 1, contacts: "[\(near)]")
        XCTAssertEqual(trafficLevel(try read(relayed)), .Good)
    }

    func testAnAircraftSquawkingAnEmergencyOutranksAMerelyCloseOne() throws {
        let panic = try read(view(alerting: "true", emergency: #""hijack""#, contacts: "[\(near)]"))
        XCTAssertEqual(trafficLevel(panic), .Critical)
        XCTAssertEqual(trafficEmergencyText(panic.emergency), "squawking hijack")
    }

    func testAStatedAllClearIsNotAnUnknownOne() throws {
        XCTAssertNil(try read(view()).alerting)
        XCTAssertEqual(try read(view(alerting: "false")).alerting, false)
    }

    func testACoarseUnitKeepsADecimalOrACloseAircraftInMilesReadsAsZeroAway() throws {
        let miles = JSON.parse(#"""
            {"kind":"object","class":"AdsbTraffic","enabled":true,"available":true,"receiving":true,
             "alerting":null,"alertUnknown":0,"emergency":null,"error":null,
             "units":{"altitude":"ft","velocity":"mph","heading":"deg","distance":"mi","bearing":"deg"},
             "contacts":[{"icaoAddress":1,"callsign":"N1","distance":0.4,"bearingDegrees":95.0,
             "altitude":1500.0,"relativeAltitude":200.0,"emergency":null,"alert":false,"stale":false}]}
            """#)
        let reading = try read(miles)
        XCTAssertEqual(trafficContactText(reading.contacts[0], reading.units), "0.4 mi  95 deg  200 ft above")
    }

    func testHeightIsStatedAgainstMyOwnBecauseAtMyLevelIsTheQuestionBeingAsked() throws {
        let at = { (relative: String) throws -> String in
            let reading = try self.read(self.view(contacts: "[\(self.near.replacingOccurrences(of: #""relativeAltitude":200.0"#, with: #""relativeAltitude":\#(relative)"#))]"))
            return trafficContactText(reading.contacts[0], reading.units)
        }
        XCTAssertEqual(try at("200.0"), "1200 m  95 deg  200 m above")
        XCTAssertEqual(try at("-150.0"), "1200 m  95 deg  150 m below")
        XCTAssertEqual(try at("0.0"), "1200 m  95 deg  my level")
        XCTAssertEqual(try at("null"), "1200 m  95 deg  450 m")
    }

    func testTheContactTheBlockIsWarningAboutSaysSoOnItsOwnRow() throws {
        let alerting = near.replacingOccurrences(of: #""alert":false"#, with: #""alert":true"#)
        let reading = try read(view(alerting: "true", contacts: "[\(alerting)]"))
        XCTAssertEqual(trafficContactText(reading.contacts[0], reading.units), "1200 m  95 deg  200 m above  alerting")
    }

    func testTheReferenceForEveryNumberIsStatedOnceNotGuessedFromARow() throws {
        XCTAssertEqual(trafficCaption(try read(view())), "Range, bearing and height are relative to the vehicle")
        XCTAssertEqual(trafficCaption(try read(view(ownPositionKnown: false))), "No vehicle position, so nothing can be ranged")
    }

    func testTheRowThatEarnedTheWarningIsTheOneMarkedNotTheWholeList() throws {
        let calm = try read(view(contacts: "[\(near)]")).contacts[0]
        let loud = try read(view(contacts: "[\(near.replacingOccurrences(of: #""alert":false"#, with: #""alert":true"#))]")).contacts[0]
        let squawking = try read(view(contacts: "[\(near.replacingOccurrences(of: #""emergency":null"#, with: #""emergency":"hijack""#))]")).contacts[0]
        XCTAssertFalse(trafficContactUrgent(calm))
        XCTAssertTrue(trafficContactUrgent(loud))
        XCTAssertTrue(trafficContactUrgent(squawking))
    }

    func testAnAbsoluteAltitudeSaysWhatItIsMeasuredFrom() {
        let units = TrafficUnits(distance: "ft", altitude: "ft", heading: "deg")
        let contact = { (type: String) in
            TrafficContact(icaoAddress: 1, callsign: "SWR000", distance: nil, bearingDegrees: nil, altitude: 1000.0, relativeAltitude: nil, emergency: "", alert: nil, stale: false, altitudeType: type)
        }
        XCTAssertEqual(trafficHeightText(contact("pressureQnh"), units), "1000 ft by pressure", "with no vehicle the bare number is the aircraft's own altitude, so it names its datum")
        XCTAssertEqual(trafficHeightText(contact("geometric"), units), "1000 ft by GPS")
        XCTAssertEqual(trafficHeightText(contact("somethingNew"), units), "1000 ft", "a datum this head does not know is left unnamed rather than guessed")
    }

    func testAHeightRelativeToTheVehicleStillReadsAsASeparation() {
        let units = TrafficUnits(distance: "ft", altitude: "ft", heading: "deg")
        let above = TrafficContact(icaoAddress: 1, callsign: "A", distance: 100.0, bearingDegrees: 90.0, altitude: 1000.0, relativeAltitude: 500.0, emergency: "", alert: nil, stale: false, altitudeType: "pressureQnh")
        XCTAssertEqual(trafficHeightText(above, units), "500 ft above", "the datum belongs only on the absolute form")
    }

    func testACalmSkyRaisesNoBannerHoweverMuchTrafficIsDrawn() throws {
        XCTAssertNil(trafficAlert(try read(view(contacts: "[\(near)," + synthetic() + "]"))))
        XCTAssertNil(trafficAlert(try read(view(enabled: false))))
    }

    func testAFeedProblemIsACautionThatNamesTheProblemNotACountOfAircraft() throws {
        XCTAssertEqual(
            trafficAlert(try read(view(receiving: false, error: #"{"token":"linkLost","detail":"closed"}"#, contacts: "[\(near)]"))),
            TrafficAlert(level: .Caution, title: "Traffic feed dropped", detail: "")
        )
        XCTAssertEqual(trafficAlert(try read(view(receiving: false))), TrafficAlert(level: .Caution, title: "No traffic feed", detail: ""))
        XCTAssertEqual(trafficAlert(try read(view(alertUnknown: 2))), TrafficAlert(level: .Caution, title: "Collision alerts unknown for 2 aircraft", detail: ""))
    }

    func testAWarningNamesTheClosestAlertingAircraftWhereItIsAndHowHigh() throws {
        let far = near.replacingOccurrences(of: "BAW42", with: "EZY9").replacingOccurrences(of: "1200.0", with: "5000.0").replacingOccurrences(of: #""alert":false"#, with: #""alert":true"#)
        let close = near.replacingOccurrences(of: #""alert":false"#, with: #""alert":true"#)
        XCTAssertEqual(
            trafficAlert(try read(view(alerting: "true", contacts: "[\(far),\(near),\(close)]"))),
            TrafficAlert(level: .Warning, title: "Aircraft nearby", detail: "BAW42 \u{00b7} 1200 m E \u{00b7} 200 m above")
        )
        XCTAssertEqual(
            trafficAlert(try read(view(alerting: "true", emergency: #""hijack""#, contacts: "[\(near)]"))),
            TrafficAlert(level: .Critical, title: "Aircraft squawking hijack", detail: "")
        )
    }

    func testABearingReadsAsTheNearestOfEightCompassPoints() {
        XCTAssertEqual([0.0, 44.0, 95.0, 180.0, 315.0, 350.0, -10.0].map(compassPoint), ["N", "NE", "E", "S", "NW", "N", "N"])
    }
}
