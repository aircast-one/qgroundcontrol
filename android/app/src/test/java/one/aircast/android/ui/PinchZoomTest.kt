package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class PinchZoomTest {
    @Test
    fun `a pinch maps to FlightDisplayViewVideo's step values`() {
        assertEquals(2, pinchStep(2.2f))
        assertEquals(1, pinchStep(1.1f))
        assertEquals(-5, pinchStep(0.5f))
    }
}
