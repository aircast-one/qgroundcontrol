package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RcToParamTest {
    @Test
    fun `every field must be a number before the mapping is sent`() {
        assertEquals(RcToParam(1.0, 6.5, 2, 0.0, 12.0), rcToParam("1.0", "6.5", 2, "0", "12"))
        assertNull(rcToParam("1.0", "", 0, "0", "12"))
        assertNull(rcToParam("x", "6.5", 0, "0", "12"))
    }
}
