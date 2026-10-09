import XCTest
@testable import Aircast

final class SettingsViewTests: XCTestCase {
    private func pages(_ raw: String...) -> JSON {
        JSON.parse(#"{"pages":["# + raw.joined(separator: ",") + "]}")
    }

    func testAPageTheHeadDrawsNothingForIsNotOffered() {
        let view = pages(
            #"{"title":"Empty","sections":[]}"#,
            #"{"title":"Connections","showsLinks":true,"sections":[{"title":"Auto Connect","group":"autoConnectSettings"}]}"#,
            #"{"title":"Maps","sections":[{"title":"Maps","group":"mapsSettings"}]}"#
        )
        XCTAssertEqual(settingsPages(view).map(\.title), ["Connections", "Maps"])
    }

    func testTheAboutPageIsOfferedWithTheHelpLinksTheCoreServes() {
        let view = pages(#"{"title":"About","showsAbout":true,"sections":[],"helpLinks":[{"name":"QGroundControl User Guide","url":"https://docs.qgroundcontrol.com","host":"docs.qgroundcontrol.com"}]}"#)
        let about = settingsPages(view)
        XCTAssertEqual(about.count, 1)
        XCTAssertEqual(about.first?.title, "About")
        XCTAssertEqual(about.first?.helpLinks.map(\.host), ["docs.qgroundcontrol.com"])
    }

    func testAPageWithNoSectionsIsStillOfferedWhenTheHeadDrawsItsOwnBlock() {
        XCTAssertEqual(settingsPages(pages(#"{"title":"Connections","showsLinks":true,"sections":[]}"#)).map(\.title), ["Connections"])
    }

    func testAPageThisHeadDeliberatelyLeavesOutIsNotOffered() {
        let raw = PAGES_WITHOUT_A_SCREEN.keys.map { #"{"title":""# + $0 + #"","sections":[{"title":""# + $0 + #"","group":"g"}]}"# }
        let view = JSON.parse(#"{"pages":["# + raw.joined(separator: ",") + "]}")
        XCTAssertTrue(settingsPages(view).isEmpty, "each of these has a stated reason, and the generic renderer would happily draw all of them")
        XCTAssertTrue(PAGES_WITHOUT_A_SCREEN.values.allSatisfy { !$0.isBlank })
    }

    func testTheActionsSectionKeepsItsHeadingButNotTheRawFilePathRows() {
        let view = JSON.parse(
            #"""
            {"title":"General","sections":[
              {"title":"Actions","group":"mavlinkActionsSettings","subsections":[
                {"title":"","controls":[{"name":"flyViewActionsFile","label":"Fly View","control":"text","path":"p"}]}]},
              {"title":"Application","group":"appSettings","subsections":[
                {"title":"","controls":[{"name":"audioMuted","label":"Mute","control":"toggle","path":"q"}]}]}
            ]}
            """#
        )
        let sections = settingsSections(view)
        XCTAssertEqual(sections.map(\.title), ["Actions", "Application"])
        XCTAssertEqual(sections.first?.blocks.flatMap(\.facts).map(\.name), [], "MavlinkActionsSection draws the pickers and the folder note")
    }

    private let general = JSON.parse(
        #"""
        {"title":"General","sections":[
          {"title":"Application","group":"appSettings","note":"","subsections":[
            {"title":"Sound","controls":[{"name":"audioMuted","label":"Mute","control":"toggle","path":"settings.appSettings.audioMuted"}]},
            {"title":"Empty","controls":[]}
          ]},
          {"title":"Units","group":"unitsSettings","note":"","subsections":[
            {"title":"","controls":[{"name":"speedUnits","label":"Speed","control":"choice","path":"settings.unitsSettings.speedUnits"}]}
          ]}
        ]}
        """#
    )

    func testASectionReadsAsItsSubsectionsAndAnEmptyOneIsDropped() {
        let read = settingsSections(general)
        XCTAssertEqual(read.map(\.title), ["Application", "Units"])
        XCTAssertEqual(read[0].blocks.map(\.title), ["Sound"])
        XCTAssertEqual(read[0].blocks[0].facts.map(\.name), ["audioMuted"])
    }

    func testAControlKeepsThePathTheWriteGoesTo() {
        let fact = settingsSections(general)[0].blocks[0].facts[0]
        XCTAssertEqual(fact.path, "settings.appSettings.audioMuted")
        XCTAssertTrue(fact.isBool)
    }

    func testAnInvertedSwitchShowsTheOppositeOfTheStoredValue() {
        let inverted = factFromControl(JSON.parse(#"{"name":"apmStartMavlinkStreams","label":"Controlled by Vehicle","control":"toggle","value":true,"inverted":true}"#))!
        XCTAssertFalse(inverted.boolValue)
        let plain = factFromControl(JSON.parse(#"{"name":"apmStartMavlinkStreams","label":"Controlled by Vehicle","control":"toggle","value":true,"inverted":false}"#))!
        XCTAssertTrue(plain.boolValue)
    }

    func testTheHeadingNamesTheSubsectionOrTheSectionWhenItSaysSomethingThePageTitleDoesNot() {
        let read = settingsSections(general)
        XCTAssertEqual(blockHeading("General", read[0], read[0].blocks[0]), "Sound")
        XCTAssertEqual(blockHeading("General", read[1], read[1].blocks[0]), "Units")
        XCTAssertEqual(
            blockHeading("Plan View", SettingsSectionRows(title: "Plan View", group: "planViewSettings", note: "", blocks: []), SettingsBlock(title: "", facts: [])),
            "",
            "a page called Plan View does not need a heading called Plan View under it"
        )
    }

    func testASectionWithNothingInItIsNotDrawn() {
        let view = JSON.parse(#"{"title":"X","sections":[{"title":"Gone","group":"g","subsections":[{"title":"","controls":[]}]}]}"#)
        XCTAssertTrue(settingsSections(view).isEmpty)
    }

    func testTheVideoPageCarriesTheFlagTheExtraSourcesEditorIsDrawnOn() {
        XCTAssertEqual(settingsPages(pages(#"{"title":"Video","showsVideoSources":true,"sections":[{"title":"Video","group":"videoSettings"}]}"#)).map(\.showsVideoSources), [true])
        XCTAssertEqual(
            settingsPages(pages(#"{"title":"Maps","sections":[{"title":"Maps","group":"mapsSettings"}]}"#)).map(\.showsVideoSources),
            [false],
            "a page that does not ask for the block must not get it"
        )
    }

    func testASearchFindsASettingByWhatItIsCalledAndByItsName() {
        let read = settingsSections(general)
        XCTAssertEqual(matchesIn("General", read, "mute").map(\.title), ["General \u{203a} Application"])
        XCTAssertEqual(
            matchesIn("General", read, "audioMuted").flatMap { $0.blocks.flatMap(\.facts) }.map(\.name),
            ["audioMuted"],
            "an operator who knows the fact's name should not have to guess the page"
        )
        XCTAssertEqual(matchesIn("General", read, "MUTE").count, 1, "a typed query is not case sensitive")
    }

    func testAnEmptySearchMatchesNothingRatherThanEverything() {
        let read = settingsSections(general)
        XCTAssertTrue(matchesIn("General", read, "").isEmpty)
        XCTAssertTrue(matchesIn("General", read, "   ").isEmpty)
    }

    func testASearchResultKeepsThePathItsWriteGoesTo() {
        let hits = matchesIn("General", settingsSections(general), "mute")
        XCTAssertEqual(hits.count, 1)
        XCTAssertEqual(hits.first?.blocks.flatMap(\.facts).map(\.path), ["settings.appSettings.audioMuted"])
        XCTAssertEqual(hits.first?.note, "", "a note about facts the head drew elsewhere makes no sense beside one search hit")
    }

    func testTheUnitRowsAreReachedThroughTheirOwnSectionNeverThroughSearch() {
        XCTAssertTrue(
            matchesIn("General", settingsSections(general), "speed").isEmpty,
            "whether a single measurement may be set at all depends on the measurement system, "
                + "and that gate lives in the section this head draws itself"
        )
    }

    func testTheHeadSuppressesTheNoteOnlyWhereItDrawsTheEditorItself() {
        XCTAssertEqual(GROUPS_WITH_A_HEAD_EDITOR, [FLY_VIEW_GROUP])
        XCTAssertFalse(
            GROUPS_WITH_A_HEAD_EDITOR.contains(UNITS_GROUP),
            "the core's note sends the operator to the desktop for on-screen RC controls, which is wrong on a head that has an editor for them"
        )
    }

    func testTheConnectionsRowCountsConnectedLinks() {
        let view = JSON.parse(#"{"links":[{"connected":true},{"connected":false},{"connected":true}]}"#)
        XCTAssertEqual(activeLinkCount(view), 2)
        XCTAssertEqual(activeLinksText(2), "2 active")
        XCTAssertEqual(activeLinksText(0), "")
        XCTAssertEqual(activeLinkCount(nil), 0)
    }

    func testASearchBreadcrumbReadsInSentenceCase() {
        XCTAssertEqual(shownBreadcrumb("Maps \u{203a} Flight Map"), "Maps \u{203a} Flight map")
    }

    func testTheConnectionsPageReadsAsLinksLikePenpot() {
        XCTAssertEqual(pageTitle(CONNECTIONS_PAGE), "Links")
        XCTAssertEqual(pageTitle("Fly View"), "Fly view")
        XCTAssertEqual(pageTitle("ADSB Server"), "ADS-B server")
    }

    func testAGlanceJoinsTheSetValuesOnly() {
        XCTAssertEqual(glanceText(["Bing", "Hybrid"]), "Bing · Hybrid")
        XCTAssertEqual(glanceText(["", " "]), "")
        XCTAssertEqual(glanceText(["Video Stream Disabled", "Google", "Satellite"]), "Video stream disabled · Google · Satellite")
    }

    func testTheConnectionsRowNamesItsLiveLinks() {
        let view = JSON.parse(#"{"links":[{"name":"UDP Link","connected":true,"summary":"UDP port 14550"},{"name":"Spare","connected":false,"summary":"COM3"},{"name":"TCP 127.0.0.1:5771","connected":true,"summary":""}]}"#)
        XCTAssertEqual(activeLinksGlance(view), "UDP port 14550 \u{00b7} TCP 127.0.0.1:5771")
        XCTAssertEqual(activeLinksGlance(nil), "")
    }

    func testPagesMatchOnTheirQgcKeywordsAndSettingsOnTheirs() {
        let telemetry = SettingsPageEntry(title: "MAVLink", showsLinks: false, showsVideoSources: false, sectionCount: 0, keywords: "ground, station, signing, forwarding")
        XCTAssertTrue(pageMatches(telemetry, "Signing"))
        XCTAssertTrue(pageMatches(telemetry, "mav"))
        XCTAssertFalse(pageMatches(telemetry, "offline"))
        XCTAssertFalse(pageMatches(telemetry, " "))
    }

    func testAboutListsThePrivacyPolicyAfterTheSupportLinks() {
        let guide = HelpLink(name: "QGroundControl User Guide", url: "https://docs.qgroundcontrol.com", host: "docs.qgroundcontrol.com")
        XCTAssertEqual(aboutLinks([guide]), [guide, PRIVACY_POLICY_LINK])
        XCTAssertEqual(aboutLinks([]).map(\.url), ["https://aircast.one/privacy"])
    }
}
