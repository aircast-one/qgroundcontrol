package one.aircast.android.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ResetAllSettingsTest {
    @Test
    fun `a pending reset reads from the setting value`() {
        assertTrue(resetPending(true))
        assertFalse(resetPending(false))
        assertFalse(resetPending(null))
    }
}
