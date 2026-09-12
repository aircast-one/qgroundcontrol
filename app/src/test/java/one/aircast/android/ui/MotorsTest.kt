package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MotorsTest {
    @Test
    fun `a vehicle that names its motor count is believed`() {
        assertEquals(6, motorCount(6.0))
        assertNull(motorCountNotice(6.0))
    }

    @Test
    fun `a vehicle that cannot say gets eight buttons and says so, rather than none`() {
        assertEquals(8, motorCount(Double.NaN))
        assertEquals(8, motorCount(-1.0))
        assertEquals(
            "The vehicle did not say how many motors it has, so eight are offered.",
            motorCountNotice(-1.0),
        )
    }
}
