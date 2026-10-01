package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SettingsGroupsTest {
    private fun page(title: String) = SettingsPageEntry(title, showsLinks = false, showsVideoSources = false, sectionCount = 0)

    @Test
    fun pagesGroupInDesignOrderKeepingTheirOrderWithinAGroup() {
        val grouped = groupedPages(listOf(page("General"), page("Fly View"), page("Video"), page("Connections"), page("Something new")))
        assertEquals(
            listOf(
                SettingsGroup.Connection to listOf("Video", "Connections"),
                SettingsGroup.Flying to listOf("Fly View"),
                SettingsGroup.App to listOf("General"),
                SettingsGroup.More to listOf("Something new"),
            ),
            grouped.map { (group, pages) -> group to pages.map { it.title } },
        )
    }
}
