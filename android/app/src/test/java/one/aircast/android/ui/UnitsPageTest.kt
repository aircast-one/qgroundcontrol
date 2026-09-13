package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class UnitsPageTest {
    private fun fact(name: String) = Fact(
        path = "settings.unitsSettings.$name",
        name = name,
        description = name,
        units = "",
        valueString = "0",
        value = 0,
        enumStrings = listOf("A", "B"),
        enumIndex = 0,
        isBool = false,
        isString = false,
        readOnly = false,
    )

    private val all = (PRESET_UNIT_FACTS + listOf("weightUnits", "customUnits")).map(::fact)

    @Test
    fun `a preset hides the measurements it already decides`() {
        val rows = unitRowsFor(0, all).map { it.name }
        assertEquals(listOf("weightUnits"), rows)
    }

    @Test
    fun `custom shows every measurement`() {
        val rows = unitRowsFor(UNIT_SYSTEM_CUSTOM, all).map { it.name }
        assertEquals(PRESET_UNIT_FACTS + listOf("weightUnits"), rows)
    }

    @Test
    fun `the custom-units flag is never a row of its own`() {
        listOf(0, 1, UNIT_SYSTEM_CUSTOM).forEach { system ->
            assertTrue(unitRowsFor(system, all).none { it.name == "customUnits" })
        }
    }

    @Test
    fun `weight survives a preset because no preset sets it`() {
        assertTrue(unitRowsFor(1, all).any { it.name == "weightUnits" })
    }

    @Test
    fun `the note names the system in force`() {
        assertTrue(unitSystemNote(0).contains("Metric"))
        assertTrue(unitSystemNote(1).contains("Imperial"))
        assertEquals("Each measurement is set on its own below.", unitSystemNote(UNIT_SYSTEM_CUSTOM))
    }

    @Test
    fun `a system index the bridge should never send is treated as custom`() {
        listOf(-1, 3, 99).forEach { rogue ->
            assertEquals("Custom", unitSystemLabel(rogue))
            assertEquals("Each measurement is set on its own below.", unitSystemNote(rogue))
            assertEquals(
                PRESET_UNIT_FACTS + listOf("weightUnits"),
                unitRowsFor(rogue, all).map { it.name },
            )
        }
    }
}
