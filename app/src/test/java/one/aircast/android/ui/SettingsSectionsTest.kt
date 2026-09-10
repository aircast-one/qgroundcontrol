package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SettingsSectionsTest {
    private fun fact(name: String) = Fact(
        path = "p.$name",
        name = name,
        description = name,
        units = "",
        valueString = "",
        value = null,
        enumStrings = emptyList(),
        enumIndex = 0,
        isBool = false,
        isString = true,
        readOnly = false,
    )

    private val appFacts = listOf(
        "offlineEditingCruiseSpeed", "audioMuted", "indoorPalette", "mapboxToken",
        "passAirLink", "savePath", "firstRunPromptIdsShown", "enableMultiVehiclePanel",
    ).map(::fact)

    @Test
    fun `an unmapped page keeps its flat list`() {
        val facts = listOf(fact("a"), fact("b"))
        assertEquals(listOf("" to facts), sectionedFacts("settings.mapsSettings", facts))
    }

    @Test
    fun `internal bookkeeping is never shown as a setting`() {
        val all = sectionedFacts("settings.appSettings", appFacts).flatMap { it.second }
        assertTrue(all.none { it.name == "firstRunPromptIdsShown" })
    }

    @Test
    fun `internal facts are hidden on unmapped pages too`() {
        val facts = listOf(fact("instrumentQmlFile2"), fact("keepMapCenteredOnVehicle"))
        val shown = sectionedFacts("settings.unmapped", facts).flatMap { it.second }
        assertEquals(listOf("keepMapCenteredOnVehicle"), shown.map { it.name })
    }

    @Test
    fun `a fact with its own editor is hidden from the fact list`() {
        val facts = listOf(fact("rcControls"), fact("extraVideoSources"), fact("audioMuted"))
        val shown = sectionedFacts("settings.unmapped", facts).flatMap { it.second }

        assertEquals(listOf("audioMuted"), shown.map { it.name })
    }

    @Test
    fun `facts land under the heading that owns them`() {
        val sections = sectionedFacts("settings.appSettings", appFacts).toMap()
        assertEquals(listOf("indoorPalette"), sections["Appearance"]?.map { it.name })
        assertEquals(listOf("mapboxToken"), sections["Map providers"]?.map { it.name })
        assertEquals(listOf("passAirLink"), sections["AirLink"]?.map { it.name })
    }

    @Test
    fun `a section with nothing in it is not rendered`() {
        val sections = sectionedFacts("settings.appSettings", listOf(fact("audioMuted"))).toMap()
        assertEquals(setOf("Sound"), sections.keys)
    }

    @Test
    fun `nothing is silently dropped - unclaimed facts fall to Other`() {
        val sections = sectionedFacts("settings.appSettings", appFacts).toMap()
        assertEquals(listOf("enableMultiVehiclePanel"), sections[OTHER_SECTION]?.map { it.name })
    }

    @Test
    fun `every fact in equals every fact out minus the internal ones`() {
        val out = sectionedFacts("settings.appSettings", appFacts).flatMap { it.second }.map { it.name }
        val expected = appFacts.map { it.name }.filterNot { it in hiddenFacts() }
        assertEquals(expected.toSet(), out.toSet())
        assertEquals(expected.size, out.size)
    }

    @Test
    fun `sections keep the order they are declared in`() {
        val titles = sectionedFacts("settings.appSettings", appFacts).map { it.first }
        assertEquals(
            listOf("Appearance", "Sound", "Planning defaults", "Map providers", "AirLink", "Files", OTHER_SECTION),
            titles,
        )
    }

    @Test
    fun `no fact is claimed by two sections on any page`() {
        SETTINGS_SECTIONS.forEach { (page, sections) ->
            val all = sections.flatMap { it.factNames }
            assertEquals("$page has a duplicate", all.size, all.toSet().size)
        }
    }
}
