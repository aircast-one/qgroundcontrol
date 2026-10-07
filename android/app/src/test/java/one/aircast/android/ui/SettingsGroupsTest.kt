package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SettingsGroupsTest {
    private fun page(title: String) = SettingsPageEntry(title, showsLinks = false, showsVideoSources = false, sectionCount = 0)

    @Test
    fun pagesGroupAndOrderInDesignOrder() {
        val grouped = groupedPages(listOf(page("General"), page("Fly View"), page("Video"), page("Connections"), page("Something new"), page("Also new")))
        assertEquals(
            listOf(
                SettingsGroup.Control to listOf("Fly View"),
                SettingsGroup.Camera to listOf("Video"),
                SettingsGroup.Transmission to listOf("Connections"),
                SettingsGroup.General to listOf("General", "Something new", "Also new"),
            ),
            grouped.map { (group, pages) -> group to pages.map { it.title } },
        )
    }
}

class SettingsSheetTest {
    @org.junit.Test
    fun `the sheet opens on the tab that holds the asked-for page, or Safety`() {
        org.junit.Assert.assertEquals(SettingsGroup.Transmission, openingGroup("Connections"))
        org.junit.Assert.assertEquals(SettingsGroup.Camera, openingGroup("Video"))
        org.junit.Assert.assertEquals(SettingsGroup.Safety, openingGroup(null))
        org.junit.Assert.assertEquals(null, one.aircast.android.Tab.from("settings"))
    }
}
