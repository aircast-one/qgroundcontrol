package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class VehicleRowCompassTest {
    @Test
    fun `a disarmed vehicle's compass is dimmed like MultiVehicleList's`() {
        assertEquals(1f, rowCompassAlpha(armed = true))
        assertEquals(0.5f, rowCompassAlpha(armed = false))
    }
}
