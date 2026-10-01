package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class AppFontScaleTest {
    @Test
    fun `font size scales from the platform default and unset keeps it`() {
        assertEquals(1f, appFontScale(14), 0.001f)
        assertEquals(1.5f, appFontScale(21), 0.001f)
        assertEquals(1f, appFontScale(0), 0.001f)
        assertEquals(1f, appFontScale(60), 0.001f)
        assertEquals(1f, appFontScale(null), 0.001f)
    }
}
