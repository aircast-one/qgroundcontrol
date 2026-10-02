package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class NavigationBlockTest {
    @Test
    fun `a set reason refuses leaving with QGC's words, like MainWindow allowViewSwitch`() {
        assertEquals(LOG_DOWNLOAD_BLOCK, navigationRefusal(LOG_DOWNLOAD_BLOCK, leaving = true))
        assertNull(navigationRefusal(null, leaving = true))
        assertNull(navigationRefusal(CALIBRATION_BLOCK, leaving = false))
        assertEquals("Complete or cancel the current calibration first", CALIBRATION_BLOCK)
    }
}
