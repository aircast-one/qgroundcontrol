package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SettingsGroupsTest {
    private fun page(title: String) = SettingsPageEntry(title, showsLinks = false, showsVideoSources = false, sectionCount = 0)

    private val every = listOf(page("General"), page("Remote ID"), page("Fly View"), page("ADSB Server"), page("Video"), page("Connections"), page("Something new"))

    @Test
    fun eachTabListsItsPagesInDesignOrderAndUnknownPagesLandInGeneral() {
        assertEquals(listOf("Fly View"), tabPages(SettingsGroup.Control, every).map { it.title })
        assertEquals(listOf("Connections"), tabPages(SettingsGroup.Transmission, every).map { it.title })
        assertEquals(listOf("General", "Something new"), tabPages(SettingsGroup.General, every).map { it.title })
    }

    @Test
    fun safetyOpensWithTheGuidedLimitsItBorrowsFromFlyView() =
        assertEquals(listOf("Fly View", "ADSB Server", "Remote ID"), tabPages(SettingsGroup.Safety, every).map { it.title })

    private fun block(title: String) = SettingsBlock(title, listOf(one.aircast.android.bridge.Fact(
        path = "settings.flyViewSettings.$title", name = title, description = "", units = "", valueString = "", value = 0,
        enumStrings = emptyList(), enumIndex = -1, isBool = true, isString = false, readOnly = false,
    )))

    private val flyView = listOf(SettingsSectionRows("Fly View", "flyViewSettings", "", listOf(block("Guided Commands"), block("Map and compass"))))

    @Test
    fun guidedCommandsMoveToSafetyAndLeaveTheRestOfFlyViewInControl() {
        assertEquals(listOf("Guided Commands"), sectionsIn(SettingsGroup.Safety, "Fly View", flyView).flatMap { it.blocks }.map { it.title })
        assertEquals(listOf("Map and compass"), sectionsIn(SettingsGroup.Control, "Fly View", flyView).flatMap { it.blocks }.map { it.title })
        assertEquals(emptyList<SettingsSectionRows>(), sectionsIn(SettingsGroup.Camera, "Fly View", flyView))
    }

    @Test
    fun borrowedBlocksAreHeadedByTheirOwnNames() =
        assertEquals("Guided commands", borrowedTitle(sectionsIn(SettingsGroup.Safety, "Fly View", flyView)))

    @Test
    fun everyKnownPageCarriesItsOwnIconAndUnknownPagesFallBackToSettings() {
        assertEquals(PAGE_LOOKS.size, PAGE_LOOKS.values.count { it.icon != 0 })
        assertEquals(one.aircast.android.R.drawable.ic_settings, pageLook("Something new").icon)
    }
}

class SettingsSheetTest {
    @org.junit.Test
    fun `the sheet opens on the tab that holds the asked-for page, or Safety`() {
        org.junit.Assert.assertEquals(SettingsGroup.Transmission, openingGroup("Connections"))
        org.junit.Assert.assertEquals(SettingsGroup.Camera, openingGroup("Video"))
        org.junit.Assert.assertEquals(SettingsGroup.Safety, openingGroup(null))
        org.junit.Assert.assertEquals(null, one.aircast.android.Tab.from("settings"))
        org.junit.Assert.assertEquals(null, one.aircast.android.Tab.from("setup"))
    }
}

class LookingForAircraftTest {
    @org.junit.Test
    fun `the empty state says what to do, not what is missing`() {
        org.junit.Assert.assertEquals("Looking for your aircraft", LOOKING_TITLE)
        org.junit.Assert.assertTrue(LOOKING_HINT.startsWith("Turn on the aircraft"))
    }
}

class TabSetupRowsTest {
    private fun component(name: String) = SetupComponent(index = 0, name = name, needsAttention = false)

    @org.junit.Test
    fun `a tab lists only the setup pages the vehicle reports, in tab order`() {
        val reported = listOf(component("Flight Modes"), component(SENSORS), component("Power"))
        org.junit.Assert.assertEquals(listOf(SENSORS), tabSetupComponents(SettingsGroup.Safety, reported).map { it.name })
        org.junit.Assert.assertEquals(listOf(FLIGHT_MODES_PAGE), tabSetupComponents(SettingsGroup.Control, reported).map { it.name })
        org.junit.Assert.assertEquals(emptyList<SetupComponent>(), tabSetupComponents(SettingsGroup.Camera, reported))
        org.junit.Assert.assertEquals(emptyList<SetupComponent>(), tabSetupComponents(SettingsGroup.Safety, emptyList()))
    }

    @org.junit.Test
    fun `search finds setup pages by name and nothing for a blank query`() {
        val reported = listOf(component("Flight Modes"), component("Flight Behavior"), component(SENSORS))
        org.junit.Assert.assertEquals(listOf("Flight Modes", "Flight Behavior"), setupSearchHits(reported, "flight").map { it.name })
        org.junit.Assert.assertEquals(emptyList<SetupComponent>(), setupSearchHits(reported, "  "))
    }
}

class ShownSubtitleTest {
    @org.junit.Test
    fun `the description hides behind help until opened and units stay`() {
        org.junit.Assert.assertEquals(ShownSubtitle("m", true), shownSubtitle("Loiter radius · m", "Loiter radius", helpBehind = true, helpOpen = false))
        org.junit.Assert.assertEquals(ShownSubtitle("Loiter radius · m", true), shownSubtitle("Loiter radius · m", "Loiter radius", helpBehind = true, helpOpen = true))
        org.junit.Assert.assertEquals(ShownSubtitle("", true), shownSubtitle("Loiter radius", "Loiter radius", helpBehind = true, helpOpen = false))
    }

    @org.junit.Test
    fun `a description containing the separator still hides whole`() {
        org.junit.Assert.assertEquals(ShownSubtitle("m", true), shownSubtitle("Speed · climb · m", "Speed · climb", helpBehind = true, helpOpen = false))
    }

    @org.junit.Test
    fun `no help outside settings, without a description, or for a custom subtitle`() {
        org.junit.Assert.assertEquals(ShownSubtitle("Loiter radius · m", false), shownSubtitle("Loiter radius · m", "Loiter radius", helpBehind = false, helpOpen = false))
        org.junit.Assert.assertEquals(ShownSubtitle("m", false), shownSubtitle("m", "", helpBehind = true, helpOpen = false))
        org.junit.Assert.assertEquals(ShownSubtitle("Custom", false), shownSubtitle("Custom", "Loiter radius", helpBehind = true, helpOpen = false))
    }
}
