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
}
