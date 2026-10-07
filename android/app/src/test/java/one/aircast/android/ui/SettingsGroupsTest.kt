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

class TabSetupPagesTest {
    @org.junit.Test
    fun `flight-relevant setup pages surface in the tab a pilot looks in`() {
        org.junit.Assert.assertEquals(listOf("Safety", "Sensors"), tabSetupPages(SettingsGroup.Safety))
        org.junit.Assert.assertEquals(listOf("Flight Modes"), tabSetupPages(SettingsGroup.Control))
        org.junit.Assert.assertEquals(emptyList<String>(), tabSetupPages(SettingsGroup.Camera))
        org.junit.Assert.assertEquals(5, SettingsGroup.entries.size)
    }
}
