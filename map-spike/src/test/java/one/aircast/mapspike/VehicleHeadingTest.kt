package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VehicleHeadingTest {
    @Test
    fun `a known heading rides along on the feature`() {
        val feature = vehicleFeature(41.0, 44.0, 90.0)

        assertTrue(feature.hasProperty(HEADING_PROPERTY))
        assertEquals(90.0, feature.getNumberProperty(HEADING_PROPERTY).toDouble(), 1e-9)
    }

    @Test
    fun `an unknown heading leaves the property off so the arrow is filtered out`() {
        assertFalse(vehicleFeature(41.0, 44.0, Double.NaN).hasProperty(HEADING_PROPERTY))
    }

    @Test
    fun `headings are normalised into a single turn`() {
        val wrapped = vehicleFeature(41.0, 44.0, 450.0)
        val negative = vehicleFeature(41.0, 44.0, -90.0)

        assertEquals(90.0, wrapped.getNumberProperty(HEADING_PROPERTY).toDouble(), 1e-9)
        assertEquals(270.0, negative.getNumberProperty(HEADING_PROPERTY).toDouble(), 1e-9)
    }

    @Test
    fun `the position still rides along without a heading`() {
        val point = vehicleFeature(41.5, 44.5, Double.NaN).geometry() as org.maplibre.geojson.Point

        assertEquals(41.5, point.latitude(), 1e-9)
        assertEquals(44.5, point.longitude(), 1e-9)
    }
}
