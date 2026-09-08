package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MapScaleTest {
    @Test
    fun `ground per pixel halves with every zoom level`() {
        val near = metresPerPixel(0.0, 10.0)
        val closer = metresPerPixel(0.0, 11.0)

        assertEquals(near / 2, closer, 1e-9)
    }

    @Test
    fun `a pixel covers less ground away from the equator`() {
        assertTrue(metresPerPixel(60.0, 12.0) < metresPerPixel(0.0, 12.0))
        assertEquals(metresPerPixel(0.0, 12.0) / 2, metresPerPixel(60.0, 12.0), 1e-6)
    }

    @Test
    fun `distances round down to something worth reading`() {
        assertEquals(500.0, niceDistance(900.0), 1e-9)
        assertEquals(200.0, niceDistance(300.0), 1e-9)
        assertEquals(100.0, niceDistance(150.0), 1e-9)
        assertEquals(1000.0, niceDistance(1800.0), 1e-9)
    }

    @Test
    fun `the bar never claims more ground than it spans`() {
        val scale = mapScale(-35.36, 16.0, 300.0)!!

        assertTrue(scale.pixels <= 300.0)
        assertEquals(scale.metres / metresPerPixel(-35.36, 16.0), scale.pixels, 1e-6)
    }

    private fun labelForExactly(metres: Double): String {
        val perPixel = metresPerPixel(0.0, 14.0)
        return mapScale(0.0, 14.0, metres / perPixel * (1 + 1e-9))!!.label
    }

    @Test
    fun `labels switch to kilometres and keep whole numbers whole`() {
        assertEquals("500 m", labelForExactly(500.0))
        assertEquals("2 km", labelForExactly(2000.0))
        assertEquals("100 m", labelForExactly(100.0))
    }

    @Test
    fun `a bar under a metre says so instead of reading zero`() {
        assertEquals("0.5 m", labelForExactly(0.5))
        assertEquals("0.2 m", labelForExactly(0.2))
    }

    @Test
    fun `the closest the map goes still labels itself`() {
        val closest = mapScale(-35.36, 25.5, 385.0)!!

        assertTrue(closest.metres > 0)
        assertEquals("0.5 m", closest.label)
    }

    @Test
    fun `an unusable camera yields no bar rather than a wrong one`() {
        assertNull(mapScale(0.0, 16.0, 0.0))
        assertNull(mapScale(Double.NaN, 16.0, 200.0))
    }
}
