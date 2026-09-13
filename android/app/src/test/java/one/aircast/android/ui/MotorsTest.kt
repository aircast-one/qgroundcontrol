package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.assertNull
import org.junit.Test

class MotorsTest {
    @Test
    fun `a vehicle that names its motor count is believed`() {
        assertEquals(6, motorCount(6.0))
        assertNull(motorCountNotice(6.0))
    }

    @Test
    fun `an airframe with no published layout gets eight buttons rather than none`() {
        assertEquals(8, motorCount(Double.NaN))
        assertEquals(8, motorCount(-1.0))
    }

    @Test
    fun `the notice does not blame the vehicle for a gap in the layout table`() {
        val notice = motorCountNotice(-1.0)!!
        assertFalse(notice.contains("did not say"))
        assertTrue(notice.contains("airframe"))
    }
}
