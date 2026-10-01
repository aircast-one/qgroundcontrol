package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class PhotoCountTest {
    @Test
    fun `the photo count is five digits like the QGC counter`() {
        assertEquals("00007", photoCountText(7))
        assertEquals("12345", photoCountText(112345))
    }
}
