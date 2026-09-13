package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class VehicleMarkerTest {
    @Test
    fun `a vehicle with a position is drawn`() {
        val features = vehicleFeatures(-35.36, 149.16, 90.0).features()!!

        assertEquals(1, features.size)
        assertEquals(90.0, features.single().getNumberProperty(HEADING_PROPERTY).toDouble(), 1e-9)
    }

    @Test
    fun `a vehicle that has gone away is not left on the map`() {
        assertTrue(vehicleFeatures(Double.NaN, Double.NaN, 90.0).features()!!.isEmpty())
        assertTrue(vehicleFeatures(0.0, 0.0, 90.0).features()!!.isEmpty())
    }

    @Test
    fun `a position without heading still draws the dot`() {
        val features = vehicleFeatures(-35.36, 149.16, Double.NaN).features()!!

        assertEquals(1, features.size)
        assertTrue(!features.single().hasProperty(HEADING_PROPERTY))
    }
}
