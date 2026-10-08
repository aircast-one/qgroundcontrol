package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LoadProgressTest {
    @Test
    fun `the chip shows load progress until the initial connect completes`() {
        assertEquals(0.4f, loadingProgress(false, 0.4))
        assertEquals(0f, loadingProgress(false, null))
        assertEquals(1f, loadingProgress(false, 3))
        assertNull(loadingProgress(true, 1.0))
        assertNull(loadingProgress(null, null))
    }
}
