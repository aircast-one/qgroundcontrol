package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class PaletteTest {
    @Test
    fun indoorIsDarkOutdoorIsLightSystemFollowsTheDevice() {
        assertEquals(true, paletteIsDark(1, systemDark = false))
        assertEquals(false, paletteIsDark(0, systemDark = true))
        assertEquals(true, paletteIsDark(2, systemDark = true))
        assertEquals(false, paletteIsDark(2, systemDark = false))
    }

    @Test
    fun darkWhileTheSettingIsUnread() = assertEquals(true, paletteIsDark(null, systemDark = false))

    @Test
    fun defaultsToDarkOnceAndOnlyWhenUntouched() {
        assertEquals(true, shouldDefaultToDark(changedFromDefault = false, alreadyDefaulted = false))
        assertEquals(false, shouldDefaultToDark(changedFromDefault = true, alreadyDefaulted = false))
        assertEquals(false, shouldDefaultToDark(changedFromDefault = false, alreadyDefaulted = true))
    }

    @Test
    fun `the palette choice is named dark and light by its stored value, as Penpot names it`() {
        val fact = one.aircast.android.bridge.Fact(
            path = PALETTE_SETTING, name = "indoorPalette", description = "Color Scheme", units = "", valueString = "1", value = 1,
            enumStrings = listOf("Innen", "Außen", "System"), enumValues = listOf("1", "0", "2"), enumIndex = 0, isBool = false, isString = false, readOnly = false,
        )
        org.junit.Assert.assertEquals(listOf("Dark", "Light", "System"), paletteNamed(fact).enumStrings)
        org.junit.Assert.assertEquals(listOf("A", "B"), paletteNamed(fact.copy(path = "settings.x", enumStrings = listOf("A", "B"), enumValues = listOf("1", "0"))).enumStrings)
    }
}
