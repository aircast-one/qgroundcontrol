package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SettingsViewTest {

    private fun pages(vararg raw: String) =
        JSONObject("""{"pages":[${raw.joinToString(",")}]}""")

    @Test
    fun `a page the head draws nothing for is not offered`() {
        val view = pages(
            """{"title":"Empty","sections":[]}""",
            """{"title":"Connections","showsLinks":true,"sections":[{"title":"Auto Connect","group":"autoConnectSettings"}]}""",
            """{"title":"Maps","sections":[{"title":"Maps","group":"mapsSettings"}]}""",
        )
        assertEquals(listOf("Connections", "Maps"), settingsPages(view).map { it.title })
    }

    @Test
    fun `the about page is offered with the help links the core serves`() {
        val view = pages(
            """{"title":"About","showsAbout":true,"sections":[],"helpLinks":[{"name":"QGroundControl User Guide","url":"https://docs.qgroundcontrol.com","host":"docs.qgroundcontrol.com"}]}""",
        )
        val about = settingsPages(view).single()
        assertEquals("About", about.title)
        assertEquals("docs.qgroundcontrol.com", about.helpLinks.single().host)
    }

    @Test
    fun `a page with no sections is still offered when the head draws its own block`() {
        val view = pages("""{"title":"Connections","showsLinks":true,"sections":[]}""")
        assertEquals(listOf("Connections"), settingsPages(view).map { it.title })
    }

    @Test
    fun `a page this head deliberately leaves out is not offered`() {
        val view = pages(
            *PAGES_WITHOUT_A_SCREEN.keys.map {
                """{"title":"$it","sections":[{"title":"$it","group":"g"}]}"""
            }.toTypedArray(),
        )
        assertTrue(
            "each of these has a stated reason, and the generic renderer would happily draw all of them",
            settingsPages(view).isEmpty(),
        )
        assertTrue(PAGES_WITHOUT_A_SCREEN.values.none { it.isBlank() })
    }

    @Test
    fun `the actions section keeps its heading but not the raw file path rows`() {
        val view = JSONObject(
            """{"title":"General","sections":[
                 {"title":"Actions","group":"mavlinkActionsSettings","subsections":[
                   {"title":"","controls":[{"name":"flyViewActionsFile","label":"Fly View","control":"text","path":"p"}]}]},
                 {"title":"Application","group":"appSettings","subsections":[
                   {"title":"","controls":[{"name":"audioMuted","label":"Mute","control":"toggle","path":"q"}]}]}
               ]}""",
        )
        val sections = settingsSections(view)
        assertEquals(listOf("Actions", "Application"), sections.map { it.title })
        assertEquals("MavlinkActionsSection draws the pickers and the folder note", emptyList<String>(), sections.first().blocks.flatMap { it.facts }.map { it.name })
    }

    @Test
    fun `every page the head offers carries a line saying what is in it`() {
        assertTrue(
            "a page with no note reads as a bare word in the list",
            PAGES_WITHOUT_A_SCREEN.keys.none { it in PAGE_NOTES },
        )
    }

    private val general = JSONObject(
        """
        {"title":"General","sections":[
          {"title":"Application","group":"appSettings","note":"","subsections":[
            {"title":"Sound","controls":[{"name":"audioMuted","label":"Mute","control":"toggle","path":"settings.appSettings.audioMuted"}]},
            {"title":"Empty","controls":[]}
          ]},
          {"title":"Units","group":"unitsSettings","note":"","subsections":[
            {"title":"","controls":[{"name":"speedUnits","label":"Speed","control":"choice","path":"settings.unitsSettings.speedUnits"}]}
          ]}
        ]}
        """,
    )

    @Test
    fun `a section reads as its subsections, and an empty one is dropped`() {
        val read = settingsSections(general)
        assertEquals(listOf("Application", "Units"), read.map { it.title })
        assertEquals(listOf("Sound"), read[0].blocks.map { it.title })
        assertEquals(listOf("audioMuted"), read[0].blocks[0].facts.map { it.name })
    }

    @Test
    fun `a control keeps the path the write goes to`() {
        val fact = settingsSections(general)[0].blocks[0].facts[0]
        assertEquals("settings.appSettings.audioMuted", fact.path)
        assertTrue(fact.isBool)
    }

    @Test
    fun `an inverted switch shows the opposite of the stored value`() {
        val control = JSONObject("""{"name":"apmStartMavlinkStreams","label":"Controlled by Vehicle","control":"toggle","value":true,"inverted":true}""")
        val fact = factFromControl(control)!!
        assertEquals(false, fact.boolValue)
        assertEquals(true, factFromControl(control.put("inverted", false))!!.boolValue)
    }

    @Test
    fun `the heading names the subsection, or the section when it says something the page title does not`() {
        val read = settingsSections(general)
        assertEquals("Sound", blockHeading("General", read[0], read[0].blocks[0]))
        assertEquals("Units", blockHeading("General", read[1], read[1].blocks[0]))
        assertEquals(
            "a page called Plan View does not need a heading called Plan View under it",
            "",
            blockHeading("Plan View", SettingsSectionRows("Plan View", "planViewSettings", "", emptyList()), SettingsBlock("", emptyList())),
        )
    }

    @Test
    fun `a section with nothing in it is not drawn`() {
        val view = JSONObject(
            """{"title":"X","sections":[{"title":"Gone","group":"g","subsections":[{"title":"","controls":[]}]}]}""",
        )
        assertTrue(settingsSections(view).isEmpty())
    }

    @Test
    fun `the video page carries the flag the extra-sources editor is drawn on`() {
        val view = pages("""{"title":"Video","showsVideoSources":true,"sections":[{"title":"Video","group":"videoSettings"}]}""")
        assertTrue(settingsPages(view).single().showsVideoSources)
        assertTrue(
            "a page that does not ask for the block must not get it",
            !settingsPages(pages("""{"title":"Maps","sections":[{"title":"Maps","group":"mapsSettings"}]}""")).single().showsVideoSources,
        )
    }

    @Test
    fun `a search finds a setting by what it is called and by its name`() {
        val read = settingsSections(general)
        assertEquals(
            listOf("General \u203a Application"),
            matchesIn("General", read, "mute").map { it.title },
        )
        assertEquals(
            "an operator who knows the fact's name should not have to guess the page",
            listOf("audioMuted"),
            matchesIn("General", read, "audioMuted").flatMap { s -> s.blocks.flatMap { it.facts } }.map { it.name },
        )
        assertEquals("a typed query is not case sensitive", 1, matchesIn("General", read, "MUTE").size)
    }

    @Test
    fun `an empty search matches nothing rather than everything`() {
        val read = settingsSections(general)
        assertTrue(matchesIn("General", read, "").isEmpty())
        assertTrue(matchesIn("General", read, "   ").isEmpty())
    }

    @Test
    fun `a search result keeps the path its write goes to`() {
        val hit = matchesIn("General", settingsSections(general), "mute").single()
        assertEquals("settings.appSettings.audioMuted", hit.blocks.single().facts.single().path)
        assertEquals(
            "a note about facts the head drew elsewhere makes no sense beside one search hit",
            "",
            hit.note,
        )
    }

    @Test
    fun `the unit rows are reached through their own section, never through search`() {
        assertTrue(
            "whether a single measurement may be set at all depends on the measurement system, " +
                "and that gate lives in the section this head draws itself",
            matchesIn("General", settingsSections(general), "speed").isEmpty(),
        )
    }

    @Test
    fun `the head suppresses the note only where it draws the editor itself`() {
        assertEquals(setOf(VIDEO_GROUP, FLY_VIEW_GROUP), GROUPS_WITH_A_HEAD_EDITOR)
        assertTrue(
            "the core's note sends the operator to the desktop for these two, which is wrong on a head that has an editor for them",
            UNITS_GROUP !in GROUPS_WITH_A_HEAD_EDITOR,
        )
    }

    @Test
    fun theConnectionsRowCountsConnectedLinks() {
        val view = org.json.JSONObject("""{"links":[{"connected":true},{"connected":false},{"connected":true}]}""")
        org.junit.Assert.assertEquals(2, activeLinkCount(view))
        org.junit.Assert.assertEquals("2 active", activeLinksText(2))
        org.junit.Assert.assertEquals("", activeLinksText(0))
        org.junit.Assert.assertEquals(0, activeLinkCount(null))
    }

    @Test
    fun aSearchBreadcrumbReadsInSentenceCase() {
        org.junit.Assert.assertEquals("Maps \u203a Flight map", shownBreadcrumb("Maps \u203a Flight Map"))
    }

    @Test
    fun theConnectionsPageReadsAsLinksLikePenpot() {
        org.junit.Assert.assertEquals("Links", pageTitle(CONNECTIONS_PAGE))
        org.junit.Assert.assertEquals("Fly view", pageTitle("Fly View"))
        org.junit.Assert.assertEquals("ADS-B server", pageTitle("ADSB Server"))
    }

    @Test
    fun aGlanceJoinsTheSetValuesOnly() {
        org.junit.Assert.assertEquals("Bing · Hybrid", glanceText(listOf("Bing", "Hybrid")))
        org.junit.Assert.assertEquals("", glanceText(listOf("", " ")))
        org.junit.Assert.assertEquals("Video stream disabled · Google · Satellite", glanceText(listOf("Video Stream Disabled", "Google", "Satellite")))
    }

    @Test
    fun aPageRowReadsItsCurrentStateUnderTheTitleAndFallsBackToWhatThePageHolds() {
        org.junit.Assert.assertEquals("Bing · Hybrid", pageSubtitle("Maps", "Bing · Hybrid"))
        org.junit.Assert.assertEquals(PAGE_NOTES["Maps"], pageSubtitle("Maps", ""))
    }

    @Test
    fun theConnectionsRowNamesItsLiveLinks() {
        val view = org.json.JSONObject("""{"links":[{"name":"UDP Link","connected":true,"summary":"UDP port 14550"},{"name":"Spare","connected":false,"summary":"COM3"},{"name":"TCP 127.0.0.1:5771","connected":true,"summary":""}]}""")
        org.junit.Assert.assertEquals("UDP port 14550 \u00b7 TCP 127.0.0.1:5771", activeLinksGlance(view))
        org.junit.Assert.assertEquals("", activeLinksGlance(null))
    }

    @Test
    fun `pages match on their QGC keywords and settings on theirs`() {
        val telemetry = SettingsPageEntry("MAVLink", false, false, 0, keywords = "ground, station, signing, forwarding")
        assertTrue(pageMatches(telemetry, "Signing"))
        assertTrue(pageMatches(telemetry, "mav"))
        assertTrue(!pageMatches(telemetry, "offline"))
        assertTrue(!pageMatches(telemetry, " "))
    }
}
