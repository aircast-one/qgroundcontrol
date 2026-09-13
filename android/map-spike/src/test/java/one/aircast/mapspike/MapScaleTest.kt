package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MapScaleTest {
    @Test
    fun `metres across the bar follow latitude and zoom`() {
        val equator = metresAcross(0.0, 16.0, 300.0)!!
        val south = metresAcross(-60.0, 16.0, 300.0)!!

        assertEquals("a degree of longitude is shorter away from the equator", true, south < equator)
        assertEquals(metresPerPixel(0.0, 16.0) * 300.0, equator, 1e-9)
    }

    @Test
    fun `a bar that cannot be measured is not drawn`() {
        assertNull(metresAcross(0.0, 16.0, 0.0))
        assertNull(metresAcross(Double.NaN, 16.0, 200.0))
    }

    @Test
    fun `the bar takes the core's text and its share of the width`() {
        val bar = mapScaleBar(JSONObject("""{"available":true,"text":"100 m","fraction":0.5}"""), 300.0)!!

        assertEquals("100 m", bar.text)
        assertEquals(150.0, bar.pixels, 1e-9)
    }

    @Test
    fun `imperial text is passed through, because the core chose the units`() {
        val bar = mapScaleBar(JSONObject("""{"available":true,"text":"500 ft","fraction":0.4}"""), 200.0)!!

        assertEquals("500 ft", bar.text)
    }

    @Test
    fun `a scale the core could not work out draws nothing`() {
        assertNull(mapScaleBar(JSONObject("""{"available":false}"""), 300.0))
        assertNull(mapScaleBar(JSONObject("""{"available":true,"text":"","fraction":0.5}"""), 300.0))
        assertNull(mapScaleBar(JSONObject("""{"available":true,"text":"100 m"}"""), 300.0))
        assertNull(mapScaleBar(null, 300.0))
    }
}
